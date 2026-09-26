//! Controller support, per screen: which button stands in for which key, and
//! the on-screen prompts rewritten to match ("[r] reroll" -> "[X] reroll").
//! One table drives both, so a prompt can never name a button that doesn't
//! do the thing, and every action shown on a screen has a button.

use crate::hud::Mode;
use crate::settings::{Action, Settings};
use engine::sdl2::controller::Button;
use engine::sdl2::keyboard::Keycode;

pub const ALL_BUTTONS: [Button; 12] = [
    Button::A,
    Button::B,
    Button::X,
    Button::Y,
    Button::LeftShoulder,
    Button::RightShoulder,
    Button::Back,
    Button::Start,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
];

/// The key a controller button presses on this screen (`None` = nothing).
pub fn key_for(button: Button, mode: Mode, s: &Settings) -> Option<Keycode> {
    use Button as P;
    let dpad = match button {
        P::DPadUp => Some(Keycode::Up),
        P::DPadDown => Some(Keycode::Down),
        P::DPadLeft => Some(Keycode::Left),
        P::DPadRight => Some(Keycode::Right),
        _ => None,
    };
    match mode {
        Mode::Playing => match button {
            P::A => s.key_for(Action::Jump),
            P::LeftShoulder | P::RightShoulder => s.key_for(Action::Ability1),
            P::X | P::Y => s.key_for(Action::Ability2),
            P::Start => Some(Keycode::Escape),
            // Arrows turn the head in first-person dreams.
            _ => dpad.filter(|&k| k == Keycode::Left || k == Keycode::Right),
        },
        Mode::Paused => match button {
            P::A | P::B | P::Start => Some(Keycode::Escape),
            P::Y => Some(Keycode::Q),
            P::X => Some(Keycode::B),
            P::LeftShoulder => Some(Keycode::L),
            P::RightShoulder => Some(Keycode::O),
            P::Back => Some(Keycode::X),
            _ => None,
        },
        Mode::Choice => match button {
            P::A => Some(Keycode::Return),
            P::X => Some(Keycode::R),
            P::Y => Some(Keycode::X),
            _ => dpad,
        },
        Mode::Reveal | Mode::Journal => match button {
            P::A => Some(Keycode::Space),
            P::X => Some(Keycode::S),
            P::Y => Some(Keycode::R),
            P::Back => Some(Keycode::B),
            P::RightShoulder => Some(Keycode::L),
            P::B => Some(Keycode::Escape),
            _ => None,
        },
        Mode::Booklet => match button {
            P::LeftShoulder => Some(Keycode::Left),
            P::RightShoulder => Some(Keycode::Right),
            P::X => Some(Keycode::P),
            P::B => Some(Keycode::Escape),
            _ => dpad,
        },
        Mode::Store | Mode::Settings => match button {
            P::A => Some(Keycode::Return),
            P::B => Some(Keycode::Escape),
            _ => dpad,
        },
        // Deliberately no B here: on the title Escape quits the game.
        Mode::Title => match button {
            P::A | P::Start => Some(Keycode::Return),
            _ => dpad,
        },
        Mode::Summary => match button {
            P::A => Some(Keycode::Return),
            _ => None,
        },
        Mode::Codex => match button {
            P::B => Some(Keycode::Escape),
            _ => None,
        },
    }
}

pub fn button_label(b: Button) -> &'static str {
    match b {
        Button::A => "A",
        Button::B => "B",
        Button::X => "X",
        Button::Y => "Y",
        Button::LeftShoulder => "LB",
        Button::RightShoulder => "RB",
        Button::Back => "SELECT",
        Button::Start => "START",
        Button::DPadUp => "UP",
        Button::DPadDown => "DOWN",
        Button::DPadLeft => "LEFT",
        Button::DPadRight => "RIGHT",
        _ => "?",
    }
}

/// A key as written inside a prompt's brackets ("esc", "enter", "a").
fn prompt_key(name: &str) -> Option<Keycode> {
    match name {
        "esc" => Some(Keycode::Escape),
        "enter" => Some(Keycode::Return),
        "space" => Some(Keycode::Space),
        "shift" => Some(Keycode::LShift),
        "1" | "2" | "3" => None, // number picks: the d-pad covers them
        n => Keycode::from_name(&n.to_uppercase()),
    }
}

