//! Input prompts: a pulsing key/button chip says so when an action can be
//! done (or must be). Pure and context-only: nothing here reads a clock or a
//! device, and a chip never names a key that does not do the thing.

use crate::hud::Mode;
use crate::pad;
use crate::settings::{Action, Settings};
use crate::ui_motion::UiJuice;
use engine::sdl2::keyboard::Keycode;
use std::f32::consts::TAU;

/// How near (world units) a door or shard must be to count as "in reach".
pub const REACH: f32 = 2.5;
/// Bob, in px, and the rates of the two pulses (Hz).
pub const BOB_PX: f32 = 3.0;
pub const AVAILABLE_HZ: f32 = 1.2;
pub const REQUIRED_HZ: f32 = 1.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Can be done right now.
    Available,
    /// A blocking choice: the rest of the screen dims.
    Required,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // M4 screens construct Reroll, Back, Sort, Export
pub enum ActionId {
    Strike,
    UseCard(u8),
    Wake,
    GoDeeper,
    PickUpgrade,
    OpenPack,
    Interact,
    Reroll,
    Confirm,
    Back,
    Sort,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prompt {
    pub action: ActionId,
    pub state: State,
    /// Screen position (0..1 each way) if the chip hangs on something.
    pub target: Option<[f32; 2]>,
}

/// The kind of blocking choice on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Upgrade,
    Event,
    /// WAKE / GO DEEPER; `deeper` is whether the second is highlighted.
    WakeDoor { deeper: bool },
}

/// What the game knows this frame.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub mode: Mode,
    pub door_in_reach: bool,
    pub shard_in_reach: bool,
    /// Off cooldown, with something to hit in reach and arc.
    pub strike_ready: bool,
    pub cards_ready: [bool; 5],
    pub pack_unopened: bool,
    pub choice: Option<Choice>,
    /// The action the menu's cursor is on (menus only).
    pub focused: Option<ActionId>,
}

impl Ctx {
    pub fn new(mode: Mode) -> Self {
        Ctx {
            mode,
            door_in_reach: false,
            shard_in_reach: false,
            strike_ready: false,
            cards_ready: [false; 5],
            pack_unopened: false,
            choice: None,
            focused: None,
        }
    }
}

fn avail(action: ActionId) -> Prompt {
    Prompt { action, state: State::Available, target: None }
}

/// The chips to show. In play: one per available action (never Required).
/// In a blocking choice: exactly the one required action. In any other menu:
/// the focused action, at most one chip.
pub fn compute(ctx: &Ctx) -> Vec<Prompt> {
    match ctx.mode {
        Mode::Playing => {
            let mut out = Vec::new();
            if ctx.strike_ready {
                out.push(avail(ActionId::Strike));
            }
            for (i, &ready) in ctx.cards_ready.iter().enumerate() {
                if ready {
                    out.push(avail(ActionId::UseCard(i as u8)));
                }
            }
            if ctx.door_in_reach || ctx.shard_in_reach {
                out.push(avail(ActionId::Interact));
            }
            out
        }
        Mode::Choice => match ctx.choice {
            Some(c) => {
                let action = match c {
                    Choice::Upgrade => ActionId::PickUpgrade,
                    Choice::Event => ActionId::Confirm,
                    Choice::WakeDoor { deeper: false } => ActionId::Wake,
                    Choice::WakeDoor { deeper: true } => ActionId::GoDeeper,
                };
                vec![Prompt { action, state: State::Required, target: None }]
            }
            None => Vec::new(),
        },
        Mode::PackPick | Mode::Reveal if ctx.pack_unopened => vec![avail(ActionId::OpenPack)],
        _ => ctx.focused.map(avail).into_iter().collect(),
    }
}

/// Only a Required prompt dims the rest of the screen.
pub fn dims_rest(prompts: &[Prompt]) -> bool {
    prompts.iter().any(|p| p.state == State::Required)
}

