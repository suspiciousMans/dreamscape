//! Chooses which dream comes next: Lobby → 4 weighted-random dreams → Awakening.

use super::theme::DreamTheme;
use rand::distributions::{Distribution, WeightedIndex};
use rand::{rngs::StdRng, SeedableRng};

pub const DREAMS_PER_RUN: u32 = 6;

pub struct DreamDirector {
    rng: StdRng,
    run_seed: u64,
    pub depth: u32,
    pub theme: DreamTheme,
}

impl DreamDirector {
    pub fn new(run_seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(run_seed),
            run_seed,
            depth: 0,
            theme: DreamTheme::Lobby,
        }
    }

    /// Seed for the current dream's content (distinct per depth).
    pub fn dream_seed(&self) -> u64 {
        self.run_seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(self.depth as u64)
    }

    /// Shift to the next dream. `None` = the player woke up.
    pub fn advance(&mut self) -> Option<DreamTheme> {
        if self.theme == DreamTheme::Awakening {
            return None;
        }
        self.depth += 1;
        self.theme = if self.depth + 1 >= DREAMS_PER_RUN {
            DreamTheme::Awakening
        } else {
            let next = self.theme.spec().next;
            let dist = WeightedIndex::new(next.iter().map(|&(_, w)| w))
                .expect("non-final themes have weighted exits (theme tests)");
            next[dist.sample(&mut self.rng)].0
        };
        Some(self.theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn run(seed: u64) -> Vec<DreamTheme> {
        let mut d = DreamDirector::new(seed);
        let mut seq = vec![d.theme];
        while let Some(t) = d.advance() {
            seq.push(t);
        }
        seq
    }

    #[test]
    fn every_run_is_lobby_dreams_awakening() {
        for seed in 0..200 {
            let r = run(seed);
            assert_eq!(r.len(), DREAMS_PER_RUN as usize, "{r:?}");
            assert_eq!(r[0], DreamTheme::Lobby);
            assert_eq!(*r.last().unwrap(), DreamTheme::Awakening);
            assert!(!r[..r.len() - 1].contains(&DreamTheme::Awakening), "{r:?}");
            assert!(r.windows(2).all(|w| w[0] != w[1]), "repeat in {r:?}");
        }
    }

    #[test]
    fn runs_are_reproducible() {
        assert_eq!(run(42), run(42));
    }

    #[test]
    fn runs_vary() {
        let distinct: HashSet<Vec<DreamTheme>> = (0..50).map(run).collect();
        assert!(
            distinct.len() >= 10,
            "only {} distinct runs",
            distinct.len()
        );
    }
}
