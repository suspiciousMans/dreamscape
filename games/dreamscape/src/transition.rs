//! Dream-to-dream melt. The current dream slumps and dissolves into fog, the
//! next dream is swapped in at the bottom of the melt (the screen is pure fog
//! by then), then it rises and reforms. This file is pure timing/state: the
//! look lives in mesh.vert / mesh.frag (driven by `melt()`), the swap itself
//! lives in main.rs.

/// Seconds to melt the current dream down.
pub const MELT_OUT: f32 = 1.1;
/// Seconds for the next dream to rise back up.
pub const REFORM: f32 = 1.3;

/// What happens at the bottom of the melt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    /// Cyan portal: one dream deeper.
    Descend,
    /// White wake door: into the Awakening dream.
    Wake,
    /// Portal out of the Awakening: the run ends and the pack reveal opens.
    Finish,
}

impl Pending {
    /// Finishing melts into the pack reveal: there is no next dream to reform.
    fn reforms(self) -> bool {
        self != Pending::Finish
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Do the actual dream change now.
    Swap(Pending),
    /// The new dream is solid again; gameplay resumes.
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Idle,
    MeltOut,
    ReformIn,
}

#[derive(Clone, Copy, Debug)]
pub struct Transition {
    phase: Phase,
    t: f32,
    pending: Pending,
    target_fog: [f32; 3],
    /// Multiplies both durations (DREAMSCAPE_MELT_SCALE, for screenshots).
    scale: f32,
}

impl Transition {
    pub fn new(scale: f32) -> Self {
        Self {
            phase: Phase::Idle,
            t: 0.0,
            pending: Pending::Descend,
            target_fog: [0.0; 3],
            scale: scale.max(0.01),
        }
    }

    pub fn active(&self) -> bool {
        self.phase != Phase::Idle
    }

    /// Starts melting. Returns false (and changes nothing) if a transition is
    /// already running — the first request wins.
    pub fn start(&mut self, pending: Pending, target_fog: [f32; 3]) -> bool {
        if self.active() {
            return false;
        }
        *self = Self {
            phase: Phase::MeltOut,
            t: 0.0,
            pending,
            target_fog,
            scale: self.scale,
        };
        true
    }

    pub fn tick(&mut self, dt: f32) -> Option<Event> {
        match self.phase {
            Phase::Idle => None,
            Phase::MeltOut => {
                self.t += dt;
                if self.t < MELT_OUT * self.scale {
                    return None;
                }
                // Don't carry leftover time: the reform always starts from a
                // full puddle, even after a long (dream-loading) frame.
                self.t = 0.0;
                self.phase = if self.pending.reforms() {
                    Phase::ReformIn
                } else {
                    Phase::Idle
                };
                Some(Event::Swap(self.pending))
            }
            Phase::ReformIn => {
                self.t += dt;
                if self.t < REFORM * self.scale {
                    return None;
                }
                self.t = 0.0;
                self.phase = Phase::Idle;
                Some(Event::Done)
            }
        }
    }

    /// 0 = solid, 1 = fully melted. Smoothstep-eased: a gentle start, then
    /// the collapse accelerates, like something giving way.
    pub fn melt(&self) -> f32 {
        let x = match self.phase {
            Phase::Idle => return 0.0,
            Phase::MeltOut => self.t / (MELT_OUT * self.scale),
            Phase::ReformIn => 1.0 - self.t / (REFORM * self.scale),
        }
        .clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    }

