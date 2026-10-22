//! Shareable seed codes: `DS-XXXXX-XXXXX-XXXXX`.
//!
//! A run is fully determined by its seed and its length, so a short code is
//! all a friend needs to dream the same dreams. No servers: the code is the
//! whole message, and it is easy to read aloud or type by hand.
//!
//! Layout: 65 payload bits (the 64-bit seed, then 1 bit for "long") as 13
//! Crockford base32 digits, then 2 digits of checksum, 15 in all. Crockford's
//! alphabet leaves out I, L, O and U, and typing `O` for `0` or `I`/`L` for
//! `1` is forgiven, so a misread never silently becomes a different run: a
//! wrong digit fails the checksum instead.

use std::fmt;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Digits in a code, without the `DS` prefix or dashes.
pub const DIGITS: usize = 15;
const PAYLOAD_DIGITS: usize = 13;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeError {
    /// Fewer than 15 digits so far.
    TooShort,
    TooLong,
    /// A character that is not in the alphabet.
    BadChar(char),
    /// All digits are valid but they do not belong together (a typo).
    BadCheck,
}

impl fmt::Display for CodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodeError::TooShort => write!(f, "that code is too short"),
            CodeError::TooLong => write!(f, "that code is too long"),
            CodeError::BadChar(c) => write!(f, "'{c}' is not part of a dream code"),
            CodeError::BadCheck => write!(f, "that code has a typo in it"),
        }
    }
}

/// 10 bits that depend on every payload bit.
fn check(payload: u128) -> u16 {
    let mut h = (payload as u64) ^ ((payload >> 64) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h & 0x3FF) as u16
}

/// The code for a run, grouped for reading: `DS-XXXXX-XXXXX-XXXXX`.
pub fn encode(seed: u64, long: bool) -> String {
    let payload = ((seed as u128) << 1) | long as u128;
    let mut digits = [0u8; DIGITS];
    for (i, d) in digits.iter_mut().take(PAYLOAD_DIGITS).enumerate() {
        let shift = 5 * (PAYLOAD_DIGITS - 1 - i);
        *d = ALPHABET[((payload >> shift) & 31) as usize];
    }
    let c = check(payload);
    digits[PAYLOAD_DIGITS] = ALPHABET[(c >> 5) as usize & 31];
    digits[PAYLOAD_DIGITS + 1] = ALPHABET[(c & 31) as usize];
    group(std::str::from_utf8(&digits).expect("ascii"))
}

/// `DS-` plus the digits in groups of five (works for partial input too).
pub fn group(digits: &str) -> String {
    let mut out = String::from("DS");
    for (i, c) in digits.chars().enumerate() {
        if i % 5 == 0 {
            out.push('-');
        }
        out.push(c);
    }
    if digits.is_empty() {
        out.push('-');
    }
    out
}

fn value(c: char) -> Option<u8> {
    let c = match c.to_ascii_uppercase() {
        'O' => '0',
        'I' | 'L' => '1',
        c => c,
    };
    ALPHABET.iter().position(|&a| a as char == c).map(|p| p as u8)
}

/// What a typed or pasted string reduces to: upper case, separators gone,
/// a leading `DS-` prefix dropped. Does not validate the digits.
pub fn clean(input: &str) -> String {
    let all: String = input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    // `DS` are legal digits too, so only treat them as the prefix when the
    // length says they are one.
    if all.len() == DIGITS + 2 && all.starts_with("DS") {
        all[2..].to_string()
    } else {
        all
    }
}

/// Add one typed character to the box. Returns whether it was taken. Keeps
/// the buffer to valid digits, upper case, at most [`DIGITS`] long.
pub fn push(buf: &mut String, c: char) -> bool {
    if !c.is_ascii_alphanumeric() || buf.len() >= DIGITS {
        return false;
    }
    let up = c.to_ascii_uppercase();
    let canon = match up {
        'O' => '0',
        'I' | 'L' => '1',
        c => c,
    };
    if value(canon).is_none() {
        return false;
    }
    buf.push(canon);
    true
}

