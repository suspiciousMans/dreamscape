use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::TcpStream;

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Hard ceiling on a single inbound frame's advertised length. Comfortably
/// larger than the biggest real message (a `Welcome` carrying a whole
/// `Level`), but bounded so a peer that sends a bogus/huge `u32` length
/// prefix can't make us buffer toward ~4 GiB waiting for a "frame" that
/// never completes — see `drain_frames`.
const MAX_FRAME_LEN: usize = 64 * 1024 * 1024;

/// Ceiling on per-connection queued *outbound* bytes — see `send`.
const MAX_WRITE_QUEUE_BYTES: usize = 16 * 1024 * 1024;

/// Signals that a peer advertised a frame longer than `MAX_FRAME_LEN`; the
/// connection is torn down rather than trusted to eventually deliver it.
#[derive(Debug)]
struct OversizeFrame;

/// Prefixes `payload` with its little-endian `u32` length — the wire shape
/// every frame uses, in both directions.
fn encode_frame(payload: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(4 + payload.len());
    framed.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    framed.extend_from_slice(payload);
    framed
}

/// Drains every complete `[len][payload]` frame currently sitting in
/// `buf`, leaving a trailing partial frame (if any) in place for a future
/// call to complete — pure buffer logic, no I/O, so it's unit-testable
/// without a real socket regardless of how ragged the underlying reads are.
/// Returns `Err(OversizeFrame)` if a length prefix exceeds `MAX_FRAME_LEN`,
/// so the caller can drop the connection instead of accumulating forever.
fn drain_frames(buf: &mut Vec<u8>) -> Result<Vec<Vec<u8>>, OversizeFrame> {
    let mut frames = Vec::new();
    let mut consumed = 0;
    loop {
        let remaining = &buf[consumed..];
        if remaining.len() < 4 {
            break;
        }
        let len = u32::from_le_bytes(remaining[0..4].try_into().unwrap()) as usize;
        if len > MAX_FRAME_LEN {
            return Err(OversizeFrame);
        }
        if remaining.len() < 4 + len {
            break;
        }
        frames.push(remaining[4..4 + len].to_vec());
        consumed += 4 + len;
    }
    if consumed > 0 {
        buf.drain(0..consumed);
    }
    Ok(frames)
}

/// Wraps one non-blocking `TcpStream` with length-prefixed bincode framing
/// in both directions. Every `pump()` call is guaranteed to never block —
/// `WouldBlock` on a partial read/write just means "nothing more this
/// frame," not an error — matching the `HotReloadWatcher::poll_events`
/// shape: called once per `Game::update`, drains what's ready, and leaves
/// the rest queued for next frame.
pub struct NetConnection {
    stream: TcpStream,
    read_buf: Vec<u8>,
    write_queue: VecDeque<Vec<u8>>,
    write_pos: usize,
    /// Total bytes currently sitting in `write_queue`, tracked so `send`
    /// can enforce `MAX_WRITE_QUEUE_BYTES` without re-summing the queue.
    queued_bytes: usize,
    disconnected: bool,
}

