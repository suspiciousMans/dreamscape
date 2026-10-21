//! Forms: who the dreamer is this run (roadmap 4.4). Crystal is the default;
//! the others are earned and each trades something for something.

use crate::progress::Codex;
use crate::upgrades::Ability;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    #[default]
    Crystal,
    /// Starts with Blink; one fewer upgrade choice.
    Moth,
    /// Starts with Dash; the dreams run hotter.
    Ember,
    /// Sees further; walks slower.
    Pearl,
}

pub const ALL: [Form; 4] = [Form::Crystal, Form::Moth, Form::Ember, Form::Pearl];

impl Form {
    pub fn label(self) -> &'static str {
        match self {
            Form::Crystal => "crystal",
            Form::Moth => "moth",
            Form::Ember => "ember",
            Form::Pearl => "pearl",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Form::Crystal => "the plain dreamer · nothing gained, nothing lost",
            Form::Moth => "starts with BLINK · one fewer upgrade to choose from",
            Form::Ember => "starts with DASH · enemies run hotter",
            Form::Pearl => "sees further · walks slower",
        }
    }

    /// How a locked form is earned.
    pub fn unlock_hint(self) -> &'static str {
        match self {
            Form::Crystal => "",
            Form::Moth => "wake from a run",
            Form::Ember => "beat a nightmare",
            Form::Pearl => "wake from a long run",
        }
    }

    pub fn unlocked(self, c: &Codex) -> bool {
        match self {
            Form::Crystal => true,
            Form::Moth => c.fastest_wake.is_some() || c.deepest_long > 0 || c.bare_wake_depth > 0,
            Form::Ember => c.nightmares_beaten > 0,
            Form::Pearl => c.deepest_long > 0,
        }
    }

    pub fn start_ability(self) -> Option<Ability> {
        match self {
            Form::Moth => Some(Ability::Blink),
            Form::Ember => Some(Ability::Dash),
            _ => None,
        }
    }

    /// Upgrade choices taken away at every pick.
    pub fn fewer_choices(self) -> usize {
        usize::from(self == Form::Moth)
    }

    /// Multiplies the run's difficulty.
    pub fn difficulty_mult(self) -> f32 {
        if self == Form::Ember {
            1.15
        } else {
            1.0
        }
    }

    /// Multiplies how far the dreamer sees through the dark.
    pub fn sight_mult(self) -> f32 {
        if self == Form::Pearl {
            1.25
        } else {
            1.0
        }
    }

    /// Multiplies walking speed.
    pub fn speed_mult(self) -> f32 {
        if self == Form::Pearl {
            0.92
        } else {
            1.0
        }
    }

    /// The next (or previous) unlocked form.
    pub fn cycle(self, c: &Codex, dir: i32) -> Form {
        let open: Vec<Form> = ALL.iter().copied().filter(|f| f.unlocked(c)).collect();
        let at = open.iter().position(|&f| f == self).unwrap_or(0) as i32;
        open[(at + dir).rem_euclid(open.len() as i32) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_save_has_only_the_crystal() {
        let c = Codex::default();
        assert_eq!(ALL.iter().filter(|f| f.unlocked(&c)).count(), 1);
        assert_eq!(Form::Crystal.cycle(&c, 1), Form::Crystal);
        assert_eq!(Form::Crystal.cycle(&c, -1), Form::Crystal);
    }

    #[test]
    fn forms_unlock_from_what_the_codex_remembers() {
        let mut c = Codex::default();
        c.nightmares_beaten = 1;
        assert!(Form::Ember.unlocked(&c) && !Form::Moth.unlocked(&c));
        c.fastest_wake = Some(300);
        assert!(Form::Moth.unlocked(&c) && !Form::Pearl.unlocked(&c));
        c.deepest_long = 8;
        assert!(ALL.iter().all(|f| f.unlocked(&c)));
    }

    #[test]
    fn cycling_walks_the_unlocked_forms_and_wraps() {
        let mut c = Codex::default();
        c.nightmares_beaten = 1;
        c.fastest_wake = Some(1);
        // Crystal, Moth, Ember are open; Pearl is not.
        assert_eq!(Form::Crystal.cycle(&c, 1), Form::Moth);
        assert_eq!(Form::Ember.cycle(&c, 1), Form::Crystal);
        assert_eq!(Form::Crystal.cycle(&c, -1), Form::Ember);
    }

    #[test]
    fn every_form_trades_something_except_the_crystal() {
        for f in ALL {
            let changes = f.start_ability().is_some()
                || f.fewer_choices() > 0
                || f.difficulty_mult() != 1.0
                || f.sight_mult() != 1.0
                || f.speed_mult() != 1.0;
            assert_eq!(changes, f != Form::Crystal, "{f:?}");
        }
        // And each non-default form has a cost as well as a gift.
        assert!(Form::Moth.fewer_choices() > 0 && Form::Moth.start_ability().is_some());
        assert!(Form::Ember.difficulty_mult() > 1.0 && Form::Ember.start_ability().is_some());
        assert!(Form::Pearl.sight_mult() > 1.0 && Form::Pearl.speed_mult() < 1.0);
    }

    #[test]
    fn old_codexes_load_as_crystal() {
        let c: Codex = ron::from_str("(nightmares_beaten: 3)").unwrap();
        assert_eq!(c.form, Form::Crystal);
    }

    #[test]
    fn a_form_survives_ron() {
        let s = ron::to_string(&Form::Pearl).unwrap();
        assert_eq!(ron::from_str::<Form>(&s).unwrap(), Form::Pearl);
    }
}
