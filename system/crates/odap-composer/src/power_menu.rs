//! The power menu's focus and choices (T1 controls spec §1.3, spec §7), apart from the layer that
//! draws it (linux/layers.rs, ui/layers.slint), so they are tested on any host. An OS Shell draft
//! hosted by the composer (RFC 0012).
//!
//! Its choices, in order: Power off, Restart, Restart to recovery (only where the device has it),
//! Cancel. The volume buttons move the focus through them, wrapping around; Action takes the
//! focused one as a touch on it would, except Power off, which only `main` confirms.

/// What the menu asks the composer for (linux/controls.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Restart,
    RestartToRecovery,
    Cancel,
}

/// What Action does on the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// Power off is the menu's primary action, which only `main` confirms: the menu points at it.
    PointAtMain,
    Request(Request),
}

/// The number of choices: with or without Restart to recovery.
pub fn choices(can_recovery: bool) -> i32 {
    if can_recovery { 4 } else { 3 }
}

/// The focus after Volume + (`step` -1, up) or − (`step` 1, down) among `n` choices, wrapping
/// around; from no focus (-1), down lands on the first and up on the last.
pub fn move_focus(focus: i32, step: i32, n: i32) -> i32 {
    if focus < 0 {
        if step > 0 { 0 } else { n - 1 }
    } else {
        (focus + step).rem_euclid(n)
    }
}

/// What Action does with `focus` among `n` choices.
pub fn activate(focus: i32, n: i32) -> Option<Activation> {
    match focus {
        // No focus: the menu's primary action, Power off.
        0 | -1 => Some(Activation::PointAtMain),
        1 => Some(Activation::Request(Request::Restart)),
        2 if n == 4 => Some(Activation::Request(Request::RestartToRecovery)),
        f if f == n - 1 => Some(Activation::Request(Request::Cancel)),
        _ => None,
    }
}

/// A choice touched on the menu, by the name the layer gives it.
pub fn choice(name: &str) -> Request {
    match name {
        "restart" => Request::Restart,
        "recovery" => Request::RestartToRecovery,
        _ => Request::Cancel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_focus_wraps_around_the_choices() {
        let n = choices(false);
        assert_eq!(n, 3);
        assert_eq!(move_focus(0, 1, n), 1);
        assert_eq!(move_focus(2, 1, n), 0);
        assert_eq!(move_focus(0, -1, n), 2);
        assert_eq!(move_focus(-1, 1, n), 0);
        assert_eq!(move_focus(-1, -1, n), 2);
        assert_eq!(move_focus(3, 1, choices(true)), 0);
    }

    #[test]
    fn action_takes_the_focused_choice_except_power_off() {
        let (with, without) = (choices(true), choices(false));
        assert_eq!(activate(0, with), Some(Activation::PointAtMain));
        assert_eq!(activate(-1, without), Some(Activation::PointAtMain));
        assert_eq!(
            activate(1, with),
            Some(Activation::Request(Request::Restart))
        );
        assert_eq!(
            activate(2, with),
            Some(Activation::Request(Request::RestartToRecovery))
        );
        assert_eq!(
            activate(3, with),
            Some(Activation::Request(Request::Cancel))
        );
        assert_eq!(
            activate(2, without),
            Some(Activation::Request(Request::Cancel))
        );
        assert_eq!(activate(3, without), None);
    }

    #[test]
    fn a_touched_choice_is_its_request() {
        assert_eq!(choice("restart"), Request::Restart);
        assert_eq!(choice("recovery"), Request::RestartToRecovery);
        assert_eq!(choice("cancel"), Request::Cancel);
    }
}
