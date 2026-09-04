use std::{
    io,
    mem::MaybeUninit,
    path::Path,
    sync::mpsc::Sender,
};

use rustix::{
    fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags},
    io::Errno,
};

#[derive(Debug)]
pub enum WatchEvent {
    ConfigChanged,
    Failed(String),
}

pub fn spawn(config_dir: &Path, sender: Sender<WatchEvent>) -> io::Result<()> {
    let config_dir = config_dir.to_owned();

    std::thread::Builder::new()
        .name("momarchy-config-watch".to_owned())
        .stack_size(128 * 1024)
        .spawn(move || {
            if let Err(error) = watch(&config_dir, &sender) {
                let _ = sender.send(WatchEvent::Failed(error.to_string()));
            }
        })?;

    Ok(())
}

fn watch(config_dir: &Path, sender: &Sender<WatchEvent>) -> io::Result<()> {
    let fd = inotify::init(CreateFlags::CLOEXEC).map_err(io::Error::from)?;
    inotify::add_watch(
        &fd,
        config_dir,
        WatchFlags::CLOSE_WRITE | WatchFlags::MOVED_TO | WatchFlags::DELETE,
    )
    .map_err(io::Error::from)?;

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

        if changed && sender.send(WatchEvent::ConfigChanged).is_err() {
            return Ok(());
        }
    }
}

fn is_relevant(event: &inotify::Event<'_>) -> bool {
    if event.events().contains(ReadFlags::QUEUE_OVERFLOW) {
        return true;
    }

    event
        .file_name()
        .is_some_and(|name| name.to_bytes().ends_with(b".lua"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_suffix_filter_is_intentionally_narrow() {
        assert!(b"init.lua".ends_with(b".lua"));
        assert!(!b"init.lua.tmp".ends_with(b".lua"));
    }
}
