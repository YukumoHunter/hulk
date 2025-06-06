use std::{collections::HashMap, fmt};

use eframe::egui::{Key, KeyboardShortcut, Modifiers};
use serde::{
    de::{self, Deserializer},
    Deserialize,
};
use thiserror::Error;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Error)]
pub enum Error {
    #[error("duplicate modifier `{0}`")]
    DuplicateModifier(String),
    #[error("invalid modifier `{0}`")]
    InvalidModifier(String),
    #[error("invalid key `{0}`")]
    InvalidKey(String),
    #[error("unsupported keybind `{0}`")]
    UnsupportedKeybind(String),
}

// Make sure to update docs/tooling/twix.md when changing this!
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KeybindAction {
    CloseTab,
    DuplicateTab,
    FocusAbove,
    FocusAddress,
    FocusBelow,
    FocusLeft,
    FocusPanel,
    FocusRight,
    NoOp,
    OpenSplit,
    OpenTab,
    Reconnect,
    CloseAll,
}

pub fn parse_modifier(value: &str) -> Result<Modifiers, Error> {
    match value {
        "A" => Ok(Modifiers::ALT),
        "C" => Ok(Modifiers::COMMAND),
        "S" => Ok(Modifiers::SHIFT),
        _ => Err(Error::InvalidModifier(String::from(value))),
    }
}

fn is_supported_keybind(shortcut: &KeyboardShortcut) -> bool {
    match shortcut {
        // Binding CTRL+[cvx] is not supported.
        // See https://github.com/emilk/egui/issues/4065
        KeyboardShortcut {
            logical_key: Key::C | Key::V | Key::X,
            modifiers: Modifiers::COMMAND,
        } => false,
        _ => true,
    }
}

pub fn parse_shortcut(raw_shortcut: &str) -> Result<KeyboardShortcut, Error> {
    let parts = raw_shortcut.split('-').collect::<Vec<_>>();

    let Some((raw_key, raw_modifiers)) = parts.split_last() else {
        return Err(Error::InvalidKey(raw_shortcut.into()));
    };

    let is_single_ascii_uppercase_letter =
        matches!(raw_key.as_bytes(), [letter] if letter.is_ascii_uppercase());

    let Some(logical_key) = Key::from_name(raw_key) else {
        return Err(Error::InvalidKey(raw_shortcut.into()));
    };

    let mut modifiers = Modifiers {
        shift: is_single_ascii_uppercase_letter,
        ..Modifiers::NONE
    };

    for raw_modifier in raw_modifiers {
        let modifier = parse_modifier(raw_modifier)?;

        if modifiers.contains(modifier) {
            return Err(Error::DuplicateModifier(String::from(*raw_modifier)));
        }

        modifiers |= modifier;
    }

    let result = KeyboardShortcut {
        logical_key,
        modifiers,
    };

    if is_supported_keybind(&result) {
        Ok(result)
    } else {
        Err(Error::UnsupportedKeybind(raw_shortcut.into()))
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub struct Keybinds {
    keybinds: HashMap<KeyboardShortcut, KeybindAction>,
}

impl Keybinds {
    pub fn new() -> Self {
        Self {
            keybinds: HashMap::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&KeyboardShortcut, &KeybindAction)> {
        self.keybinds.iter()
    }
}

impl<'de> Deserialize<'de> for Keybinds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Keybinds;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Keybinds::new())
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut keybinds = HashMap::new();

                while let Some((raw_trigger, action)) = visitor.next_entry::<String, _>()? {
                    let trigger = parse_shortcut(&raw_trigger).map_err(de::Error::custom)?;
                    keybinds.insert(trigger, action);
                }

                Ok(Keybinds { keybinds })
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Key, KeyboardShortcut, Modifiers};

    use crate::configuration::keys::parse_shortcut;

    use super::Error;

    #[test]
    fn parse_triggers() {
        assert_eq!(
            parse_shortcut("C-p"),
            Ok(KeyboardShortcut {
                logical_key: Key::P,
                modifiers: Modifiers::COMMAND
            })
        );

        assert_eq!(
            parse_shortcut("A-S-Esc"),
            Ok(KeyboardShortcut {
                logical_key: Key::Escape,
                modifiers: Modifiers::ALT | Modifiers::SHIFT
            })
        );

        assert_eq!(
            parse_shortcut("C-ArrowDown"),
            Ok(KeyboardShortcut {
                logical_key: Key::ArrowDown,
                modifiers: Modifiers::COMMAND
            })
        );

        assert_eq!(
            parse_shortcut("F1"),
            Ok(KeyboardShortcut {
                logical_key: Key::F1,
                modifiers: Modifiers::NONE
            })
        );

        assert_eq!(
            parse_shortcut("X-X"),
            Err(Error::InvalidModifier("X".into()))
        );

        assert_eq!(parse_shortcut("XX"), Err(Error::InvalidKey("XX".into())));

        assert_eq!(
            parse_shortcut("S-A"),
            Err(Error::DuplicateModifier("S".into()))
        );

        assert_eq!(
            parse_shortcut("C-A-C-x"),
            Err(Error::DuplicateModifier("C".into()))
        );

        assert_eq!(
            parse_shortcut("C-c"),
            Err(Error::UnsupportedKeybind("C-c".into()))
        );
    }
}