/// Keys that mean the same thing on menus (either one works).
fn same(a: Keycode, b: Keycode) -> bool {
    use Keycode as K;
    let norm = |k| match k {
        K::A => K::Left,
        K::D => K::Right,
        K::W => K::Up,
        K::S => K::Down,
        K::Space => K::Return,
        k => k,
    };
    a == b || norm(a) == norm(b)
}

/// The button that does what `key` does on this screen.
fn button_for(key: Keycode, mode: Mode, s: &Settings) -> Option<Button> {
    // Exact first (so [s] = save beats [s] = down), then the menu aliases.
    let exact = ALL_BUTTONS
        .iter()
        .copied()
        .find(|&b| key_for(b, mode, s) == Some(key));
    exact.or_else(|| {
        let menu = !matches!(mode, Mode::Playing | Mode::Reveal | Mode::Journal);
        ALL_BUTTONS
            .iter()
            .copied()
            .find(|&b| menu && key_for(b, mode, s).is_some_and(|k| same(k, key)))
    })
}

/// One bracketed token ("a/d", "esc") as controller buttons, if all map.
fn token(tok: &str, mode: Mode, s: &Settings) -> Option<String> {
    let buttons: Option<Vec<Button>> = tok
        .split('/')
        .map(|part| prompt_key(part.trim()).and_then(|k| button_for(k, mode, s)))
        .collect();
    let buttons = buttons?;
    let horizontal = [Button::DPadLeft, Button::DPadRight];
    let vertical = [Button::DPadUp, Button::DPadDown];
    Some(if buttons == horizontal || buttons == vertical {
        "D-PAD".into()
    } else if buttons == [Button::LeftShoulder, Button::RightShoulder] {
        "LB/RB".into()
    } else {
        buttons
            .iter()
            .map(|&b| button_label(b))
            .collect::<Vec<_>>()
            .join("/")
    })
}