impl NetConnection {
    /// Takes ownership of an already-connected `TcpStream` (from
    /// `TcpListener::accept` or `TcpStream::connect`) and puts it in
    /// non-blocking mode. `TCP_NODELAY` is set so small, frequent
    /// messages (an `Input` every frame) aren't held up by Nagle's
    /// algorithm waiting to coalesce with more data.
    pub fn wrap(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            read_buf: Vec::new(),
            write_queue: VecDeque::new(),
            write_pos: 0,
            queued_bytes: 0,
            disconnected: false,
        })
    }

    pub fn is_disconnected(&self) -> bool {
        self.disconnected
    }

    /// Serializes and queues `msg` for sending — the actual bytes go out
    /// over however many future `pump()` calls it takes; this call itself
    /// never touches the socket and never blocks.
    ///
    /// If the peer stalls (its TCP window fills, so `pump_writes` keeps
    /// hitting `WouldBlock`) while we keep queuing snapshots ~20x/sec, the
    /// queue would otherwise grow without bound and exhaust host memory. A
    /// peer that far behind is effectively gone, so once the queue would
    /// exceed `MAX_WRITE_QUEUE_BYTES` the connection is marked disconnected
    /// and the message dropped, rather than buffered forever.
    pub fn send<T: Serialize>(&mut self, msg: &T) {
        if self.disconnected {
            return;
        }
        match bincode::serialize(msg) {
            Ok(payload) => {
                let framed = encode_frame(&payload);
                if self.queued_bytes.saturating_add(framed.len()) > MAX_WRITE_QUEUE_BYTES {
                    log::warn!("peer too far behind (outbound queue over {MAX_WRITE_QUEUE_BYTES} bytes); disconnecting");
                    self.disconnected = true;
                    return;
                }
                self.queued_bytes += framed.len();
                self.write_queue.push_back(framed);
            }
            Err(err) => log::error!("failed to encode outgoing network message: {err}"),
        }
    }

    /// Pumps both directions for this frame — never blocks. Returns every
    /// message that completed a frame this call, decoded as `T` (messages
    /// that fail to decode are logged and dropped rather than tearing down
    /// the connection, since a lone corrupt frame shouldn't be fatal).
    /// Check `is_disconnected()` afterward to see whether the peer is gone.
    pub fn pump<T: DeserializeOwned>(&mut self) -> Vec<T> {
        self.pump_writes();
        self.pump_reads()
            .into_iter()
            .filter_map(|bytes| match bincode::deserialize(&bytes) {
                Ok(msg) => Some(msg),
                Err(err) => {
                    log::error!("failed to decode incoming network message: {err}");
                    None
                }
            })
            .collect()
    }

    fn pump_writes(&mut self) {
        if self.disconnected {
            return;
        }
        while let Some(front) = self.write_queue.front() {
            match self.stream.write(&front[self.write_pos..]) {
                Ok(0) => {
                    self.disconnected = true;
                    break;
                }
                Ok(n) => {
                    self.write_pos += n;
                    let front_len = front.len();
                    if self.write_pos >= front_len {
                        self.write_queue.pop_front();
                        self.queued_bytes = self.queued_bytes.saturating_sub(front_len);
                        self.write_pos = 0;
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    self.disconnected = true;
                    break;
                }
            }
        }
    }

    fn pump_reads(&mut self) -> Vec<Vec<u8>> {
        if self.disconnected {
            return Vec::new();
        }
        let mut tmp = [0u8; 4096];
        loop {
            match self.stream.read(&mut tmp) {
                Ok(0) => {
                    self.disconnected = true;
                    break;
                }
                Ok(n) => self.read_buf.extend_from_slice(&tmp[..n]),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    self.disconnected = true;
                    break;
                }
            }
        }
        match drain_frames(&mut self.read_buf) {
            Ok(frames) => frames,
            Err(OversizeFrame) => {
                log::error!("peer advertised a frame over {MAX_FRAME_LEN} bytes; disconnecting");
                self.disconnected = true;
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_frames_waits_for_a_complete_frame() {
        let payload = b"hi".to_vec();
        let framed = encode_frame(&payload);

        // Feed it one byte at a time — nothing should emerge until the
        // very last byte completes the frame.
        let mut buf = Vec::new();
        let mut emitted = Vec::new();
        for &byte in &framed {
            buf.push(byte);
            emitted.extend(drain_frames(&mut buf).unwrap());
        }

        assert_eq!(emitted, vec![payload]);
        assert!(buf.is_empty());
    }

    #[test]
    fn drain_frames_handles_multiple_frames_in_one_chunk() {
        let mut buf = Vec::new();
        buf.extend(encode_frame(b"first"));
        buf.extend(encode_frame(b"second"));
        // Trailing partial frame that shouldn't be emitted yet.
        buf.extend(&encode_frame(b"third")[..3]);

        let frames = drain_frames(&mut buf).unwrap();

        assert_eq!(frames, vec![b"first".to_vec(), b"second".to_vec()]);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn drain_frames_rejects_an_oversize_length_prefix() {
        // A bogus/huge length prefix must be reported (so the connection
        // gets torn down) rather than accumulated toward ~4 GiB.
        let mut buf = Vec::new();
        buf.extend_from_slice(&(u32::MAX).to_le_bytes());
        buf.extend_from_slice(b"a few trailing bytes");

        assert!(drain_frames(&mut buf).is_err());
    }

    #[test]
    fn encode_decode_round_trip_over_a_real_loopback_socket() {
        use std::net::{TcpListener, TcpStream};

        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Msg {
            n: u32,
            text: String,
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client_stream = TcpStream::connect(addr).unwrap();
        let (server_stream, _) = listener.accept().unwrap();

        let mut server = NetConnection::wrap(server_stream).unwrap();
        let mut client = NetConnection::wrap(client_stream).unwrap();

        server.send(&Msg { n: 7, text: "hello".to_string() });

        // The message may take a couple of pump() calls to actually reach
        // the client's socket buffer on some platforms — poll a bounded
        // number of times rather than assuming one call suffices. Queued
        // bytes only actually hit the socket inside pump()'s write half,
        // so the server side needs polling too, not just the client.
        let mut received: Vec<Msg> = Vec::new();
        for _ in 0..50 {
            let _: Vec<Msg> = server.pump();
            received.extend(client.pump::<Msg>());
            if !received.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert_eq!(received, vec![Msg { n: 7, text: "hello".to_string() }]);
        assert!(!client.is_disconnected());
    }
}
