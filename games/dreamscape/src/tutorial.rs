//! The prologue: three short dreams (the Lobby, the Garden, then the Lobby
//! again with the wake door), the eye walking you through the basics.

use crate::dream::{DreamDirector, DreamTheme, RunLength};

/// Fixed so every new player walks the same first dreams.
pub const PROLOGUE_SEED: u64 = 0x5EED_0001;

/// The prologue's director: Lobby, Garden, Lobby; one shard wakes you.
pub fn director() -> DreamDirector {
    let mut d = DreamDirector::with_length(PROLOGUE_SEED, RunLength::Short);
    d.shards_to_wake = 1;
    d.forced = vec![DreamTheme::Garden, DreamTheme::Lobby];
    d.next = DreamTheme::Garden;
    d
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Move,
    Jump,
    Note,
    Portal,
    Shard,
    Enemy,
    Wake,
    Done,
}

#[cfg(test)]
pub const ALL_STEPS: [Step; 7] = [
    Step::Move,
    Step::Jump,
    Step::Note,
    Step::Portal,
    Step::Shard,
    Step::Enemy,
    Step::Wake,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Metres walked since the step began.
    Moved(f32),
    Jumped,
    NoteRead,
    Portal,
    ShardTaken,
    Woke,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prologue {
    pub step: Step,
    walked: f32,
}

impl Default for Prologue {
    fn default() -> Self {
        Self {
            step: Step::Move,
            walked: 0.0,
        }
    }
}

impl Prologue {
    pub fn on(&mut self, e: Event) {
        let next = match (self.step, e) {
            (Step::Move, Event::Moved(m)) => {
                self.walked += m;
                (self.walked >= 3.0).then_some(Step::Jump)
            }
            (Step::Jump, Event::Jumped) => Some(Step::Note),
            (Step::Note, Event::NoteRead) => Some(Step::Portal),
            (Step::Portal, Event::Portal) => Some(Step::Shard),
            (Step::Shard, Event::ShardTaken) => Some(Step::Enemy),
            (Step::Enemy, Event::Portal) => Some(Step::Wake),
            (Step::Wake, Event::Woke) => Some(Step::Done),
            _ => None,
        };
        if let Some(s) = next {
            self.step = s;
        }
    }

    #[cfg(test)]
    pub fn done(&self) -> bool {
        self.step == Step::Done
    }
}

/// What the eye says for each step (keys rewritten for controllers by `pad`).
pub fn prompt(s: Step) -> &'static str {
    match s {
        Step::Move => "You're dreaming. Walk with me. [WASD]",
        Step::Jump => "Good. Now jump. [SPACE]",
        Step::Note => "Something of yours is lying there. Pick it up.",
        Step::Portal => "The light in the floor goes deeper. Step into it.",
        Step::Shard => "That shape is lucidity. Follow the arrow and take it.",
        Step::Enemy => "Something's awake. Don't let it touch you. Get past it.",
        Step::Wake => "You're lucid. The door will take you up. Choose WAKE.",
        Step::Done => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_advance_only_on_their_own_event() {
        let mut t = Prologue::default();
        assert_eq!(t.step, Step::Move);
        t.on(Event::Jumped);
        assert_eq!(t.step, Step::Move, "jumping doesn't finish 'move'");
        t.on(Event::Moved(3.0));
        assert_eq!(t.step, Step::Jump);
        for e in [
            Event::Jumped,
            Event::NoteRead,
            Event::Portal,
            Event::ShardTaken,
            Event::Portal,
            Event::Woke,
        ] {
            t.on(e);
        }
        assert!(t.done());
    }

    #[test]
    fn the_prologue_garden_has_a_shard_spot_and_an_enemy() {
        let mut d = director();
        assert_eq!(d.descend(), Some(DreamTheme::Garden));
        let g = crate::dream::generate_with(
            DreamTheme::Garden,
            d.dream_seed(),
            d.depth,
            None,
            true,
            crate::dream::Pressure::default(),
            Default::default(),
        );
        assert!(g.shard.is_some());
        assert!(!g.patrols.is_empty(), "no enemy to teach dodging");
        assert_eq!(d.descend(), Some(DreamTheme::Lobby));
        assert!(!d.nightmare);
    }

    #[test]
    fn every_step_says_something_and_names_real_keys() {
        for s in ALL_STEPS {
            let l = prompt(s);
            assert!(!l.is_empty());
            assert!(l.matches('[').count() == l.matches(']').count(), "{l}");
        }
    }
}