    /// The fog colour to render with. While melting out it slides from the
    /// current dream's fog toward the next dream's, so the swap is invisible.
    /// Otherwise (reforming, idle) the current dream's own fog is correct.
    pub fn fog(&self, current: [f32; 3]) -> [f32; 3] {
        if self.phase != Phase::MeltOut {
            return current;
        }
        let m = self.melt();
        std::array::from_fn(|i| current[i] + (self.target_fog[i] - current[i]) * m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// Ticks until an event fires (or `secs` elapse); returns it and the
    /// melt() values seen along the way.
    fn run(tr: &mut Transition, secs: f32) -> (Option<Event>, Vec<f32>) {
        let mut seen = Vec::new();
        for _ in 0..(secs / DT) as usize {
            let ev = tr.tick(DT);
            seen.push(tr.melt());
            if ev.is_some() {
                return (ev, seen);
            }
        }
        (None, seen)
    }

    #[test]
    fn idle_is_solid_and_silent() {
        let mut tr = Transition::new(1.0);
        assert!(!tr.active());
        assert_eq!(tr.melt(), 0.0);
        assert_eq!(tr.tick(1.0), None);
        assert_eq!(tr.fog([0.1, 0.2, 0.3]), [0.1, 0.2, 0.3]);
    }

    #[test]
    fn a_running_transition_cannot_be_restarted() {
        let mut tr = Transition::new(1.0);
        assert!(tr.start(Pending::Descend, [0.0; 3]));
        assert!(!tr.start(Pending::Wake, [1.0; 3]));
        let (ev, _) = run(&mut tr, 5.0);
        assert_eq!(
            ev,
            Some(Event::Swap(Pending::Descend)),
            "first request wins"
        );
    }

    #[test]
    fn melts_down_swaps_once_then_reforms() {
        let mut tr = Transition::new(1.0);
        tr.start(Pending::Descend, [0.0; 3]);
        let (ev, down) = run(&mut tr, 5.0);
        assert_eq!(ev, Some(Event::Swap(Pending::Descend)));
        assert!(
            down.windows(2).all(|w| w[1] >= w[0]),
            "melt-out never un-melts"
        );
        assert!(tr.active(), "still reforming after the swap");
        assert!(tr.melt() > 0.95, "reform starts from a full puddle");
        let (ev, up) = run(&mut tr, 5.0);
        assert_eq!(ev, Some(Event::Done));
        assert!(up.windows(2).all(|w| w[1] <= w[0]), "reform never re-melts");
        assert!(!tr.active());
        assert_eq!(tr.melt(), 0.0);
        assert_eq!(run(&mut tr, 2.0).0, None, "no stray events after Done");
    }

    #[test]
    fn finishing_the_run_melts_out_and_never_reforms() {
        let mut tr = Transition::new(1.0);
        tr.start(Pending::Finish, [0.0; 3]);
        let (ev, _) = run(&mut tr, 5.0);
        assert_eq!(ev, Some(Event::Swap(Pending::Finish)));
        assert!(!tr.active());
        assert_eq!(run(&mut tr, 5.0).0, None);
    }

    #[test]
    fn a_long_frame_still_starts_the_reform_from_full() {
        let mut tr = Transition::new(1.0);
        tr.start(Pending::Wake, [0.0; 3]);
        assert_eq!(tr.tick(10.0), Some(Event::Swap(Pending::Wake)));
        assert!(tr.melt() > 0.99, "leftover time must not skip the reform");
    }

    #[test]
    fn fog_slides_to_the_next_dream_while_melting() {
        let mut tr = Transition::new(1.0);
        let (old, new) = ([0.0, 0.0, 0.0], [1.0, 0.5, 0.25]);
        tr.start(Pending::Descend, new);
        run(&mut tr, MELT_OUT * 0.5);
        let mid = tr.fog(old);
        assert!(mid[0] > 0.1 && mid[0] < 0.9, "halfway-ish: {mid:?}");
        tr.tick(MELT_OUT);
        // After the swap the new dream's own fog is passed in; use it as-is.
        assert_eq!(tr.fog(new), new);
    }

    #[test]
    fn scale_stretches_both_halves() {
        let mut slow = Transition::new(2.0);
        slow.start(Pending::Descend, [0.0; 3]);
        assert_eq!(slow.tick(MELT_OUT * 1.5), None, "not melted yet at 1.5x");
        assert_eq!(slow.tick(MELT_OUT), Some(Event::Swap(Pending::Descend)));
        assert_eq!(slow.tick(REFORM * 1.5), None);
        assert_eq!(slow.tick(REFORM), Some(Event::Done));
    }

    #[test]
    fn both_shaders_listen_for_the_melt() {
        let vert = include_str!("../assets/shaders/mesh.vert");
        let frag = include_str!("../assets/shaders/mesh.frag");
        assert!(vert.contains("uniform float uMelt;"), "mesh.vert has no uMelt");
        assert!(frag.contains("uniform float uMelt;"), "mesh.frag has no uMelt");
    }
}
