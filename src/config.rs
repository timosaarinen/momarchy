use std::{
    collections::HashSet,
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use mlua::{Lua, Table};

pub const DEFAULT_INIT_LUA: &str = include_str!("../lua/init.lua");

#[derive(Clone, Debug)]
pub struct Config {
    pub home: String,
    pub screens: Vec<Screen>,
}

#[derive(Clone, Debug)]
pub struct Screen {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub body: Option<String>,
    pub buttons: Vec<Button>,
}

#[derive(Clone, Debug)]
pub struct Button {
    pub id: String,
    pub label: String,
    pub hint: String,
    pub action: Action,
}

#[derive(Clone, Debug)]
pub enum Action {
    Navigate(String),
    Message(String),
    Open {
        target: String,
        live_message: String,
    },
    Command {
        kind: String,
        program: String,
        args: Vec<String>,
        live_message: String,
    },
}

pub struct InitialConfig {
    pub config: Config,
    pub path: PathBuf,
    pub used_embedded_fallback: bool,
}

impl Config {
    pub fn screen(&self, id: &str) -> Option<&Screen> {
        self.screens.iter().find(|screen| screen.id == id)
    }
}

impl Action {
    pub fn kind(&self) -> &str {
        match self {
            Self::Navigate(_) | Self::Message(_) => "internal",
            Self::Open { .. } => "browser",
            Self::Command { kind, .. } => kind,
        }
    }
}

pub fn initialize() -> io::Result<InitialConfig> {
    let path = ensure_user_config()?;
    let config_dir = path
        .parent()
        .ok_or_else(|| io::Error::other("Momarchy config path has no parent directory"))?;

    match load(&path) {
        Ok(config) => Ok(InitialConfig {
            config,
            path,
            used_embedded_fallback: false,
        }),
        Err(error) => {
            eprintln!(
                "momarchy: could not load {}: {error}; using embedded config",
                path.display()
            );

            let config = load_source(DEFAULT_INIT_LUA, config_dir, "<embedded init.lua>").map_err(
                |fallback| {
                    io::Error::other(format!(
                        "user config failed ({error}); embedded config also failed ({fallback})"
                    ))
                },
            )?;

            Ok(InitialConfig {
                config,
                path,
                used_embedded_fallback: true,
            })
        }
    }
}

pub fn load(path: &Path) -> Result<Config, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let config_dir = path
        .parent()
        .ok_or_else(|| format!("config path has no parent: {}", path.display()))?;

    load_source(&source, config_dir, &path.display().to_string())
}

fn ensure_user_config() -> io::Result<PathBuf> {
    let path = user_config_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Momarchy config path has no parent directory"))?;
    fs::create_dir_all(parent)?;

    match fs::symlink_metadata(&path) {
        Ok(_) => return Ok(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    materialize_default(&path, parent)?;
    Ok(path)
}

fn materialize_default(path: &Path, parent: &Path) -> io::Result<()> {
    let temp = parent.join(format!(".init.lua.{}.tmp", std::process::id()));
    let _ = fs::remove_file(&temp);

    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(DEFAULT_INIT_LUA.as_bytes())?;
        file.sync_all()?;
        drop(file);

        // hard_link is an atomic no-replace publication step: another Momarchy
        // process or administrator-created init.lua wins rather than being overwritten.
        match fs::hard_link(&temp, path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            Err(error) => Err(error),
        }
    })();

    let _ = fs::remove_file(&temp);
    result
}

fn user_config_path() -> io::Result<PathBuf> {
    let base = if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return Err(io::Error::other(
            "neither XDG_CONFIG_HOME nor HOME is set; cannot locate Momarchy config",
        ));
    };

    Ok(base.join("momarchy/init.lua"))
}

