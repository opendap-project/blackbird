//! The Keyboard screen: the keyboard's level, the form's binding, filter and issues.

use super::*;

pub fn opens_and_closes(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    let height = 480.0;
    // kb-layout, kb-dismiss: Action on a field opens the keyboard and its editing level; back
    // closes it and leaves the focus on the field.
    open(&ui, "Keyboard");
    key(&ui, Key::Tab);
    let field = focus(&ui);
    key(&ui, Key::Return);
    run.check(
        "keyboard opens",
        ui.get_kb_open() && ui.get_kb_field() == 0,
        || format!("open {} field {}", ui.get_kb_open(), ui.get_kb_field()),
    );
    run.check(
        "keyboard keeps the field in view",
        {
            let (y, h) = focus(&ui);
            y >= 0.0 && y + h <= height
        },
        || format!("{:?}", focus(&ui)),
    );
    key(&ui, Key::Escape);
    run.check("back closes the keyboard", !ui.get_kb_open(), || {
        "still open".into()
    });
    run.check(
        "back stays on the screen",
        ui.get_screen() == "Keyboard",
        || ui.get_screen().to_string(),
    );
    run.check("the focus stays on the field", focus(&ui) == field, || {
        format!("{field:?} → {:?}", focus(&ui))
    });
}

pub fn binding_and_filter(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // field-binding, field-filter: typing reaches the model; a kind refuses what it cannot hold.
    key(&ui, Key::Return);
    typed(&ui, "Studio");
    let name = field_text(&ui, "Profile name");
    run.check("typing shows", name.as_deref() == Some("Studio"), || {
        format!("{name:?}")
    });
    let model = ui.get_kb_fields().row_data(0).map(|f| f.text.to_string());
    run.check(
        "typing reaches the model",
        model.as_deref() == Some("Studio"),
        || format!("{model:?}"),
    );
    key(&ui, Key::Escape);
    key(&ui, Key::Tab);
    key(&ui, Key::Return);
    let before = field_text(&ui, "Preamp (dB)");
    typed(&ui, "x");
    let after = field_text(&ui, "Preamp (dB)");
    run.check(
        "a letter in a gain is refused",
        before.is_some() && before == after,
        || format!("{before:?} → {after:?}"),
    );
    key(&ui, Key::Escape);
}

pub fn next_field_and_issues(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // kb-next: with the keyboard open, Volume goes to the next field and the keyboard stays, editing
    // it.
    open(&ui, "Keyboard");
    key(&ui, Key::Tab);
    key(&ui, Key::Return);
    let first = ui.get_kb_field();
    key(&ui, Key::Tab);
    run.check("kb-next: the keyboard stays open", ui.get_kb_open(), || {
        "closed".into()
    });
    run.check(
        "kb-next: it edits the next field",
        ui.get_kb_field() == first + 1,
        || format!("{first} → {}", ui.get_kb_field()),
    );
    // field-shift: an issue appearing above the edited field (the previous one, left with a bad
    // value) moves it down; it is revealed again, whole.
    key(&ui, Key::Escape);
    open(&ui, "Keyboard");
    key(&ui, Key::Tab);
    key(&ui, Key::Tab); // Preamp
    key(&ui, Key::Return);
    typed(&ui, "99");
    key(&ui, Key::Tab); // leaves it: its issue (above the range) shows above the next field
    tick(100);
    let (y, h) = focus(&ui);
    let issue = ui
        .get_kb_fields()
        .row_data(1)
        .map(|f| f.issue.to_string())
        .unwrap_or_default();
    run.check(
        "field-shift: the left field shows its issue",
        !issue.is_empty(),
        || "no issue".into(),
    );
    run.check(
        "field-shift: the edited field is revealed whole",
        y >= CONTENT_TOP && y + h <= 480.0,
        || format!("{y} + {h}"),
    );
    // field-a11y: a screen reader reads the issue after the field's name: it is the field's
    // description (aria-describedby), on the element that holds its text, and only while it stands.
    let described = |ui: &Testbed, label: &str| {
        ElementHandle::find_by_accessible_label(ui, label)
            .find(|e| {
                e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::TextInput)
            })
            .and_then(|e| e.accessible_description())
            .map(|d| d.to_string())
            .unwrap_or_default()
    };
    let told = described(&ui, "Preamp (dB)");
    run.check(
        "field-a11y: the issue follows the field's name",
        !issue.is_empty() && told == issue,
        || format!("{told:?} for {issue:?}"),
    );
    run.check(
        "field-a11y: a field without an issue describes nothing",
        described(&ui, "Profile name").is_empty(),
        || described(&ui, "Profile name"),
    );
    key(&ui, Key::Escape);
}

pub fn digits_variant(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // kb-variant: a digits field asks for the digit keyboard.
    open(&ui, "Keyboard");
    for _ in 0..5 {
        key(&ui, Key::Tab);
    }
    key(&ui, Key::Return);
    let digits = ui.get_kb_digits();
    let at = ui.get_kb_field();
    run.check("a code asks for digits", at == 4 && digits, || {
        format!("field {at}, digits {digits}")
    });
    key(&ui, Key::Escape);
}
