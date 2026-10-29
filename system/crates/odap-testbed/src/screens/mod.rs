//! The screens' logic, one module per screen: each wires its own callbacks, models and timers,
//! registers the controls it owns with the key router, and what it prepares as it opens.

use std::collections::HashMap;

use blackbird_device::Description;

use crate::{SCREENS, Testbed, keys::Router};

mod arena;
mod calibration;
mod exceptions;
mod gestures;
mod keyboard;
mod layers;
mod media;
mod navigation;
mod spaces;
mod transitions;
mod turns;
mod virtual_list;

/// What a screen prepares as it opens.
type Opener<U> = Box<dyn Fn(&U)>;

/// What a screen prepares when the list opens it, by the screen's name (screens.tsv).
pub(crate) struct Openers<U> {
    by_name: HashMap<&'static str, Opener<U>>,
}

impl<U> Default for Openers<U> {
    fn default() -> Self {
        Openers {
            by_name: HashMap::new(),
        }
    }
}

impl<U> Openers<U> {
    /// Registers what the screen `name` prepares as it opens.
    ///
    /// # Panics
    ///
    /// When `name` is not a screen of the list, or already has an opener.
    pub(crate) fn add(&mut self, name: &'static str, opener: impl Fn(&U) + 'static) {
        assert!(SCREENS.contains(&name), "{name} is not in screens.tsv");
        assert!(
            self.by_name.insert(name, Box::new(opener)).is_none(),
            "{name} has two openers"
        );
    }

    /// The list opened the screen `name`: what it prepares, if anything.
    pub(crate) fn open(&self, ui: &U, name: &str) {
        if let Some(opener) = self.by_name.get(name) {
            opener(ui);
        }
    }
}

/// Wires every screen that has logic of its own (the others are the interface alone).
pub(crate) fn wire(
    ui: &Testbed,
    desc: Option<Description>,
    keys: &mut Router<Testbed>,
    opened: &mut Openers<Testbed>,
) {
    calibration::wire(ui, desc);
    spaces::wire(ui, keys);
    turns::wire(ui, keys);
    arena::wire(ui, keys);
    virtual_list::wire(ui, opened);
    media::wire(ui, keys, opened);
    navigation::wire(ui, keys);
    keyboard::wire(ui);
    layers::wire(ui);
    transitions::wire(ui, keys);
    gestures::wire(ui);
    exceptions::wire(ui);
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[test]
    fn a_screen_opened_prepares_what_it_registered() {
        let mut o: Openers<RefCell<Vec<&str>>> = Openers::default();
        o.add("Virtual", |s| s.borrow_mut().push("virtual"));
        let seen = RefCell::new(Vec::new());
        o.open(&seen, "Virtual");
        o.open(&seen, "Calibration");
        assert_eq!(*seen.borrow(), ["virtual"]);
    }

    #[test]
    #[should_panic(expected = "not in screens.tsv")]
    fn an_opener_names_a_screen_of_the_list() {
        let mut o: Openers<()> = Openers::default();
        o.add("Nowhere", |_| {});
    }

    #[test]
    fn the_list_names_each_screen_once() {
        let mut names = SCREENS.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SCREENS.len());
    }
}
