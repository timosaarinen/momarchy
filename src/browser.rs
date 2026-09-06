use std::{io, process::Command};

#[cfg(target_os = "linux")]
use std::{
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
const BROWSER_READY_TIMEOUT: Duration = Duration::from_secs(45);
#[cfg(target_os = "linux")]
const BROWSER_READY_POLL: Duration = Duration::from_millis(250);

#[cfg(target_os = "linux")]
pub fn open(target: &str) -> io::Result<()> {
    // Omarchy already owns default-browser selection and launches the browser
    // detached from the caller. Do not wait on a browser PID: modern browsers
    // may reuse an existing process, so the child process is not a meaningful
    // lifecycle signal. Home stays alive underneath and Hyprland owns focus.
    match spawn("omarchy-launch-browser", target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => spawn("xdg-open", target),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "linux")]
pub fn focus_or_open(target: &str) -> io::Result<()> {
    let pattern = match default_browser_focus_pattern() {
        Ok(pattern) => pattern,
        Err(_) => return open(target),
    };

    match focus_browser(&pattern) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return open(target),
        Err(error) => return Err(error),
    }

    open(target)?;

    let deadline = Instant::now() + BROWSER_READY_TIMEOUT;
    loop {
        match focus_browser(&pattern) {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }

        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "browser window did not appear before timeout",
            ));
        }

        thread::sleep(BROWSER_READY_POLL);
    }
}

#[cfg(target_os = "linux")]
fn focus_browser(pattern: &str) -> io::Result<bool> {
    let status = Command::new("omarchy-hyprland-focus-app")
        .arg(pattern)
        .status()?;
    Ok(status.success())
}

#[cfg(target_os = "linux")]
fn default_browser_focus_pattern() -> io::Result<String> {
    let desktop_id = default_browser_desktop_id()?;
    let executable = desktop_executable(&desktop_id);
    let app_name = executable
        .as_deref()
        .and_then(|value| Path::new(value).file_name())
        .and_then(|value| value.to_str())
        .unwrap_or_else(|| desktop_id.trim_end_matches(".desktop"));
    let app_name = app_name.strip_suffix("-stable").unwrap_or(app_name);

    if app_name.is_empty() {
        return Err(io::Error::other("default browser application name is empty"));
    }

    Ok(format!("^{}.*$", regex_escape(app_name)))
}

#[cfg(target_os = "linux")]
fn default_browser_desktop_id() -> io::Result<String> {
    let mut settings = Command::new("xdg-settings");
    settings
        .env_remove("BROWSER")
        .args(["get", "default-web-browser"]);
    if let Some(value) = successful_stdout(settings)? {
        return Ok(value);
    }

    let mut mime = Command::new("xdg-mime");
    mime.args(["query", "default", "x-scheme-handler/https"]);
    successful_stdout(mime)?.ok_or_else(|| io::Error::other("default browser is not configured"))
}

#[cfg(target_os = "linux")]
fn successful_stdout(mut command: Command) -> io::Result<Option<String>> {
    let output = command.output()?;
    if !output.status.success() {
        return Ok(None);
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok((!value.is_empty()).then_some(value))
}

#[cfg(target_os = "linux")]
fn desktop_executable(desktop_id: &str) -> Option<String> {
    application_dirs().into_iter().find_map(|directory| {
        let source = fs::read_to_string(directory.join(desktop_id)).ok()?;
        source.lines().find_map(|line| {
            let exec = line.strip_prefix("Exec=")?;
            exec.split_whitespace()
                .next()
                .map(|value| value.trim_matches('"').to_owned())
        })
    })
}

#[cfg(target_os = "linux")]
fn application_dirs() -> Vec<PathBuf> {
    let mut directories = Vec::new();

    if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
        directories.push(PathBuf::from(xdg_data_home).join("applications"));
    } else if let Some(home) = env::var_os("HOME") {
        directories.push(PathBuf::from(&home).join(".local/share/applications"));
    }

    if let Some(home) = env::var_os("HOME") {
        directories.push(PathBuf::from(home).join(".nix-profile/share/applications"));
    }

    directories.push(PathBuf::from("/usr/share/applications"));
    directories
}

#[cfg(target_os = "linux")]
fn regex_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '\\' | '.' | '^' | '$' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(target_os = "macos")]
pub fn open(target: &str) -> io::Result<()> {
    spawn("open", target)
}

#[cfg(target_os = "macos")]
pub fn focus_or_open(target: &str) -> io::Result<()> {
    // macOS is a development host, not the appliance target. Keep the live-action
    // fallback simple here; the Linux path owns Hyprland window reuse semantics.
    open(target)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn open(target: &str) -> io::Result<()> {
    spawn("xdg-open", target)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn focus_or_open(target: &str) -> io::Result<()> {
    open(target)
}

fn spawn(program: &str, target: &str) -> io::Result<()> {
    Command::new(program).arg(target).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn browser_focus_regex_is_literal() {
        assert_eq!(regex_escape("com.example+browser"), "com\\.example\\+browser");
    }
}
