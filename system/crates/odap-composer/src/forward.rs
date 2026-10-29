//! What a control press means for the activity and which keys it sends, apart from how they are
//! sent (linux/input/forward.rs), so the choices are tested on any host.
//!
//! A role the system combinations left to the activity (T1 controls spec §1.3): Volume ± is the
//! volume while sound plays or the activity asks for it, and otherwise moves the focus, Volume − to
//! the next element and Volume + to the previous (Tab, Shift+Tab). Action takes the focused element
//! (Enter), or the screen's primary action where it declares one (Blackbird UI's components).
//! With the display off nothing reaches the focus: Action is play/pause and Volume the volume
//! (§1.3, T1-060). Main stays the system's. Other hardware's controls map as input events §3.4
//! says: a select button is Enter, a back button Escape.

use blackbird_device::Role;
use blackbird_ui_input::double::{DoublePress, Down};
use blackbird_ui_input::semantic::{self, Context, Semantic};

// Linux key codes (linux/input-event-codes.h) the activity receives.
pub const KEY_ESC: u32 = 1;
pub const KEY_TAB: u32 = 15;
pub const KEY_ENTER: u32 = 28;
pub const KEY_LEFTSHIFT: u32 = 42;
pub const KEY_NEXTSONG: u32 = 163;
pub const KEY_PREVIOUSSONG: u32 = 165;

/// What a press does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press {
    /// What it means here (input events §3.4).
    pub meaning: Semantic,
    /// The keys it holds down until the role is released, in order.
    pub keys: &'static [u32],
    /// Whether it repeats while held (the focus, the volume; never Action).
    pub repeats: bool,
    /// A second press of a volume button within the double-press time: the first one's step is
    /// taken back and this track key is sent and released at once (T1-062, T1-063). The press is
    /// spent: it holds nothing and does not repeat.
    pub track: Option<u32>,
}

/// Which of the two volume buttons' double-press states a role uses.
fn volume_slot(role: Role) -> Option<usize> {
    match role {
        Role::VolumeUp => Some(0),
        Role::VolumeDown => Some(1),
        _ => None,
    }
}

/// A press of `role`: `None` when it is nothing the activity gets (Main, a role with no meaning).
/// `volume` keeps the volume buttons' double presses; `now_ms` is a monotonic clock.
pub fn press(
    role: Role,
    display_on: bool,
    sound_playing: bool,
    volume: &mut [DoublePress; 2],
    now_ms: u64,
) -> Option<Press> {
    // The composer does not see the activity's focus or locks: the components refine Activate into
    // the primary action and Next/Previous into a value's steps (Blackbird UI).
    let ctx = Context {
        display_on,
        volume_wanted: display_on && sound_playing,
        ..Default::default()
    };
    let meaning = semantic::press(role, ctx);
    let keys: &'static [u32] = match meaning {
        Semantic::System | Semantic::None => return None,
        Semantic::Next => &[KEY_TAB],
        Semantic::Previous => &[KEY_LEFTSHIFT, KEY_TAB],
        Semantic::Activate | Semantic::Primary => &[KEY_ENTER],
        Semantic::Back => &[KEY_ESC],
        _ => &[],
    };
    let mut track = None;
    if let (Semantic::VolumeUp | Semantic::VolumeDown, Some(slot)) = (meaning, volume_slot(role))
        && volume[slot].down(now_ms) == Down::Second
    {
        track = Some(match semantic::double_press(role, ctx) {
            Semantic::NextTrack => KEY_NEXTSONG,
            _ => KEY_PREVIOUSSONG,
        });
    }
    Some(Press {
        meaning,
        keys: if track.is_some() { &[] } else { keys },
        repeats: track.is_none() && semantic::repeats(role, ctx),
        track,
    })
}

/// A role released: a volume button's double-press state learns of it.
pub fn release(role: Role, volume: &mut [DoublePress; 2], now_ms: u64) {
    if let Some(slot) = volume_slot(role) {
        volume[slot].up(now_ms);
    }
}

