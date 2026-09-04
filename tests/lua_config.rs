use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio},
};

const DEFAULT_INIT_LUA: &str = include_str!("../lua/init.lua");

#[test]
fn first_home_run_materializes_embedded_config() {
    let root = test_config_home("materialize");

    let output = run_home_automation(&root, "quit\n");
    assert_success(&output);

    let path = root.join("momarchy/init.lua");
    assert_eq!(fs::read_to_string(path).unwrap(), DEFAULT_INIT_LUA);

    cleanup(&root);
}

#[test]
fn existing_admin_config_is_authoritative() {
    let root = test_config_home("preserve");
    let path = root.join("momarchy/init.lua");
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let custom = r#"return {
  version = 1,
  home = "home",
  screens = {
    home = {
      title = "CUSTOM",
      subtitle = "Test",
      buttons = {
        {
          id = "custom",
          label = "CUSTOM BUTTON",
          hint = "From admin config",
          action = { message = "Works" },
        },
      },
    },
  },
}
"#;
    fs::write(&path, custom).unwrap();

    let output = run_home_automation(&root, "quit\n");
    assert_success(&output);
    assert_eq!(fs::read_to_string(&path).unwrap(), custom);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("custom\tinternal\tCUSTOM BUTTON\tFrom admin config"));

    cleanup(&root);
}

#[test]
fn broken_cold_start_config_falls_back_without_overwriting_it() {
    let root = test_config_home("fallback");
    let path = root.join("momarchy/init.lua");
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let broken = "this is definitely not valid Lua\n";
    fs::write(&path, broken).unwrap();

    let output = run_home_automation(&root, "quit\n");
    assert_success(&output);
    assert_eq!(fs::read_to_string(&path).unwrap(), broken);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("internet\tbrowser\tINTERNET\tAvaa selain"));
    assert!(stdout.contains(
        "STATUS Asetuksissa on virhe. Käytetään turvallisia oletusasetuksia."
    ));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("momarchy: could not load"));
    assert!(stderr.contains("using embedded config"));

    cleanup(&root);
}

#[test]
fn reload_uses_a_fresh_lua_vm_and_reloads_required_modules() {
    let root = test_config_home("fresh-vm");
    let config_dir = root.join("momarchy");
    fs::create_dir_all(&config_dir).unwrap();

    fs::write(
        config_dir.join("init.lua"),
        r#"local home = require("home")
return {
  version = 1,
  home = "home",
  screens = { home = home },
}
"#,
    )
    .unwrap();
    write_home_module(&config_dir, "BEFORE");

    let (mut child, mut stdin, mut stdout) = spawn_home_automation(&root);
    let first = read_snapshot(&mut stdout);
    assert!(first.contains("custom\tinternal\tBEFORE\tModule value"));

    write_home_module(&config_dir, "AFTER");
    stdin.write_all(b"reload\n").unwrap();
    stdin.flush().unwrap();

    let second = read_snapshot(&mut stdout);
    assert!(second.contains("custom\tinternal\tAFTER\tModule value"));
    assert!(second.contains("STATUS Asetukset päivitetty."));

    stdin.write_all(b"quit\n").unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    assert!(child.wait().unwrap().success());

    cleanup(&root);
}

fn write_home_module(config_dir: &Path, label: &str) {
    fs::write(
        config_dir.join("home.lua"),
        format!(
            r#"return {{
  title = "HOME",
  subtitle = "Test",
  buttons = {{
    {{
      id = "custom",
      label = "{label}",
      hint = "Module value",
      action = {{ message = "Works" }},
    }},
  }},
}}
"#
        ),
    )
    .unwrap();
}

fn run_home_automation(config_home: &Path, input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_momarchy"))
        .args(["home", "--automation"])
        .env("XDG_CONFIG_HOME", config_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();

    child.wait_with_output().unwrap()
}

fn spawn_home_automation(config_home: &Path) -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_momarchy"))
        .args(["home", "--automation"])
        .env("XDG_CONFIG_HOME", config_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    (child, stdin, stdout)
}

fn read_snapshot(stdout: &mut impl BufRead) -> String {
    let mut snapshot = String::new();
    loop {
        let mut line = String::new();
        assert_ne!(stdout.read_line(&mut line).unwrap(), 0, "unexpected EOF");
        let end = line.trim_end() == "END";
        snapshot.push_str(&line);
        if end {
            return snapshot;
        }
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "momarchy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn test_config_home(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "momarchy-lua-config-test-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}
