use std::{collections::HashMap, io::Error as IoError, path::Path};

use anyhow::Result;
use mlua::{Error as LuaError, Lua, LuaSerdeExt};
use serde::Deserialize;
use smithay::input::keyboard::{keysyms as Keysyms, xkb, Keysym, ModifiersState};
use tracing::{debug, error, info, warn};

use crate::PKG_NAME;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub bindings: Bindings,
    #[serde(default = "default_workspace_count")]
    pub workspace_count: usize,
    pub outline: Outline,
}

impl mlua::UserData for Config {}

impl Config {
    pub fn load() -> Result<Self> {
        let path = if cfg!(debug_assertions) {
            std::env::current_dir().ok().map(|mut cwd| {
                cwd.push("examples/config.lua");
                cwd
            })
        } else {
            let xdg = xdg::BaseDirectories::new().ok();

            xdg.and_then(|base| {
                base.find_config_file(format!("{PKG_NAME}/config.lua"))
                    .or_else(|| base.find_config_file(format!("{PKG_NAME}.lua")))
            })
        };

        let config = if let Some(path) = path {
            info!(?path, "Trying to load configuration file");

            match Self::try_from(path.as_path()) {
                Ok(cfg) => cfg,
                Err(Error::Io(err)) => {
                    error!(?err, "Failed to load configuration file");
                    warn!("Using default configuration");
                    Self::default()
                }
                Err(Error::Lua(err)) => anyhow::bail!("Failed to parse configuration file: {err}"),
            }
        } else {
            warn!("Using default configuration");
            Self::default()
        };

        debug!(?config);

        Ok(config)
    }
}

impl TryFrom<&Path> for Config {
    type Error = Error;

    fn try_from(path: &Path) -> std::result::Result<Self, Self::Error> {
        let file = std::fs::read_to_string(path)?;
        let lua = Lua::new();

        let config = lua.from_value(lua.load(file).eval()?)?;

        Ok(config)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct Bindings(HashMap<Pattern, Action>);

impl Bindings {
    pub fn action(&self, raw_syms: &[u32], modifiers: &ModifiersState) -> Option<Action> {
        self.0.iter().find_map(|(pattern, action)| {
            (pattern.modifiers == (*modifiers).into() && raw_syms.contains(&pattern.key))
                .then_some(action.to_owned())
        })
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Outline {
    #[serde(default = "default_outline_color")]
    pub color: [f32; 3],
    #[serde(default = "default_outline_focus_color")]
    pub focus_color: [f32; 3],
    #[serde(default = "default_outline_thickness")]
    pub thickness: u8,
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct Pattern {
    #[serde(deserialize_with = "deserialize_KeyModifiers")]
    pub modifiers: KeyModifiers,
    #[serde(deserialize_with = "deserialize_Keysym")]
    pub key: u32,
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize)]
enum KeyModifier {
    Ctrl,
    Alt,
    Shift,
    Super,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

impl std::ops::AddAssign<KeyModifier> for KeyModifiers {
    fn add_assign(&mut self, rhs: KeyModifier) {
        match rhs {
            KeyModifier::Ctrl => self.ctrl = true,
            KeyModifier::Alt => self.alt = true,
            KeyModifier::Shift => self.shift = true,
            KeyModifier::Super => self.logo = true,
        };
    }
}

impl From<ModifiersState> for KeyModifiers {
    fn from(s: ModifiersState) -> Self {
        KeyModifiers {
            ctrl: s.ctrl,
            alt: s.alt,
            shift: s.shift,
            logo: s.logo,
        }
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct KeyModifiersDef(Vec<KeyModifier>);

impl From<KeyModifiersDef> for KeyModifiers {
    fn from(src: KeyModifiersDef) -> Self {
        src.0.into_iter().fold(
            KeyModifiers {
                ctrl: false,
                alt: false,
                shift: false,
                logo: false,
            },
            |mut modis, modi: KeyModifier| {
                modis += modi;
                modis
            },
        )
    }
}

#[allow(non_snake_case)]
pub fn deserialize_KeyModifiers<'de, D>(deserializer: D) -> Result<KeyModifiers, D::Error>
where
    D: serde::Deserializer<'de>,
{
    KeyModifiersDef::deserialize(deserializer).map(Into::into)
}

#[allow(non_snake_case)]
fn deserialize_Keysym<'de, D>(deserializer: D) -> Result<Keysym, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, Unexpected};

    let name = String::deserialize(deserializer)?;

    match xkb::keysym_from_name(&name, xkb::KEYSYM_NO_FLAGS) {
        Keysyms::KEY_NoSymbol => match xkb::keysym_from_name(&name, xkb::KEYSYM_CASE_INSENSITIVE) {
            Keysyms::KEY_NoSymbol => Err(<D::Error as Error>::invalid_value(
                Unexpected::Str(&name),
                &"One of the keysym names of xkbcommon.h without the 'KEY_' prefix",
            )),
            sym => {
                warn!(
                    "Key-Binding '{}' only matched case insensitive for {:?}",
                    name,
                    xkb::keysym_get_name(sym)
                );
                Ok(sym)
            }
        },
        sym => Ok(sym),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] IoError),
    #[error(transparent)]
    Lua(#[from] LuaError),
}

#[derive(Debug, Clone, Deserialize)]
pub enum Action {
    Exit,
    Spawn(String),
    SwitchToWorkspace(usize),
    MoveToWorkspace(usize),
    ToggleFullscreen,
}

fn default_workspace_count() -> usize {
    9
}

fn default_outline_color() -> [f32; 3] {
    [0.3, 0.3, 0.3]
}

fn default_outline_focus_color() -> [f32; 3] {
    [0.5, 0.5, 1.0]
}

fn default_outline_thickness() -> u8 {
    5
}
