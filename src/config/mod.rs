use std::collections::HashMap;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Result;
use mlua::{Error as LuaError, Lua, LuaSerdeExt};
use serde::Deserialize;
use smithay::input::keyboard::{keysyms as Keysyms, xkb, Keysym, ModifiersState};
use smithay::reexports::calloop::channel::Event as ChannelEvent;
use smithay::reexports::calloop::{self, LoopHandle};
use tracing::{debug, error, info, warn};

use self::watcher::Watcher;
use crate::state::CalloopData;
use crate::PKG_NAME;

mod watcher;

const DEFAULT_CONFIG: &str = include_str!("../../examples/config.lua");

pub type Color = [f32; 3];

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub path: PathBuf,
    pub bindings: Bindings,
    #[serde(default = "default_workspace_count")]
    pub workspace_count: usize,
    #[serde(alias = "border")]
    pub outline: Outline,
}

impl Default for Config {
    fn default() -> Self {
        warn!("Using default configuration");
        let config = Config::from_str(DEFAULT_CONFIG).expect("Default config contains errors");
        Config { path: PathBuf::from("../../examples/config.lua"), ..config }
    }
}

impl mlua::UserData for Config {}

impl Config {
    pub fn new() -> Result<Self> {
        let config = if let Some(path) = {
            let xdg = xdg::BaseDirectories::new().ok();
            xdg.and_then(|base| {
                base.find_config_file(format!("{PKG_NAME}/config.lua"))
                    .or_else(|| base.find_config_file(format!("{PKG_NAME}.lua")))
            })
        } {
            match Config::try_from(path.as_path()) {
                Ok(cfg) => cfg,
                Err(Error::Io(err)) => {
                    error!(?err, "Failed to load configuration file");
                    Self::default()
                }
                Err(Error::Lua(err)) => {
                    anyhow::bail!("Failed to parse configuration file: {err}");
                }
            }
        } else if cfg!(debug_assertions) {
            Self::default()
        } else {
            let xdg = xdg::BaseDirectories::new().ok();
            if let Some(path) = xdg.and_then(|base| base.create_config_directory(PKG_NAME).ok()) {
                let path = path.join("config.lua");
                info!(?path, "Writing default configuration");
                std::fs::write(path.as_path(), DEFAULT_CONFIG)?;
                Self::try_from(path.as_path())?
            } else {
                Self::default()
            }
        };

        debug!("{:#?}", config);

        Ok(config)
    }

    fn reload(&mut self) {
        debug!("Reloading configuration");
        let config = Self::try_from(self.path.as_path()).unwrap();
        *self = config;
    }

    pub fn setup_watcher(path: &Path, event_loop: LoopHandle<'static, CalloopData>) {
        let (tx, rx) = calloop::channel::sync_channel(1);
        let watcher = Watcher::new(path.to_owned(), tx);
        event_loop
            .insert_source(rx, move |event, _, data| match event {
                ChannelEvent::Msg(()) => data.state.config.reload(),
                ChannelEvent::Closed => (),
            })
            .unwrap();
        Box::leak(Box::new(watcher));
    }
}

impl FromStr for Config {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lua = Lua::new();

        let value = lua.load(s).eval()?;
        Ok(lua.from_value(value)?)
    }
}

impl TryFrom<&Path> for Config {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        info!(?path, "Trying to load configuration file");
        let s = std::fs::read_to_string(path)?;
        let config = Config::from_str(&s)?;
        Ok(Config { path: path.to_owned(), ..config })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct Bindings(HashMap<Pattern, Action>);

impl Bindings {
    pub fn action(&self, raw_syms: &[Keysym], modifiers: &ModifiersState) -> Option<Action> {
        self.0.iter().find_map(|(pattern, action)| {
            (pattern.modifiers == (*modifiers).into() && raw_syms.contains(&pattern.key))
                .then_some(action.to_owned())
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Outline {
    #[serde(deserialize_with = "deserialize_Color", default = "default_outline_color")]
    pub color: Color,
    #[serde(deserialize_with = "deserialize_Color", default = "default_outline_focus_color")]
    pub focused_color: Color,
    #[serde(default = "default_outline_radius")]
    pub radius: usize,
    #[serde(default = "default_outline_thickness")]
    pub thickness: usize,
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct Pattern {
    #[serde(deserialize_with = "deserialize_KeyModifiers")]
    pub modifiers: KeyModifiers,
    #[serde(deserialize_with = "deserialize_Keysym")]
    pub key: Keysym,
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
        KeyModifiers { ctrl: s.ctrl, alt: s.alt, shift: s.shift, logo: s.logo }
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct KeyModifiersDef(Vec<KeyModifier>);

impl From<KeyModifiersDef> for KeyModifiers {
    fn from(src: KeyModifiersDef) -> Self {
        src.0.into_iter().fold(
            KeyModifiers { ctrl: false, alt: false, shift: false, logo: false },
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

    match xkb::keysym_from_name(&name, xkb::KEYSYM_NO_FLAGS).raw() {
        Keysyms::KEY_NoSymbol => {
            match xkb::keysym_from_name(&name, xkb::KEYSYM_CASE_INSENSITIVE).raw() {
                Keysyms::KEY_NoSymbol => Err(<D::Error as Error>::invalid_value(
                    Unexpected::Str(&name),
                    &"One of the keysym names of xkbcommon.h without the 'KEY_' prefix",
                )),
                sym => {
                    warn!(
                        "Key-Binding '{}' only matched case insensitive for {:?}",
                        name,
                        xkb::keysym_get_name(sym.into())
                    );
                    Ok(sym.into())
                }
            }
        }
        sym => Ok(sym.into()),
    }
}

#[allow(non_snake_case)]
fn deserialize_Color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // TODO: other formats
    <[f32; 3]>::deserialize(deserializer)
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
    Close,
    Spawn(String),
    SwitchToWorkspace(usize),
    MoveToWorkspace(usize),
    ToggleFullscreen,
}

fn default_workspace_count() -> usize {
    9
}

fn default_outline_color() -> Color {
    [0.3, 0.3, 0.3]
}

fn default_outline_focus_color() -> Color {
    [0.5, 0.5, 1.0]
}

fn default_outline_radius() -> usize {
    24
}

fn default_outline_thickness() -> usize {
    5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_default_config_ok() {
        assert!(Config::from_str(DEFAULT_CONFIG).is_ok());
    }
}
