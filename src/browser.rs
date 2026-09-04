use std::{io, process::Command};

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

#[cfg(target_os = "macos")]
pub fn open(target: &str) -> io::Result<()> {
    spawn("open", target)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn open(target: &str) -> io::Result<()> {
    spawn("xdg-open", target)
}

fn spawn(program: &str, target: &str) -> io::Result<()> {
    Command::new(program).arg(target).spawn()?;
    Ok(())
}
