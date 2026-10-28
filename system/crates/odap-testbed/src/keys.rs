//! How a control's keys reach the screen that owns it, apart from any screen, so the routing and
//! the shared mapping are tested on any host.
//!
//! Every control under the buttons (Slint's `KeyControl`) reports its keys by its name: `next`,
//! `prev`, `inc`, `dec`, `activate`, `escape`, a keyboard's `left` and `right`, and the focus's
//! own (`enter-down`, `enter-up`, `reveal`, `enter`, `release`, `touch:<part>`). Each screen
//! registers the names it owns with a handler; the router hands each key to the one that owns its
//! control, and a name nobody owns lets the focus move on.

use std::collections::HashMap;

/// How a control takes a keyboard's arrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrows {
    /// As they come: the handler reads `left` and `right` itself (or ignores them).
    Raw,
    /// As more and less (`inc`, `dec`) by reading order: see [`semantic`].
    Values,
}

/// A keyboard's ← and → as more and less: more toward the end of the reading order, so mirrored
/// right to left. Volume and ↑ ↓ (`inc`, `dec`) are more and less by their place, never mirrored;
/// every other key goes as it came.
pub fn semantic(key: &str, rtl: bool) -> &str {
    match (key, rtl) {
        ("right", false) | ("left", true) => "inc",
        ("left", false) | ("right", true) => "dec",
        _ => key,
    }
}

/// A screen's handler: the window, the control's name and the key; whether it took the key.
type Handler<U> = Box<dyn Fn(&U, &str, &str) -> bool>;

/// The controls' owners, by name.
pub struct Router<U> {
    owners: HashMap<&'static str, usize>,
    handlers: Vec<(Arrows, Handler<U>)>,
}

impl<U> Default for Router<U> {
    fn default() -> Self {
        Router {
            owners: HashMap::new(),
            handlers: Vec::new(),
        }
    }
}

impl<U> Router<U> {
    /// Registers `handler` as the owner of the controls called `names`.
    ///
    /// # Panics
    ///
    /// When a name already has an owner: two screens never share a control's name.
    pub fn add(
        &mut self,
        names: &[&'static str],
        arrows: Arrows,
        handler: impl Fn(&U, &str, &str) -> bool + 'static,
    ) {
        let at = self.handlers.len();
        self.handlers.push((arrows, Box::new(handler)));
        for name in names {
            assert!(
                self.owners.insert(name, at).is_none(),
                "control {name} has two owners"
            );
        }
    }

    /// Hands `key` on the control `name` to its owner, mapped as it asked; false when nobody owns
    /// the control or its owner did not take the key.
    pub fn route(&self, ui: &U, name: &str, key: &str, rtl: bool) -> bool {
        let Some(&at) = self.owners.get(name) else {
            return false;
        };
        let (arrows, handler) = &self.handlers[at];
        let key = match arrows {
            Arrows::Raw => key,
            Arrows::Values => semantic(key, rtl),
        };
        handler(ui, name, key)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[test]
    fn arrows_are_more_and_less_by_reading_order() {
        assert_eq!(semantic("right", false), "inc");
        assert_eq!(semantic("left", false), "dec");
        assert_eq!(semantic("right", true), "dec");
        assert_eq!(semantic("left", true), "inc");
    }

    #[test]
    fn volume_and_the_rest_are_never_mirrored() {
        for rtl in [false, true] {
            for key in [
                "inc", "dec", "next", "prev", "activate", "escape", "touch:a3",
            ] {
                assert_eq!(semantic(key, rtl), key);
            }
        }
    }

    #[test]
    fn a_key_reaches_its_owner_mapped_as_it_asked() {
        let seen = RefCell::new(Vec::new());
        let mut r: Router<RefCell<Vec<String>>> = Router::default();
        r.add(&["dial", "strip"], Arrows::Values, |s, n, k| {
            s.borrow_mut().push(format!("{n}:{k}"));
            true
        });
        r.add(&["shelf"], Arrows::Raw, |s, n, k| {
            s.borrow_mut().push(format!("{n}:{k}"));
            k != "escape"
        });
        assert!(r.route(&seen, "dial", "right", true));
        assert!(r.route(&seen, "strip", "left", false));
        assert!(r.route(&seen, "shelf", "right", true));
        assert!(!r.route(&seen, "shelf", "escape", false));
        assert_eq!(
            *seen.borrow(),
            ["dial:dec", "strip:dec", "shelf:right", "shelf:escape"]
        );
    }

    #[test]
    fn a_control_nobody_owns_lets_the_focus_move_on() {
        let r: Router<()> = Router::default();
        assert!(!r.route(&(), "pager", "inc", false));
    }

    #[test]
    #[should_panic(expected = "two owners")]
    fn a_name_has_one_owner() {
        let mut r: Router<()> = Router::default();
        r.add(&["dial"], Arrows::Values, |_, _, _| true);
        r.add(&["dial"], Arrows::Raw, |_, _, _| true);
    }
}