fn load_source(source: &str, config_dir: &Path, source_name: &str) -> Result<Config, String> {
    let lua = Lua::new();
    configure_package_path(&lua, config_dir)?;

    let root: Table = lua
        .load(source)
        .set_name(source_name)
        .eval()
        .map_err(|error| format!("{source_name}: {error}"))?;

    let version: i64 = get(&root, "version", source_name)?;
    if version != 1 {
        return Err(format!(
            "{source_name}: unsupported config version {version}; expected 1"
        ));
    }

    let home: String = get(&root, "home", source_name)?;
    let screens_table: Table = get(&root, "screens", source_name)?;
    let mut screens = Vec::new();

    for pair in screens_table.pairs::<String, Table>() {
        let (id, table) = pair.map_err(|error| format!("{source_name}: screens: {error}"))?;
        screens.push(parse_screen(id, table, source_name)?);
    }

    screens.sort_by(|left, right| left.id.cmp(&right.id));

    let config = Config { home, screens };
    validate(&config, source_name)?;
    Ok(config)
}

fn configure_package_path(lua: &Lua, config_dir: &Path) -> Result<(), String> {
    let package: Table = lua
        .globals()
        .get("package")
        .map_err(|error| format!("could not access Lua package table: {error}"))?;
    let existing: String = package
        .get("path")
        .map_err(|error| format!("could not read Lua package.path: {error}"))?;
    let dir = config_dir.to_string_lossy();

    package
        .set("path", format!("{dir}/?.lua;{dir}/?/init.lua;{existing}"))
        .map_err(|error| format!("could not configure Lua package.path: {error}"))
}

fn parse_screen(id: String, table: Table, source_name: &str) -> Result<Screen, String> {
    let title: String = get(&table, "title", source_name)?;
    let subtitle: String = get(&table, "subtitle", source_name)?;
    let body: Option<String> = table
        .get("body")
        .map_err(|error| format!("{source_name}: screen {id}.body: {error}"))?;
    let buttons_table: Table = get(&table, "buttons", source_name)?;
    let mut buttons = Vec::new();

    for value in buttons_table.sequence_values::<Table>() {
        let table =
            value.map_err(|error| format!("{source_name}: screen {id}.buttons: {error}"))?;
        buttons.push(parse_button(&id, table, source_name)?);
    }

    Ok(Screen {
        id,
        title,
        subtitle,
        body,
        buttons,
    })
}

fn parse_button(screen_id: &str, table: Table, source_name: &str) -> Result<Button, String> {
    let id: String = get(&table, "id", source_name)?;
    let label: String = get(&table, "label", source_name)?;
    let hint: String = get(&table, "hint", source_name)?;
    let action_table: Table = get(&table, "action", source_name)?;
    let action = parse_action(screen_id, &id, action_table, source_name)?;

    Ok(Button {
        id,
        label,
        hint,
        action,
    })
}

fn parse_action(
    screen_id: &str,
    button_id: &str,
    table: Table,
    source_name: &str,
) -> Result<Action, String> {
    let prefix = format!("{source_name}: screen {screen_id}, button {button_id}");
    let screen: Option<String> = table
        .get("screen")
        .map_err(|error| format!("{prefix}.action.screen: {error}"))?;
    let message: Option<String> = table
        .get("message")
        .map_err(|error| format!("{prefix}.action.message: {error}"))?;
    let open: Option<String> = table
        .get("open")
        .map_err(|error| format!("{prefix}.action.open: {error}"))?;
    let command: Option<Table> = table
        .get("command")
        .map_err(|error| format!("{prefix}.action.command: {error}"))?;

    let variants = usize::from(screen.is_some())
        + usize::from(message.is_some())
        + usize::from(open.is_some())
        + usize::from(command.is_some());
    if variants != 1 {
        return Err(format!(
            "{prefix}.action must contain exactly one of screen, message, open, command"
        ));
    }

    if let Some(screen) = screen {
        return Ok(Action::Navigate(screen));
    }
    if let Some(message) = message {
        return Ok(Action::Message(message));
    }
    if let Some(target) = open {
        let live_message = table
            .get::<Option<String>>("live_message")
            .map_err(|error| format!("{prefix}.action.live_message: {error}"))?
            .unwrap_or_else(|| "Avataan.".to_owned());
        return Ok(Action::Open {
            target,
            live_message,
        });
    }

    let command = command.expect("variant count guarantees command exists");
    let mut parts = Vec::new();
    for value in command.sequence_values::<String>() {
        parts.push(value.map_err(|error| format!("{prefix}.action.command: {error}"))?);
    }
    if parts.is_empty() {
        return Err(format!("{prefix}.action.command must not be empty"));
    }

    let program = parts.remove(0);
    let kind = table
        .get::<Option<String>>("kind")
        .map_err(|error| format!("{prefix}.action.kind: {error}"))?
        .unwrap_or_else(|| "process".to_owned());
    let live_message = table
        .get::<Option<String>>("live_message")
        .map_err(|error| format!("{prefix}.action.live_message: {error}"))?
        .unwrap_or_else(|| "Avataan.".to_owned());

    Ok(Action::Command {
        kind,
        program,
        args: parts,
        live_message,
    })
}

