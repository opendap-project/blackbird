//! The Keyboard screen's form (blackbird-ui-forms): fields of several kinds, their filter before
//! anything is typed, their values bound without echo, issues once a field is left; and the
//! keyboard experiment's keys.

use std::{cell::RefCell, rc::Rc};

use blackbird_ui_forms::{
    form::Form,
    kind::{Issue, Kind, Value},
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{FieldView, Testbed};

pub(super) fn wire(ui: &Testbed) {
    let form = Rc::new(RefCell::new(form()));
    {
        let f = form.borrow();
        ui.set_kb_fields(ModelRc::new(VecModel::from(
            (0..f.fields.len())
                .map(|i| field_view(&f, i))
                .collect::<Vec<_>>(),
        )));
    }
    let (weak, form_e) = (ui.as_weak(), form.clone());
    ui.on_field_edited(move |i, text| {
        let Some(ui) = weak.upgrade() else {
            return text;
        };
        let mut f = form_e.borrow_mut();
        let i = i as usize;
        let name = f.fields[i].name.clone();
        // What this edit inserted (between what stayed before and after it): the kind's filter
        // refuses it before it reaches anything, the text goes back to what it was.
        let prev = f.fields[i].binding.draft().to_string();
        let inserted = inserted_text(&prev, &text);
        if !f.fields[i].binding.kind.allows(&inserted) {
            return prev.into();
        }
        f.input(&name, &text);
        let view = field_view(&f, i);
        ui.get_kb_fields().set_row_data(
            i,
            FieldView {
                text: text.clone(),
                ..view
            },
        );
        text
    });
    let (weak, form_l) = (ui.as_weak(), form.clone());
    ui.on_field_left(move |i| {
        let Some(ui) = weak.upgrade() else { return };
        let mut f = form_l.borrow_mut();
        let name = f.fields[i as usize].name.clone();
        f.blur(&name);
        ui.get_kb_fields()
            .set_row_data(i as usize, field_view(&f, i as usize));
    });

    // The keyboard experiment's keys reach the focused field as key events, as a virtual
    // keyboard's do (virtual-keyboard-v1): the field keeps the focus.
    let weak = ui.as_weak();
    ui.on_kb_key(move |k| {
        let Some(ui) = weak.upgrade() else { return };
        let text: SharedString = if k.as_str() == "⌫" {
            slint::platform::Key::Backspace.into()
        } else {
            k
        };
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
    });
}

/// The form's fields.
fn form() -> Form {
    Form::new()
        .field(
            "Profile name",
            Kind::Text {
                min: 1,
                max: Some(24),
            },
            Value::Empty,
            true,
        )
        .field(
            "Preamp (dB)",
            Kind::Gain {
                min: -12.0,
                max: 0.0,
                step: 0.5,
            },
            Value::Number(0.0),
            false,
        )
        .field(
            "Crossfeed (Hz)",
            Kind::Frequency {
                min: 300.0,
                max: 1200.0,
            },
            Value::Number(700.0),
            false,
        )
        .field("Email", Kind::Email, Value::Empty, false)
        .field("Pairing code", Kind::Pin { len: 6 }, Value::Empty, false)
        .field("Released", Kind::Date, Value::Empty, false)
        .field(
            "Sleep timer",
            Kind::Duration,
            Value::Duration(30 * 60_000),
            false,
        )
        .field("Tags", Kind::Tags { max: Some(5) }, Value::Empty, false)
}

/// Field `i` as the interface shows it.
fn field_view(form: &Form, i: usize) -> FieldView {
    let f = &form.fields[i];
    FieldView {
        label: f.name.clone().into(),
        text: f.binding.draft().into(),
        issue: form
            .shown(&f.name)
            .first()
            .map_or(String::new(), |iss| issue_words(&f.binding.kind, iss))
            .into(),
        purpose: format!("{:?}", f.binding.kind.purpose())
            .to_lowercase()
            .into(),
    }
}

/// An issue in words, for the field it is about (English here; the interface's language on the
/// device, from its catalogue).
fn issue_words(kind: &Kind, issue: &Issue) -> String {
    match issue {
        Issue::Required => "Required".into(),
        Issue::Malformed => match kind {
            Kind::Email => "Not an email address".into(),
            Kind::Date => "A date as YYYY-MM-DD".into(),
            Kind::Pin { len } => format!("{len} digits"),
            Kind::Duration => "A time as m:ss or h:mm:ss".into(),
            _ => "Not understood".into(),
        },
        Issue::BelowMin(m) => format!("At least {m}"),
        Issue::AboveMax(m) => format!("At most {m}"),
        Issue::OffStep(s) => format!("In steps of {s}"),
        Issue::TooShort(n) => format!("At least {n}"),
        Issue::TooLong(n) => format!("At most {n}"),
        Issue::Custom(k) => k.clone(),
    }
}

/// What an edit put in, between what stayed before it and after it ("" for a delete).
fn inserted_text(before: &str, after: &str) -> String {
    let (b, a): (Vec<char>, Vec<char>) = (before.chars().collect(), after.chars().collect());
    let start = b.iter().zip(&a).take_while(|(x, y)| x == y).count();
    let room = b.len().min(a.len()) - start;
    let end = b
        .iter()
        .rev()
        .zip(a.iter().rev())
        .take(room)
        .take_while(|(x, y)| x == y)
        .count();
    a[start..a.len() - end].iter().collect()
}