/// (y offset in px, glow 0..1). Off and reduced motion are constant.
pub fn bob(t: f32, state: State, juice: UiJuice, reduced: bool) -> (f32, f32) {
    if reduced || juice == UiJuice::Off {
        return (0.0, 0.6);
    }
    let hz = match state {
        State::Available => AVAILABLE_HZ,
        State::Required => REQUIRED_HZ,
    };
    let s = (t * hz * TAU).sin();
    (BOB_PX * s, (0.55 + 0.45 * s).clamp(0.0, 1.0))
}

/// `reduce_flashing` halves how far the glow swings around its middle.
pub fn soften(glow: f32, reduce_flashing: bool) -> f32 {
    if reduce_flashing {
        0.55 + (glow - 0.55) * 0.5
    } else {
        glow
    }
}

/// A short caption beside the key (empty for most chips).
pub fn caption(action: ActionId) -> &'static str {
    match action {
        ActionId::Wake => "WAKE",
        ActionId::GoDeeper => "GO DEEPER",
        _ => "",
    }
}

/// The key this action is bound to. `None` = nothing does it (never guessed).
fn key_of(action: ActionId, s: &Settings) -> Option<Keycode> {
    match action {
        ActionId::Strike => s.key_for(Action::Strike),
        ActionId::UseCard(i) => match i {
            0 => s.key_for(Action::Ability1),
            1 => s.key_for(Action::Ability2),
            2 => s.key_for(Action::Ability3),
            3 => s.key_for(Action::Ability4),
            4 => s.key_for(Action::Ability5),
            _ => None,
        },
        ActionId::Wake
        | ActionId::GoDeeper
        | ActionId::PickUpgrade
        | ActionId::OpenPack
        | ActionId::Confirm => Some(Keycode::Return),
        ActionId::Reroll => Some(Keycode::R),
        ActionId::Back => Some(Keycode::Escape),
        ActionId::Sort => Some(Keycode::T),
        ActionId::Export => Some(Keycode::P),
        // Doors and shards open by touch: there is no key to name.
        ActionId::Interact => None,
    }
}

/// A key as a chip writes it.
pub fn key_text(k: Keycode) -> String {
    match k {
        Keycode::Return => "ENTER".into(),
        Keycode::Escape => "ESC".into(),
        Keycode::LShift | Keycode::RShift => "SHIFT".into(),
        k => k.name().to_uppercase(),
    }
}

