//! The system combinations on `main` (T1 controls and status LED spec §1.3), as decisions apart from
//! how they are carried out (linux/controls.rs), so the table is tested on any host. OS Shell
//! policy, drafted in the composer until the OS Shell exists (RFC 0012).
//!
//! | Press | Result |
//! |---|---|
//! | `main`, short (released before `LONG`) | The display off, or on again; playback goes on |
//! | `main`, long (held `LONG`) | The power menu |
//! | `main` + `volume-down`, short | A screenshot |
//! | `main` + `volume-down`, held `HOLD` | The power menu, also with the display off |
//!
//! With the power menu up, a press of `main` powers off: the confirmation is the same physical
//! gesture (spec §7), and Volume + and − move its focus and Action takes it (SYS-039). Other
//! controls pass through as roles.

use std::{collections::HashSet, time::Duration};

use opendap_device::Role;

/// Short against long (spec §1.1, Android's long-press timeout; a proposed value).
pub const LONG: Duration = Duration::from_millis(opendap_input::thresholds::LONG_MS);
/// Held, for deliberate system actions (spec §1.1; a proposed value).
pub const HOLD: Duration = Duration::from_millis(opendap_input::thresholds::HOLD_MS);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `main` alone.
    Main,
    /// `main` + `volume-down`.
    MainVolumeDown,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Main => "main",
            Kind::MainVolumeDown => "main + volume-down",
        }
    }

    /// How long it is held before its held action (the power menu).
    pub fn hold(self) -> Duration {
        match self {
            Kind::Main => LONG,
            Kind::MainVolumeDown => HOLD,
        }
    }
}

/// What a control change means for the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Not the system's: the activity gets it.
    Pass,
    /// The system's, with nothing more to do.
    Taken,
    /// A press starts: its held action is due after `kind.hold()`, if `id` is still the press.
    Started { id: u64, kind: Kind },
    /// The power menu's focus moves by this many items.
    MenuMove(i32),
    /// The power menu's focused item is taken.
    MenuActivate,
    /// A short press ended: what it does.
    Short(Short),
}

/// What a short press does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Short {
    /// `main` with the power menu up: the confirmation.
    PowerOff,
    /// `main`: the display off, or on again.
    Display { on: bool },
    /// `main` + `volume-down`.
    Screenshot,
}

/// The system press in progress.
#[derive(Debug)]
struct Press {
    id: u64,
    kind: Kind,
    /// Its held action has run; the release does nothing more.
    held: bool,
}

/// The controls down and the system press they make.
#[derive(Debug, Default)]
pub struct Combinations {
    down: HashSet<Role>,
    press: Option<Press>,
    presses: u64,
}

impl Combinations {
    /// A control changed, with the power menu up or not and the display on or not.
    pub fn control(
        &mut self,
        role: Role,
        pressed: bool,
        menu_open: bool,
        display_on: bool,
    ) -> Outcome {
        if pressed {
            self.down.insert(role);
        } else {
            self.down.remove(&role);
        }
        let main = self.down.contains(&Role::Main);
        match (pressed, role) {
            // `main` pressed, or `volume-down` joining it: a press starts, or becomes the combination.
            (true, Role::Main | Role::VolumeDown) if main => {
                let kind = if self.down.contains(&Role::VolumeDown) {
                    Kind::MainVolumeDown
                } else {
                    Kind::Main
                };
                if self.press.as_ref().is_some_and(|p| p.kind == kind) {
                    return Outcome::Taken;
                }
                self.presses += 1;
                let id = self.presses;
                self.press = Some(Press {
                    id,
                    kind,
                    held: false,
                });
                Outcome::Started { id, kind }
            }
            (true, Role::VolumeUp) if menu_open => Outcome::MenuMove(-1),
            (true, Role::VolumeDown) if menu_open => Outcome::MenuMove(1),
            (true, Role::Action) if menu_open => Outcome::MenuActivate,
            (false, Role::VolumeUp | Role::Action) if menu_open => Outcome::Taken,
            // A key of the press released: it ends.
            (false, Role::Main | Role::VolumeDown) if self.press.is_some() => {
                let press = self.press.take().expect("a press");
                if press.held {
                    return Outcome::Taken;
                }
                Outcome::Short(match press.kind {
                    Kind::Main if menu_open => Short::PowerOff,
                    Kind::Main => Short::Display { on: !display_on },
                    Kind::MainVolumeDown => Short::Screenshot,
                })
            }
            _ => Outcome::Pass,
        }
    }

