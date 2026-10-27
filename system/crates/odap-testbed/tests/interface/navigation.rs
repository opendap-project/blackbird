//! The Navigation screen: its stops' ends.

use super::*;

pub fn stop_ends(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // Past the ends (opendap-input's ends): the days of the week go round, the shelf of albums stops.
    open(&ui, "Navigation");
    let days = |ui: &Testbed, k: &str| {
        ui.invoke_control_key("days".into(), k.into());
        tick(16);
        (
            ui.get_nav_days().focus.to_string(),
            ui.get_nav_text().to_string(),
        )
    };
    days(&ui, "enter");
    let (at, _) = days(&ui, "activate");
    run.check("days: Action enters on Monday", at == "Mon", || at.clone());
    let (at, said) = days(&ui, "prev");
    run.check(
        "days: before Monday, Sunday",
        at == "Sun" && said.starts_with("round to Sun"),
        || format!("{at}: {said}"),
    );
    let (at, said) = days(&ui, "next");
    run.check(
        "days: after Sunday, Monday",
        at == "Mon" && said.starts_with("round to Mon"),
        || format!("{at}: {said}"),
    );
    let (at, said) = days(&ui, "next");
    run.check(
        "days: a move inside is a move",
        at == "Tue" && said.starts_with("moved to Tue"),
        || format!("{at}: {said}"),
    );
    ui.invoke_control_key("shelf".into(), "enter".into());
    ui.invoke_control_key("shelf".into(), "activate".into());
    ui.invoke_control_key("shelf".into(), "prev".into());
    tick(16);
    let said = ui.get_nav_text().to_string();
    run.check(
        "shelf: a list stops; past its start, out",
        said.starts_with("released shelf"),
        || said.clone(),
    );
}