/// One more step of a held focus step, as the keys go (code, pressed): the press's whole chord
/// released, in reverse, and pressed again, as a person tapping Shift+Tab again would. No step
/// leans on a modifier the activity saw earlier: an activity that let Shift go meanwhile (a
/// screen's end stop does, Blackbird UI) would otherwise take Volume + for Volume −.
pub fn step_again(keys: &[u32]) -> impl Iterator<Item = (u32, bool)> + '_ {
    let up = keys.iter().rev().map(|&k| (k, false));
    up.chain(keys.iter().map(|&k| (k, true)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(role: Role, display_on: bool, sound: bool) -> Option<Press> {
        press(role, display_on, sound, &mut Default::default(), 0)
    }

    #[test]
    fn the_focus_moves_with_the_volume_buttons_and_repeats() {
        let next = at(Role::VolumeDown, true, false).unwrap();
        assert_eq!(
            (next.meaning, next.keys, next.repeats),
            (Semantic::Next, &[KEY_TAB][..], true)
        );
        let prev = at(Role::VolumeUp, true, false).unwrap();
        assert_eq!(prev.keys, &[KEY_LEFTSHIFT, KEY_TAB][..]);
    }

    #[test]
    fn a_held_step_again_is_the_whole_chord_again() {
        let prev = at(Role::VolumeUp, true, false).unwrap();
        let steps: Vec<_> = step_again(prev.keys).collect();
        assert_eq!(
            steps,
            [
                (KEY_TAB, false),
                (KEY_LEFTSHIFT, false),
                (KEY_LEFTSHIFT, true),
                (KEY_TAB, true)
            ]
        );
        let next = at(Role::VolumeDown, true, false).unwrap();
        let steps: Vec<_> = step_again(next.keys).collect();
        assert_eq!(steps, [(KEY_TAB, false), (KEY_TAB, true)]);
    }

    #[test]
    fn action_takes_the_focus_and_never_repeats() {
        let p = at(Role::Action, true, false).unwrap();
        assert_eq!(
            (p.meaning, p.keys, p.repeats),
            (Semantic::Activate, &[KEY_ENTER][..], false)
        );
    }

    #[test]
    fn with_the_display_off_action_is_play_pause_and_volume_the_volume() {
        let p = at(Role::Action, false, false).unwrap();
        assert_eq!((p.meaning, p.keys), (Semantic::PlayPause, &[][..]));
        let v = at(Role::VolumeUp, false, false).unwrap();
        assert_eq!(
            (v.meaning, v.keys, v.repeats),
            (Semantic::VolumeUp, &[][..], true)
        );
    }

    #[test]
    fn while_sound_plays_the_volume_buttons_are_the_volume() {
        let v = at(Role::VolumeDown, true, true).unwrap();
        assert_eq!(
            (v.meaning, v.repeats, v.track),
            (Semantic::VolumeDown, true, None)
        );
    }

    #[test]
    fn a_second_volume_press_changes_the_track_and_is_spent() {
        let mut volume: [DoublePress; 2] = Default::default();
        let first = press(Role::VolumeUp, true, true, &mut volume, 0).unwrap();
        assert_eq!(first.track, None);
        release(Role::VolumeUp, &mut volume, 60);
        let second = press(Role::VolumeUp, true, true, &mut volume, 150).unwrap();
        assert_eq!(
            (second.track, second.keys, second.repeats),
            (Some(KEY_NEXTSONG), &[][..], false)
        );
        let mut volume: [DoublePress; 2] = Default::default();
        press(Role::VolumeDown, true, true, &mut volume, 0);
        release(Role::VolumeDown, &mut volume, 60);
        assert_eq!(
            press(Role::VolumeDown, true, true, &mut volume, 150)
                .unwrap()
                .track,
            Some(KEY_PREVIOUSSONG)
        );
    }

    #[test]
    fn main_and_a_role_without_meaning_give_the_activity_nothing() {
        assert_eq!(at(Role::Main, true, false), None);
        assert_eq!(at(Role::Menu, true, false), None);
    }

    #[test]
    fn other_hardware_s_select_and_back_are_enter_and_escape() {
        assert_eq!(
            at(Role::Select, true, false).unwrap().keys,
            &[KEY_ENTER][..]
        );
        let back = at(Role::Back, true, false).unwrap();
        assert_eq!((back.keys, back.repeats), (&[KEY_ESC][..], false));
    }
}