/// The chip's text for this action on this screen: the bound key, or the
/// controller button that presses it when `pad` is set. Empty when nothing
/// is bound (the painter then draws no chip).
pub fn label(action: ActionId, mode: Mode, settings: &Settings, pad: bool) -> String {
    let Some(key) = key_of(action, settings) else {
        return String::new();
    };
    if pad {
        pad::button_for(key, mode, settings)
            .map(|b| pad::button_label(b).to_string())
            .unwrap_or_default()
    } else {
        key_text(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODES: [Mode; 12] = [
        Mode::Title,
        Mode::Playing,
        Mode::Paused,
        Mode::Reveal,
        Mode::PackPick,
        Mode::Store,
        Mode::Booklet,
        Mode::Choice,
        Mode::Summary,
        Mode::Settings,
        Mode::Codex,
        Mode::Memories,
    ];

    fn has(p: &[Prompt], a: ActionId) -> bool {
        p.iter().any(|x| x.action == a)
    }

    #[test]
    fn play_chips_appear_only_when_the_action_can_be_done() {
        let mut c = Ctx::new(Mode::Playing);
        assert!(compute(&c).is_empty());
        c.strike_ready = true;
        assert!(has(&compute(&c), ActionId::Strike));
        c.strike_ready = false;
        c.cards_ready = [false, true, false, false, true];
        let p = compute(&c);
        assert!(has(&p, ActionId::UseCard(1)) && has(&p, ActionId::UseCard(4)));
        assert!(!has(&p, ActionId::UseCard(0)) && !has(&p, ActionId::Strike));
        let mut c = Ctx::new(Mode::Playing);
        c.door_in_reach = true;
        assert!(has(&compute(&c), ActionId::Interact));
        let mut c = Ctx::new(Mode::Playing);
        c.shard_in_reach = true;
        assert!(has(&compute(&c), ActionId::Interact));
        // Open play never has a Required chip, whatever is true.
        let all = Ctx {
            strike_ready: true,
            cards_ready: [true; 5],
            door_in_reach: true,
            shard_in_reach: true,
            pack_unopened: true,
            choice: Some(Choice::Upgrade),
            ..Ctx::new(Mode::Playing)
        };
        let p = compute(&all);
        assert_eq!(p.len(), 7);
        assert!(!dims_rest(&p));
        assert!(p.iter().all(|x| x.state == State::Available));
    }

    #[test]
    fn packs_chip_only_while_one_is_unopened() {
        for m in [Mode::PackPick, Mode::Reveal] {
            let mut c = Ctx::new(m);
            assert!(compute(&c).is_empty());
            c.pack_unopened = true;
            assert_eq!(compute(&c)[0].action, ActionId::OpenPack);
        }
        let mut c = Ctx::new(Mode::Playing);
        c.pack_unopened = true;
        assert!(compute(&c).is_empty());
    }

    #[test]
    fn required_only_in_choice_and_only_there_does_it_dim() {
        for choice in [
            Choice::Upgrade,
            Choice::Event,
            Choice::WakeDoor { deeper: false },
            Choice::WakeDoor { deeper: true },
        ] {
            let mut c = Ctx::new(Mode::Choice);
            c.choice = Some(choice);
            let p = compute(&c);
            assert_eq!(p.len(), 1);
            assert_eq!(p[0].state, State::Required);
            assert!(dims_rest(&p));
        }
        let wake = |deeper| {
            let mut c = Ctx::new(Mode::Choice);
            c.choice = Some(Choice::WakeDoor { deeper });
            compute(&c)[0].action
        };
        assert_eq!(wake(false), ActionId::Wake);
        assert_eq!(wake(true), ActionId::GoDeeper);
        for m in MODES.into_iter().filter(|&m| m != Mode::Choice) {
            let c = Ctx {
                choice: Some(Choice::Upgrade),
                focused: Some(ActionId::Confirm),
                strike_ready: true,
                pack_unopened: true,
                ..Ctx::new(m)
            };
            assert!(!dims_rest(&compute(&c)), "{m:?} dimmed");
        }
        assert!(compute(&Ctx::new(Mode::Choice)).is_empty());
        assert!(!dims_rest(&[]));
    }

    #[test]
    fn menus_give_at_most_one_chip_for_the_focused_action() {
        for m in MODES.into_iter().filter(|&m| m != Mode::Playing) {
            for focused in [None, Some(ActionId::Sort), Some(ActionId::Back)] {
                let c = Ctx {
                    focused,
                    pack_unopened: true,
                    choice: Some(Choice::WakeDoor { deeper: true }),
                    door_in_reach: true,
                    cards_ready: [true; 5],
                    strike_ready: true,
                    ..Ctx::new(m)
                };
                assert!(compute(&c).len() <= 1, "{m:?}");
            }
        }
        let c = Ctx { focused: Some(ActionId::Export), ..Ctx::new(Mode::Booklet) };
        let p = compute(&c);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].action, p[0].state), (ActionId::Export, State::Available));
    }

    #[test]
    fn bob_is_bounded_and_constant_when_off_or_reduced() {
        for state in [State::Available, State::Required] {
            let base = bob(0.0, state, UiJuice::Soft, true);
            for i in 0..200 {
                let t = i as f32 * 0.037;
                assert_eq!(bob(t, state, UiJuice::Soft, true), base);
                assert_eq!(bob(t, state, UiJuice::Off, false), base);
                for j in [UiJuice::Soft, UiJuice::Full] {
                    let (y, g) = bob(t, state, j, false);
                    assert!(y.abs() <= BOB_PX + 1e-4, "{y}");
                    assert!((0.0..=1.0).contains(&g), "{g}");
                }
            }
        }
        // 1.2 Hz repeats every 1/1.2 s; 1.8 Hz every 1/1.8 s.
        let a = bob(0.3, State::Available, UiJuice::Soft, false);
        let b = bob(0.3 + 1.0 / AVAILABLE_HZ, State::Available, UiJuice::Soft, false);
        assert!((a.0 - b.0).abs() < 1e-3);
        let a = bob(0.2, State::Required, UiJuice::Soft, false);
        let b = bob(0.2 + 1.0 / REQUIRED_HZ, State::Required, UiJuice::Soft, false);
        assert!((a.0 - b.0).abs() < 1e-3);
    }

    #[test]
    fn reduce_flashing_halves_the_glow_swing() {
        for i in 0..100 {
            let (_, g) = bob(i as f32 * 0.05, State::Required, UiJuice::Full, false);
            let soft = soften(g, true);
            assert!((soft - 0.55).abs() <= (g - 0.55).abs() * 0.5 + 1e-5);
            assert_eq!(soften(g, false), g);
        }
    }

    #[test]
    fn labels_match_the_bound_key_and_never_invent_one() {
        let mut s = Settings::default();
        let acts = [
            ActionId::Strike,
            ActionId::UseCard(0),
            ActionId::UseCard(4),
            ActionId::Wake,
            ActionId::GoDeeper,
            ActionId::PickUpgrade,
            ActionId::OpenPack,
            ActionId::Reroll,
            ActionId::Confirm,
            ActionId::Back,
            ActionId::Sort,
            ActionId::Export,
        ];
        for pass in 0..2 {
            for &a in &acts {
                for m in MODES {
                    let key = key_of(a, &s).expect("bound");
                    assert_eq!(label(a, m, &s, false), key_text(key));
                    let l = label(a, m, &s, true);
                    match pad::button_for(key, m, &s) {
                        Some(b) => {
                            assert_eq!(l, pad::button_label(b));
                            assert!(pad::key_for(b, m, &s).is_some());
                        }
                        None => assert!(l.is_empty(), "{a:?} {m:?}: {l}"),
                    }
                }
            }
            // Rebinding follows the setting; the second pass uses new keys.
            if pass == 0 {
                s.bind(Action::Strike, Keycode::G);
                assert_eq!(label(ActionId::Strike, Mode::Playing, &s, false), "G");
                s.bind(Action::Ability2, Keycode::H);
                assert_eq!(label(ActionId::UseCard(1), Mode::Playing, &s, false), "H");
            }
        }
        // Nothing is bound to Interact, and out-of-range cards: no text.
        for m in MODES {
            for pad in [false, true] {
                assert_eq!(label(ActionId::Interact, m, &s, pad), "");
                assert_eq!(label(ActionId::UseCard(9), m, &s, pad), "");
            }
        }
    }

    #[test]
    fn play_pad_labels_name_the_button_that_strikes() {
        let s = Settings::default();
        assert_eq!(label(ActionId::Strike, Mode::Playing, &s, true), "B");
        assert_eq!(label(ActionId::UseCard(1), Mode::Playing, &s, true), "X");
        assert_eq!(label(ActionId::Confirm, Mode::Choice, &s, true), "A");
        assert_eq!(label(ActionId::Reroll, Mode::Choice, &s, true), "X");
    }

    #[test]
    fn captions_are_only_for_wake_and_go_deeper() {
        assert_eq!(caption(ActionId::Wake), "WAKE");
        assert_eq!(caption(ActionId::GoDeeper), "GO DEEPER");
        assert_eq!(caption(ActionId::Strike), "");
    }
}
