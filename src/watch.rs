use std::{io, path::Path};

#[cfg(target_os = "linux")]
use std::mem::MaybeUninit;

#[cfg(target_os = "linux")]
use rustix::{
    fd::OwnedFd,
    fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags},
    io::Errno,
};

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug)]
pub enum WatchEvent {
    ConfigChanged,
    Failed(String),
}

pub fn spawn<F>(config_dir: &Path, notify: F) -> io::Result<()>
where
    F: Fn(WatchEvent) + Send + 'static,
{
    #[cfg(target_os = "linux")]
    {
        let fd = inotify::init(CreateFlags::CLOEXEC).map_err(io::Error::from)?;
        inotify::add_watch(
            &fd,
            config_dir,
            WatchFlags::CLOSE_WRITE | WatchFlags::MOVED_TO | WatchFlags::DELETE,
        )
        .map_err(io::Error::from)?;

        std::thread::Builder::new()
            .name("momarchy-config-watch".to_owned())
            .stack_size(128 * 1024)
            .spawn(move || {
                if let Err(error) = watch(fd, &notify) {
                    notify(WatchEvent::Failed(error.to_string()));
                }
            })?;

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (config_dir, notify);
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn watch<F>(fd: OwnedFd, notify: &F) -> io::Result<()>
where
    F: Fn(WatchEvent),
{
    let mut buffer = [MaybeUninit::uninit(); 4096];
    let mut reader = inotify::Reader::new(fd, &mut buffer);

    loop {
        let first = match reader.next() {
            Ok(event) => event,
            Err(Errno::INTR) => continue,
            Err(error) => return Err(io::Error::from(error)),
        };

        let mut changed = is_relevant(&first);
        while !reader.is_buffer_empty() {
            match reader.next() {
                Ok(event) => changed |= is_relevant(&event),
                Err(Errno::INTR) => continue,
                Err(error) => return Err(io::Error::from(error)),
            }
        }

        if changed {
            notify(WatchEvent::ConfigChanged);
        }
    }
}

#[cfg(target_os = "linux")]
fn is_relevant(event: &inotify::Event<'_>) -> bool {
    if event.events().contains(ReadFlags::QUEUE_OVERFLOW) {
        return true;
    }

    event
        .file_name()
        .is_some_and(|name| is_lua_name(name.to_bytes()))
}

#[cfg(any(target_os = "linux", test))]
fn is_lua_name(name: &[u8]) -> bool {
    name.ends_with(b".lua")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_suffix_filter_is_intentionally_narrow() {
        assert!(is_lua_name(b"init.lua"));
        assert!(is_lua_name(b"home.lua"));
        assert!(!is_lua_name(b"init.lua.tmp"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn watcher_reports_lua_write_without_polling() {
        use std::{fs, sync::mpsc, time::Duration};

        let dir = std::env::temp_dir().join(format!("momarchy-watch-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let (sender, receiver) = mpsc::channel();
        spawn(&dir, move |event| {
            let _ = sender.send(event);
        })
        .unwrap();

        fs::write(dir.join("init.lua"), "return {}\n").unwrap();

        match receiver.recv_timeout(Duration::from_secs(2)).unwrap() {
            WatchEvent::ConfigChanged => {}
            WatchEvent::Failed(error) => panic!("watcher failed: {error}"),
        }

        let _ = fs::remove_dir_all(&dir);
    }
}