/// Rewrites every `[key]` in a prompt for controller players. Tokens with no
/// button on this screen are left as they are.
pub fn prompt(text: &str, mode: Mode, s: &Settings, pad: bool) -> String {
    if !pad {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']').map(|c| open + c) else {
            break;
        };
        out.push_str(&rest[..open]);
        let tok = &rest[open + 1..close];
        match token(tok, mode, s) {
            Some(b) => out.push_str(&format!("[{b}]")),
            None => out.push_str(&rest[open..=close]),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// The controls legend for the pause screen and the start of a dream.
pub fn controls(pad: bool, first_person: bool) -> Vec<(&'static str, &'static str)> {
    let mut v = if pad {
        vec![("L-STICK", "move"), ("A", "jump")]
    } else {
        vec![("WASD", "move"), ("SPACE", "jump")]
    };
    if first_person {
        v.push(if pad {
            ("R-STICK", "look")
        } else {
            ("MOUSE / ARROWS", "look")
        });
    }
    v.extend(if pad {
        [
            ("LB/RB", "ability 1"),
            ("X/Y", "ability 2"),
            ("START", "pause"),
        ]
    } else {
        [("SHIFT", "ability 1"), ("E", "ability 2"), ("ESC", "pause")]
    });
    v.push(("", "touch shards to wake · the portal goes deeper"));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_MODES: [Mode; 11] = [
        Mode::Title,
        Mode::Playing,
        Mode::Paused,
        Mode::Journal,
        Mode::Reveal,
        Mode::Store,
        Mode::Booklet,
        Mode::Choice,
        Mode::Summary,
        Mode::Settings,
        Mode::Codex,
    ];

    /// Every prompt the game shows, with the screen it's shown on.
    const PROMPTS: &[(Mode, &str)] = &[
        (Mode::Paused, "[esc] keep dreaming"),
        (Mode::Paused, "[q] wake up for real"),
        (Mode::Paused, "[b] dream booklet"),
        (Mode::Paused, "[l] lucid store   [o] settings   [x] codex"),
        (
            Mode::Choice,
            "[a/d] choose   [enter] take it   [r] reroll (1 left)   [x] skip (+5 dust)",
        ),
        (Mode::Choice, "[a/d] choose   [enter] decide"),
        (Mode::Reveal, "remembering...    [space] skip"),
        (
            Mode::Reveal,
            "[b] booklet   [l] lucid store   [r] dream again   [esc] wake",
        ),
        (
            Mode::Reveal,
            "[s] press into booklet   [r] dream again   [esc] wake",
        ),
        (
            Mode::Booklet,
            "[a/d] turn page      [p] save these cards as images      [esc] back",
        ),
        (Mode::Codex, "[esc] back"),
        (
            Mode::Settings,
            "[w/s] choose   [a/d] change   [enter] rebind / reset   [esc] back",
        ),
        (
            Mode::Store,
            "[w/s] shelf   [a/d] item   [enter] buy / equip   [esc] back",
        ),
        (Mode::Summary, "[enter] see what you remember"),
        (Mode::Playing, "[shift]"),
        (Mode::Playing, "[e]"),
    ];

    #[test]
    fn every_prompt_has_a_button_for_every_key() {
        let s = Settings::default();
        for &(mode, text) in PROMPTS {
            let pad = prompt(text, mode, &s, true);
            // No keyboard token survives: each bracket names a button now.
            let mut rest = pad.as_str();
            while let Some(o) = rest.find('[') {
                let c = o + rest[o..].find(']').unwrap();
                let tok = &rest[o + 1..c];
                assert!(
                    tok.split('/').all(|b| {
                        ALL_BUTTONS.iter().any(|&x| button_label(x) == b)
                            || ["D-PAD", "LB", "RB"].contains(&b)
                    }),
                    "{mode:?}: '{tok}' has no button in {pad:?}"
                );
                rest = &rest[c + 1..];
            }
        }
    }

    #[test]
    fn prompts_name_the_button_that_does_it() {
        let s = Settings::default();
        let p = |m, t| prompt(t, m, &s, true);
        assert_eq!(p(Mode::Choice, "[r] reroll"), "[X] reroll");
        assert_eq!(p(Mode::Choice, "[a/d] choose"), "[D-PAD] choose");
        assert_eq!(p(Mode::Reveal, "[s] press"), "[X] press");
        assert_eq!(p(Mode::Paused, "[q] wake up"), "[Y] wake up");
        assert_eq!(p(Mode::Playing, "[shift]"), "[LB]");
        assert_eq!(prompt("[r] reroll", Mode::Choice, &s, false), "[r] reroll");
    }

    #[test]
    fn b_never_quits_from_the_title() {
        let s = Settings::default();
        for b in ALL_BUTTONS {
            let k = key_for(b, Mode::Title, &s);
            assert!(
                !matches!(k, Some(Keycode::Escape | Keycode::Q)),
                "{b:?} quits"
            );
        }
    }

    #[test]
    fn in_a_dream_the_face_buttons_do_what_the_legend_says() {
        let s = Settings::default();
        assert_eq!(
            key_for(Button::A, Mode::Playing, &s),
            s.key_for(Action::Jump)
        );
        assert_eq!(
            key_for(Button::LeftShoulder, Mode::Playing, &s),
            s.key_for(Action::Ability1)
        );
        assert_eq!(
            key_for(Button::X, Mode::Playing, &s),
            s.key_for(Action::Ability2)
        );
        assert_eq!(
            key_for(Button::Start, Mode::Playing, &s),
            Some(Keycode::Escape)
        );
        // Every screen can be left or confirmed without a keyboard.
        for m in ALL_MODES {
            assert!(
                ALL_BUTTONS.iter().any(|&b| key_for(b, m, &s).is_some()),
                "{m:?}"
            );
        }
    }

    #[test]
    fn the_legend_covers_moving_jumping_abilities_and_pausing() {
        for pad in [false, true] {
            for fp in [false, true] {
                let c = controls(pad, fp);
                let has = |w: &str| c.iter().any(|(_, what)| what.contains(w));
                assert!(has("move") && has("jump") && has("ability") && has("pause"));
                assert_eq!(has("look"), fp);
            }
        }
    }
}