    /// The hold time of press `id` ran out: its kind when it is still the press (its held action is
    /// due, and its release then does nothing more), else `None`.
    pub fn held(&mut self, id: u64) -> Option<Kind> {
        let press = self.press.as_mut().filter(|p| p.id == id)?;
        press.held = true;
        Some(press.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down(c: &mut Combinations, role: Role) -> Outcome {
        c.control(role, true, false, true)
    }

    fn up(c: &mut Combinations, role: Role) -> Outcome {
        c.control(role, false, false, true)
    }

    #[test]
    fn main_short_turns_the_display_off_or_on() {
        let mut c = Combinations::default();
        assert!(matches!(
            down(&mut c, Role::Main),
            Outcome::Started {
                kind: Kind::Main,
                ..
            }
        ));
        assert_eq!(
            up(&mut c, Role::Main),
            Outcome::Short(Short::Display { on: false })
        );
        c.control(Role::Main, true, false, false);
        assert_eq!(
            c.control(Role::Main, false, false, false),
            Outcome::Short(Short::Display { on: true })
        );
    }

    #[test]
    fn main_held_opens_the_menu_and_its_release_does_nothing_more() {
        let mut c = Combinations::default();
        let Outcome::Started { id, kind } = down(&mut c, Role::Main) else {
            panic!("no press");
        };
        assert_eq!((kind.name(), kind.hold()), ("main", LONG));
        assert_eq!(c.held(id), Some(Kind::Main));
        assert_eq!(c.control(Role::Main, false, true, true), Outcome::Taken);
    }

    #[test]
    fn main_with_the_menu_up_powers_off() {
        let mut c = Combinations::default();
        c.control(Role::Main, true, true, true);
        assert_eq!(
            c.control(Role::Main, false, true, true),
            Outcome::Short(Short::PowerOff)
        );
    }

    #[test]
    fn main_and_volume_down_short_is_a_screenshot_held_the_menu() {
        let mut c = Combinations::default();
        down(&mut c, Role::Main);
        let Outcome::Started { id, kind } = down(&mut c, Role::VolumeDown) else {
            panic!("no combination");
        };
        assert_eq!((kind, kind.hold()), (Kind::MainVolumeDown, HOLD));
        assert_eq!(
            up(&mut c, Role::VolumeDown),
            Outcome::Short(Short::Screenshot)
        );
        // `main`, still down, is no press any more: its release passes.
        assert_eq!(up(&mut c, Role::Main), Outcome::Pass);

        down(&mut c, Role::Main);
        let Outcome::Started { id: held, .. } = down(&mut c, Role::VolumeDown) else {
            panic!("no combination");
        };
        assert_ne!(held, id);
        assert_eq!(c.held(held), Some(Kind::MainVolumeDown));
        assert_eq!(up(&mut c, Role::Main), Outcome::Taken);
    }

    #[test]
    fn a_stale_timer_does_nothing() {
        let mut c = Combinations::default();
        let Outcome::Started { id, .. } = down(&mut c, Role::Main) else {
            panic!("no press");
        };
        up(&mut c, Role::Main);
        assert_eq!(c.held(id), None);
    }

    #[test]
    fn the_menu_takes_the_volume_buttons_and_action() {
        let mut c = Combinations::default();
        assert_eq!(
            c.control(Role::VolumeUp, true, true, true),
            Outcome::MenuMove(-1)
        );
        assert_eq!(c.control(Role::VolumeUp, false, true, true), Outcome::Taken);
        assert_eq!(
            c.control(Role::VolumeDown, true, true, true),
            Outcome::MenuMove(1)
        );
        assert_eq!(
            c.control(Role::Action, true, true, true),
            Outcome::MenuActivate
        );
        assert_eq!(c.control(Role::Action, false, true, true), Outcome::Taken);
    }

    #[test]
    fn without_main_or_the_menu_controls_pass() {
        let mut c = Combinations::default();
        for role in [Role::VolumeUp, Role::VolumeDown, Role::Action, Role::Back] {
            assert_eq!(down(&mut c, role), Outcome::Pass);
            assert_eq!(up(&mut c, role), Outcome::Pass);
        }
    }
}