fn validate(config: &Config, source_name: &str) -> Result<(), String> {
    if config.screens.is_empty() {
        return Err(format!("{source_name}: screens must not be empty"));
    }
    if config.screen(&config.home).is_none() {
        return Err(format!(
            "{source_name}: home screen {:?} does not exist",
            config.home
        ));
    }

    let mut screen_ids = HashSet::new();
    for screen in &config.screens {
        if screen.id.is_empty() {
            return Err(format!("{source_name}: screen id must not be empty"));
        }
        if !screen_ids.insert(screen.id.as_str()) {
            return Err(format!(
                "{source_name}: duplicate screen id {:?}",
                screen.id
            ));
        }
        if screen.buttons.is_empty() {
            return Err(format!(
                "{source_name}: screen {:?} must contain at least one button",
                screen.id
            ));
        }

        let mut button_ids = HashSet::new();
        for button in &screen.buttons {
            if button.id.is_empty() {
                return Err(format!(
                    "{source_name}: screen {:?} has an empty button id",
                    screen.id
                ));
            }
            if !button_ids.insert(button.id.as_str()) {
                return Err(format!(
                    "{source_name}: screen {:?} has duplicate button id {:?}",
                    screen.id, button.id
                ));
            }
        }
    }

    for screen in &config.screens {
        for button in &screen.buttons {
            if let Action::Navigate(target) = &button.action
                && config.screen(target).is_none()
            {
                return Err(format!(
                    "{source_name}: screen {:?}, button {:?} targets missing screen {:?}",
                    screen.id, button.id, target
                ));
            }
        }
    }

    Ok(())
}

fn get<T: mlua::FromLua>(table: &Table, key: &str, source_name: &str) -> Result<T, String> {
    table
        .get(key)
        .map_err(|error| format!("{source_name}: {key}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_is_valid() {
        let config = load_source(DEFAULT_INIT_LUA, Path::new("."), "embedded").unwrap();
        assert_eq!(config.home, "home");
        assert_eq!(config.screen("home").unwrap().buttons.len(), 8);
    }

    #[test]
    fn missing_navigation_target_is_rejected() {
        let source = r#"
            return {
              version = 1,
              home = "home",
              screens = {
                home = {
                  title = "Home",
                  subtitle = "Test",
                  buttons = {
                    {
                      id = "bad",
                      label = "Bad",
                      hint = "Bad target",
                      action = { screen = "missing" },
                    },
                  },
                },
              },
            }
        "#;

        let error = load_source(source, Path::new("."), "test").unwrap_err();
        assert!(error.contains("missing screen"));
    }

    #[test]
    fn action_requires_one_variant() {
        let source = r#"
            return {
              version = 1,
              home = "home",
              screens = {
                home = {
                  title = "Home",
                  subtitle = "Test",
                  buttons = {
                    {
                      id = "bad",
                      label = "Bad",
                      hint = "Ambiguous",
                      action = { message = "Hi", screen = "home" },
                    },
                  },
                },
              },
            }
        "#;

        let error = load_source(source, Path::new("."), "test").unwrap_err();
        assert!(error.contains("exactly one"));
    }

    #[test]
    fn empty_screen_is_rejected() {
        let source = r#"
            return {
              version = 1,
              home = "home",
              screens = {
                home = {
                  title = "Home",
                  subtitle = "Test",
                  buttons = {},
                },
              },
            }
        "#;

        let error = load_source(source, Path::new("."), "test").unwrap_err();
        assert!(error.contains("at least one button"));
    }
}
