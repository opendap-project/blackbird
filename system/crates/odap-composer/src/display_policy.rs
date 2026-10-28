//! The display's decisions, apart from DRM and the renderer (linux/backend/), so they are tested on
//! any host: which mode to drive, the panel's physical size, and when the composer draws, keeps the
//! last frame, keeps drawing or wakes itself (spec §5: a frame only after a change, at most once
//! per refresh; never a periodic timer).

/// A mode the connector offers: its size and how the kernel marks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeOffer {
    pub size: (u16, u16),
    /// Set on the kernel command line (`video=`, DRM's USERDEF).
    pub user_defined: bool,
    /// The connector's preferred mode.
    pub preferred: bool,
}

/// The mode to drive, by its index: the panel's own size from the device description first, then a
/// mode set on the kernel command line, then the connector's preferred one, then its first.
pub fn choose_mode(offers: &[ModeOffer], native: Option<(u16, u16)>) -> Option<usize> {
    offers
        .iter()
        .position(|m| Some(m.size) == native)
        .or_else(|| offers.iter().position(|m| m.user_defined))
        .or_else(|| offers.iter().position(|m| m.preferred))
        .or(if offers.is_empty() { None } else { Some(0) })
}

/// The panel's physical size in mm: what the connector reports, else the device description's.
pub fn physical_size(reported: Option<(u32, u32)>, described: Option<(f32, f32)>) -> (i32, i32) {
    let reported = reported
        .map(|(w, h)| (w as i32, h as i32))
        .unwrap_or((0, 0));
    match (reported, described) {
        ((0, _) | (_, 0), Some((w, h))) => (w.round() as i32, h.round() as i32),
        (r, _) => r,
    }
}

/// What the frame loop knows when something asks it to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameState {
    pub display_off: bool,
    /// A frame is queued and the display has not shown it yet.
    pub frame_pending: bool,
    /// Something changed since the last frame.
    pub redraw: bool,
    pub session_active: bool,
}

/// Whether to draw now: the display is on, the session is ours, something changed and no frame is
/// waiting for the display (at most one per refresh).
pub fn may_draw(s: FrameState) -> bool {
    !s.display_off && !s.frame_pending && s.redraw && s.session_active
}

/// Whether to keep the last frame instead of drawing: no client on screen after boot (the product
/// host exited and is being restarted) and no power menu open, so the display never goes black
/// between hosts (spec §9).
pub fn keep_last_frame(client_on_screen: bool, boot_done: bool, power_open: bool) -> bool {
    !client_on_screen && boot_done && !power_open
}

/// Whether the next refresh draws again by itself: something animates (the splash screen fading,
/// the touch indicator, a system layer's transition).
pub fn keep_drawing(splash: bool, touches: bool, layers: bool) -> bool {
    splash || touches || layers
}

/// Whether to arm a one-shot wake: a deadline exists and no timer for it is armed yet.
pub fn arm_wake<T>(deadline: Option<T>, armed: bool) -> Option<T> {
    if armed { None } else { deadline }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(w: u16, h: u16, user_defined: bool, preferred: bool) -> ModeOffer {
        ModeOffer {
            size: (w, h),
            user_defined,
            preferred,
        }
    }

    #[test]
    fn the_panel_s_own_size_wins_then_the_command_line_then_the_preference() {
        let offers = [
            offer(1024, 768, false, true),
            offer(800, 600, true, false),
            offer(320, 480, false, false),
        ];
        assert_eq!(choose_mode(&offers, Some((320, 480))), Some(2));
        assert_eq!(choose_mode(&offers, Some((1, 1))), Some(1));
        assert_eq!(choose_mode(&offers[..1], None), Some(0));
        assert_eq!(choose_mode(&[offer(640, 480, false, false)], None), Some(0));
        assert_eq!(choose_mode(&[], Some((320, 480))), None);
    }

    #[test]
    fn the_physical_size_comes_from_the_description_when_the_connector_has_none() {
        assert_eq!(
            physical_size(Some((49, 73)), Some((48.96, 73.44))),
            (49, 73)
        );
        assert_eq!(physical_size(Some((0, 0)), Some((48.96, 73.44))), (49, 73));
        assert_eq!(physical_size(None, Some((48.96, 73.44))), (49, 73));
        assert_eq!(physical_size(None, None), (0, 0));
    }

    #[test]
    fn a_frame_is_drawn_only_when_it_can_be_shown() {
        let ready = FrameState {
            redraw: true,
            session_active: true,
            ..Default::default()
        };
        assert!(may_draw(ready));
        assert!(!may_draw(FrameState {
            display_off: true,
            ..ready
        }));
        assert!(!may_draw(FrameState {
            frame_pending: true,
            ..ready
        }));
        assert!(!may_draw(FrameState {
            redraw: false,
            ..ready
        }));
        assert!(!may_draw(FrameState {
            session_active: false,
            ..ready
        }));
    }

    #[test]
    fn between_hosts_the_last_frame_stays_unless_the_power_menu_shows() {
        assert!(keep_last_frame(false, true, false));
        assert!(!keep_last_frame(false, true, true));
        assert!(
            !keep_last_frame(false, false, false),
            "the splash screen draws while booting"
        );
        assert!(!keep_last_frame(true, true, false));
    }

    #[test]
    fn wakes_are_one_shot_and_only_for_animations_or_deadlines() {
        assert!(!keep_drawing(false, false, false));
        assert!(keep_drawing(false, true, false));
        assert_eq!(arm_wake(Some(5), false), Some(5));
        assert_eq!(arm_wake(Some(5), true), None);
        assert_eq!(arm_wake::<u32>(None, false), None);
    }
}
