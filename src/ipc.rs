use std::{
    env, fs, io,
    os::unix::net::UnixDatagram,
    path::PathBuf,
};

const SOCKET_NAME: &str = "momarchy-home.sock";
const MAX_PACKET_BYTES: usize = 4096;
const MAX_MESSAGE_BYTES: usize = MAX_PACKET_BYTES - 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Ping,
    Message(String),
    Failed(String),
}

pub struct Listener {
    path: PathBuf,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn spawn_listener<F>(notify: F) -> io::Result<Listener>
where
    F: Fn(Event) + Send + 'static,
{
    let path = socket_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        let probe = UnixDatagram::unbound()?;
        let live = probe
            .connect(&path)
            .and_then(|()| probe.send(b"PROBE"))
            .is_ok();
        if live {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("Momarchy Home is already listening at {}", path.display()),
            ));
        }
        fs::remove_file(&path)?;
    }

    let socket = UnixDatagram::bind(&path)?;
    let listener_path = path.clone();
    std::thread::Builder::new()
        .name("momarchy-home-ipc".to_owned())
        .stack_size(128 * 1024)
        .spawn(move || {
            let mut buffer = [0u8; MAX_PACKET_BYTES];
            loop {
                match socket.recv(&mut buffer) {
                    Ok(length) => {
                        if let Some(event) = decode_packet(&buffer[..length]) {
                            notify(event);
                        }
                    }
                    Err(error) => {
                        notify(Event::Failed(error.to_string()));
                        break;
                    }
                }
            }
        })?;

    Ok(Listener {
        path: listener_path,
    })
}

pub fn send_ping() -> io::Result<()> {
    send_packet(b"PING")
}

pub fn send_message(message: &str) -> io::Result<()> {
    if message.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "message must not be empty",
        ));
    }
    if message.len() > MAX_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("message is too long (maximum {MAX_MESSAGE_BYTES} UTF-8 bytes)"),
        ));
    }

    let mut packet = Vec::with_capacity(4 + message.len());
    packet.extend_from_slice(b"MSG\0");
    packet.extend_from_slice(message.as_bytes());
    send_packet(&packet)
}

fn send_packet(packet: &[u8]) -> io::Result<()> {
    let path = socket_path()?;
    let socket = UnixDatagram::unbound()?;
    socket.connect(&path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "could not reach Momarchy Home at {}: {error}. Is Home running?",
                path.display()
            ),
        )
    })?;
    socket.send(packet)?;
    Ok(())
}

fn socket_path() -> io::Result<PathBuf> {
    if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR")
        && !runtime_dir.is_empty()
    {
        return Ok(PathBuf::from(runtime_dir).join(SOCKET_NAME));
    }

    let home = env::var_os("HOME").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "HOME and XDG_RUNTIME_DIR are both unavailable",
        )
    })?;
    Ok(PathBuf::from(home)
        .join(".local/state/momarchy")
        .join(SOCKET_NAME))
}

fn decode_packet(packet: &[u8]) -> Option<Event> {
    if packet == b"PING" {
        return Some(Event::Ping);
    }
    let message = packet.strip_prefix(b"MSG\0")?;
    let message = std::str::from_utf8(message).ok()?;
    if message.trim().is_empty() {
        return None;
    }
    Some(Event::Message(message.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_ping_and_utf8_message() {
        assert_eq!(decode_packet(b"PING"), Some(Event::Ping));
        assert_eq!(
            decode_packet("MSG\0Hei äiti!".as_bytes()),
            Some(Event::Message("Hei äiti!".to_owned()))
        );
    }

    #[test]
    fn ignores_probe_and_malformed_packets() {
        assert_eq!(decode_packet(b"PROBE"), None);
        assert_eq!(decode_packet(b"MSG\0"), None);
        assert_eq!(decode_packet(b"wat"), None);
    }
}
