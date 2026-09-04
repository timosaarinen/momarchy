use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
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
    assert!(stdout.contains("STATUS Lua-asetuksessa on virhe."));

    cleanup(&root);
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
