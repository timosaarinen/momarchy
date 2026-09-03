use std::{fs, io};

pub fn print() -> io::Result<()> {
    println!("Momarchy {}", env!("CARGO_PKG_VERSION"));
    println!("platform: {}", std::env::consts::OS);

    if let Some(hostname) = read_trimmed("/etc/hostname") {
        println!("hostname: {hostname}");
    }

    if let Some(os_name) = os_pretty_name() {
        println!("os: {os_name}");
    }

    if let Some(meminfo) = read_trimmed("/proc/meminfo") {
        print_meminfo(&meminfo, "MemAvailable", "memory_available");
        print_meminfo(&meminfo, "SwapFree", "swap_free");
    }

    Ok(())
}

fn read_trimmed(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn os_pretty_name() -> Option<String> {
    let os_release = read_trimmed("/etc/os-release")?;

    for line in os_release.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return Some(value.trim_matches('"').to_owned());
        }
    }

    None
}

fn print_meminfo(meminfo: &str, key: &str, label: &str) {
    let Some(line) = meminfo.lines().find(|line| line.starts_with(key)) else {
        return;
    };

    let mut fields = line.split_whitespace();
    let _ = fields.next();
    let Some(kib) = fields.next().and_then(|value| value.parse::<u64>().ok()) else {
        return;
    };

    println!("{label}: {} MiB", kib / 1024);
}
