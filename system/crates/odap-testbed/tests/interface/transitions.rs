//! The Transitions screen: the pager under the buttons.

use super::*;

pub fn pager_buttons(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // tr-pager-buttons: locked, Volume + turns the pager forward, Volume − back.
    open(&ui, "Transitions");
    key(&ui, Key::Tab);
    key(&ui, Key::Return);
    let p0 = ui.get_tr_page();
    back_key(&ui);
    tick(600);
    let p1 = ui.get_tr_page();
    run.check(
        "pager: locked, Volume + turns forward",
        p1 == p0 + 1,
        || format!("page {p0} → {p1}"),
    );
    key(&ui, Key::Tab);
    tick(600);
    let p2 = ui.get_tr_page();
    run.check("pager: locked, Volume − turns back", p2 == p0, || {
        format!("page {p1} → {p2}")
    });
}