/// Read a code back into `(seed, long)`.
pub fn decode(input: &str) -> Result<(u64, bool), CodeError> {
    let digits = clean(input);
    if digits.len() < DIGITS {
        return Err(CodeError::TooShort);
    }
    if digits.len() > DIGITS {
        return Err(CodeError::TooLong);
    }
    let mut v = Vec::with_capacity(DIGITS);
    for c in digits.chars() {
        v.push(value(c).ok_or(CodeError::BadChar(c))?);
    }
    let payload = v[..PAYLOAD_DIGITS]
        .iter()
        .fold(0u128, |acc, &d| (acc << 5) | d as u128);
    // 13 digits hold 65 bits; the top bit of the first digit must be a seed bit,
    // and nothing may spill past 65 bits.
    if payload >> 65 != 0 {
        return Err(CodeError::BadCheck);
    }
    let got = ((v[PAYLOAD_DIGITS] as u16) << 5) | v[PAYLOAD_DIGITS + 1] as u16;
    if got != check(payload) {
        return Err(CodeError::BadCheck);
    }
    Ok(((payload >> 1) as u64, payload & 1 == 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_looks_like_ds_and_three_groups_of_five() {
        let c = encode(123_456_789, false);
        assert_eq!(c.len(), 2 + 3 * 6, "{c}");
        assert!(c.starts_with("DS-"));
        assert_eq!(c.matches('-').count(), 3);
        assert!(c.chars().all(|ch| ch == '-' || ALPHABET.contains(&(ch as u8)) || ch == 'S'));
    }

    #[test]
    fn every_kind_of_seed_round_trips() {
        let seeds = [0, 1, 2, 42, u64::MAX, u64::MAX - 1, 1 << 63, 0xDEAD_BEEF_CAFE_F00D];
        for s in seeds {
            for long in [false, true] {
                assert_eq!(decode(&encode(s, long)), Ok((s, long)), "seed {s} long {long}");
            }
        }
        // And a pile of generated ones, the way real runs are seeded.
        let mut s = 7u64;
        for _ in 0..2000 {
            s = crate::gameplay::next_run_seed(s);
            assert_eq!(decode(&encode(s, s % 2 == 0)), Ok((s, s % 2 == 0)));
        }
    }

    #[test]
    fn different_runs_get_different_codes() {
        assert_ne!(encode(5, false), encode(5, true));
        assert_ne!(encode(5, false), encode(6, false));
    }

    #[test]
    fn reading_is_forgiving_about_case_spaces_dashes_and_lookalikes() {
        let c = encode(0x1234_5678_9ABC_DEF0, true);
        let want = Ok((0x1234_5678_9ABC_DEF0, true));
        assert_eq!(decode(&c.to_lowercase()), want);
        assert_eq!(decode(&c.replace('-', " ")), want);
        assert_eq!(decode(&c.replace('-', "")), want);
        assert_eq!(decode(&format!("  {c}\n")), want);
        // O for 0 and I/L for 1 read as the digits they look like.
        let looky = c.replace('0', "O").replace('1', "l");
        assert_eq!(decode(&looky), want);
    }

    #[test]
    fn a_code_without_the_ds_prefix_still_reads() {
        let c = encode(99, false);
        let bare = clean(&c);
        assert_eq!(bare.len(), DIGITS);
        assert_eq!(decode(&bare), Ok((99, false)));
    }

    #[test]
    fn a_seed_whose_digits_start_with_ds_is_not_mistaken_for_the_prefix() {
        // Find a code whose bare digits begin with D,S and make sure both
        // spellings (with and without the prefix) still decode.
        let mut s = 1u64;
        let mut found = false;
        for _ in 0..400_000 {
            s = crate::gameplay::next_run_seed(s);
            let bare = clean(&encode(s, false));
            if bare.starts_with("DS") {
                assert_eq!(decode(&bare), Ok((s, false)));
                assert_eq!(decode(&encode(s, false)), Ok((s, false)));
                found = true;
                break;
            }
        }
        assert!(found, "no seed started with DS in the search range");
    }

    #[test]
    fn every_single_digit_typo_is_caught() {
        let code = clean(&encode(0xABCD_EF01_2345_6789, true));
        for i in 0..DIGITS {
            for &repl in ALPHABET.iter() {
                let mut b = code.clone().into_bytes();
                if b[i] == repl {
                    continue;
                }
                b[i] = repl;
                let bad = String::from_utf8(b).unwrap();
                assert!(
                    decode(&bad).is_err(),
                    "typo at {i} ({}) became a different valid run",
                    repl as char
                );
            }
        }
    }

    #[test]
    fn two_swapped_neighbours_are_caught() {
        let code = clean(&encode(0x0F0F_1234_5678_9ABC, false));
        let mut missed = 0;
        let mut tried = 0;
        for i in 0..DIGITS - 1 {
            let mut b = code.clone().into_bytes();
            if b[i] == b[i + 1] {
                continue;
            }
            b.swap(i, i + 1);
            tried += 1;
            if decode(&String::from_utf8(b).unwrap()).is_ok() {
                missed += 1;
            }
        }
        assert!(tried > 5);
        assert_eq!(missed, 0, "a 10-bit check should catch these here");
    }

    #[test]
    fn errors_say_what_is_wrong() {
        assert_eq!(decode(""), Err(CodeError::TooShort));
        assert_eq!(decode("DS-ABCDE"), Err(CodeError::TooShort));
        let good = clean(&encode(1, false));
        assert_eq!(decode(&format!("{good}0")), Err(CodeError::TooLong));
        assert_eq!(decode(&format!("U{}", &good[1..])), Err(CodeError::BadChar('U')));
        let mut b = good.into_bytes();
        b[3] = if b[3] == b'7' { b'8' } else { b'7' };
        assert_eq!(decode(&String::from_utf8(b).unwrap()), Err(CodeError::BadCheck));
        assert!(!CodeError::BadCheck.to_string().is_empty());
    }

    #[test]
    fn typing_keeps_the_box_clean_and_capped() {
        let mut b = String::new();
        for c in "ds-1o l!U 2".chars() {
            push(&mut b, c);
        }
        // d,s,1,0,1 kept; '-', '!', ' ' and U refused; 2 kept.
        assert_eq!(b, "DS1012");
        for c in "ABCDEFGHJKMNPQRSTVWXYZ".chars() {
            push(&mut b, c);
        }
        assert_eq!(b.len(), DIGITS);
        assert!(!push(&mut b, 'A'));
    }

    #[test]
    fn partial_input_is_grouped_as_you_type() {
        assert_eq!(group(""), "DS-");
        assert_eq!(group("AB"), "DS-AB");
        assert_eq!(group("ABCDE"), "DS-ABCDE");
        assert_eq!(group("ABCDEF"), "DS-ABCDE-F");
        assert_eq!(group("ABCDEFGHJKMNPQR"), "DS-ABCDE-FGHJK-MNPQR");
    }
}
