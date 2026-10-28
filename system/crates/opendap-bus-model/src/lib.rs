//! One input model for the device and the host (RFC 0020): what a control means ([`Role`]), the
//! events a host sends a device ([`InputEvent`], `opendap.input/v1`), and the schema identifiers
//! both sides check. Plain data with serde; no platform, so both build it anywhere.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The schema of the input events a host sends a device.
pub const INPUT_SCHEMA: &str = "opendap.input/v1";
/// The schema of a device's description of its hardware (its controls among them).
pub const DEVICE_DESCRIPTION_SCHEMA: &str = "opendap.device-description/v1";

/// What a control means: never which key or button produces it. One closed vocabulary, versioned
/// with this crate (RFC 0020 §2): a device declares the roles it has and rejects others; a new
/// kind of control adds a role here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Action,
    VolumeUp,
    VolumeDown,
    Main,
    Back,
    Select,
    Menu,
}

impl Role {
    /// Every role, in the vocabulary's order.
    pub const ALL: [Role; 7] = [
        Role::Action,
        Role::VolumeUp,
        Role::VolumeDown,
        Role::Main,
        Role::Back,
        Role::Select,
        Role::Menu,
    ];

    /// Its name on the wire and in descriptions (kebab-case).
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Action => "action",
            Role::VolumeUp => "volume-up",
            Role::VolumeDown => "volume-down",
            Role::Main => "main",
            Role::Back => "back",
            Role::Select => "select",
            Role::Menu => "menu",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A name that is not a role: says which roles there are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRole(pub String);

impl fmt::Display for UnknownRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let known: Vec<&str> = Role::ALL.iter().map(|r| r.as_str()).collect();
        write!(
            f,
            "{:?} is not a role (roles: {})",
            self.0,
            known.join(", ")
        )
    }
}

impl std::error::Error for UnknownRole {}

impl FromStr for Role {
    type Err = UnknownRole;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Role::ALL
            .into_iter()
            .find(|r| r.as_str() == s)
            .ok_or_else(|| UnknownRole(s.to_string()))
    }
}

/// One input event (`opendap.input/v1`), tagged by `kind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum InputEvent {
    /// Press the control with `role` for `hold_ms` milliseconds.
    Key { role: Role, hold_ms: u64 },
    /// Put the control with `role` down and leave it there, as a finger resting on it: for
    /// surfaces that follow a person's hand (the simulator window), paired with [`Self::KeyUp`].
    #[serde(rename = "key-down")]
    KeyDown { role: Role },
    /// Release the control with `role`.
    #[serde(rename = "key-up")]
    KeyUp { role: Role },
    /// A key of a keyboard attached to the device (an external one, or the simulator's arrows),
    /// by its Linux name without `KEY_`, lower case (`left`, `enter`): not a control of the
    /// device, so it reaches the activity as a keyboard's key.
    Keyboard { key: String, down: bool },
    /// Touch the display at a pixel.
    Tap { x: u32, y: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_reads_back_from_its_name() {
        for r in Role::ALL {
            assert_eq!(r.as_str().parse::<Role>(), Ok(r));
            assert_eq!(
                serde_json::to_string(&r).unwrap(),
                format!("\"{}\"", r.as_str())
            );
            assert_eq!(
                serde_json::from_str::<Role>(&format!("\"{r}\"")).unwrap(),
                r
            );
        }
    }

    #[test]
    fn an_unknown_role_is_refused_with_the_known_ones() {
        let e = "volume_up".parse::<Role>().unwrap_err();
        assert!(e.to_string().contains("volume-up"), "{e}");
        assert!(serde_json::from_str::<Role>("\"shutter\"").is_err());
    }

    #[test]
    fn events_keep_the_wire_format_of_v1() {
        let cases = [
            (
                InputEvent::Key {
                    role: Role::Action,
                    hold_ms: 80,
                },
                r#"{"kind":"key","role":"action","hold_ms":80}"#,
            ),
            (
                InputEvent::KeyDown {
                    role: Role::VolumeUp,
                },
                r#"{"kind":"key-down","role":"volume-up"}"#,
            ),
            (
                InputEvent::KeyUp { role: Role::Main },
                r#"{"kind":"key-up","role":"main"}"#,
            ),
            (
                InputEvent::Keyboard {
                    key: "left".into(),
                    down: true,
                },
                r#"{"kind":"keyboard","key":"left","down":true}"#,
            ),
            (
                InputEvent::Tap { x: 3, y: 4 },
                r#"{"kind":"tap","x":3,"y":4}"#,
            ),
        ];
        for (event, json) in cases {
            assert_eq!(serde_json::to_string(&event).unwrap(), json);
            assert_eq!(serde_json::from_str::<InputEvent>(json).unwrap(), event);
        }
    }

    proptest::proptest! {
        #[test]
        fn any_name_is_a_role_or_refused(name in "[a-z-]{0,12}") {
            match name.parse::<Role>() {
                Ok(r) => proptest::prop_assert_eq!(r.as_str(), name.as_str()),
                Err(e) => proptest::prop_assert_eq!(e.0, name),
            }
        }
    }
}
