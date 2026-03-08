use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn relay_bin() -> &'static str {
    env!("CARGO_BIN_EXE_relay")
}

fn make_codex_home(path: &Path, label: &str) {
    fs::create_dir_all(path).expect("codex home");
    fs::write(path.join("config.toml"), format!("model = \"{label}\"")).expect("config");
    fs::write(path.join("auth.json"), format!("{{\"token\":\"{label}\"}}")).expect("auth");
    fs::write(path.join("version.json"), "{\"version\":\"1\"}").expect("version");
}

fn run_json(relay_home: &Path, codex_home: &Path, args: &[&str]) -> Value {
    let output = Command::new(relay_bin())
        .args(args)
        .env("RELAY_HOME", relay_home)
        .env("CODEX_HOME", codex_home)
        .output()
        .expect("command output");
    assert!(
        output.status.success(),
        "command failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json output")
}

fn run_failure(relay_home: &Path, codex_home: &Path, args: &[&str]) -> Value {
    let output = Command::new(relay_bin())
        .args(args)
        .env("RELAY_HOME", relay_home)
        .env("CODEX_HOME", codex_home)
        .output()
        .expect("command output");
    assert!(!output.status.success(), "command unexpectedly succeeded");
    serde_json::from_slice(&output.stdout).expect("json output")
}

#[test]
fn profile_crud_and_auto_switch_commands_work() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let alternate_home = temp.path().join("alternate");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&alternate_home, "alternate");

    let add = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "profiles",
            "add",
            "--nickname",
            "alternate",
            "--codex-home",
            alternate_home.to_string_lossy().as_ref(),
        ],
    );
    let profile_id = add["data"]["id"].as_str().expect("profile id").to_string();

    let edit = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "profiles",
            "edit",
            &profile_id,
            "--nickname",
            "alternate-updated",
        ],
    );
    assert_eq!(edit["data"]["nickname"], "alternate-updated");

    let disable = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "profiles", "disable", &profile_id],
    );
    assert_eq!(disable["data"]["enabled"], false);

    let enable = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "profiles", "enable", &profile_id],
    );
    assert_eq!(enable["data"]["enabled"], true);

    let auto_switch = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "auto-switch", "enable"],
    );
    assert_eq!(auto_switch["data"]["auto_switch_enabled"], true);

    let status = run_json(&relay_home, &live_codex_home, &["--json", "status"]);
    assert_eq!(status["data"]["settings"]["auto_switch_enabled"], true);
}

#[test]
fn import_switch_events_logs_and_diagnostics_work() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let alternate_home = temp.path().join("alternate");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&alternate_home, "alternate");

    let imported = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "profiles",
            "import-codex",
            "--nickname",
            "imported-live",
        ],
    );
    let imported_id = imported["data"]["id"]
        .as_str()
        .expect("imported id")
        .to_string();

    let added = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "profiles",
            "add",
            "--nickname",
            "alternate",
            "--codex-home",
            alternate_home.to_string_lossy().as_ref(),
        ],
    );
    let alternate_id = added["data"]["id"]
        .as_str()
        .expect("alternate id")
        .to_string();

    let switched = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &alternate_id],
    );
    assert_eq!(switched["data"]["profile_id"], alternate_id);
    assert_eq!(
        fs::read_to_string(live_codex_home.join("config.toml")).expect("live config"),
        "model = \"alternate\""
    );

    let next = run_json(&relay_home, &live_codex_home, &["--json", "switch", "next"]);
    assert_eq!(next["data"]["profile_id"], imported_id);

    let imported_home = Path::new(
        imported["data"]["codex_home"]
            .as_str()
            .expect("imported codex home"),
    );
    fs::remove_file(imported_home.join("config.toml")).expect("remove imported config");

    let failed = run_failure(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &imported_id],
    );
    assert_eq!(failed["success"], false);

    let events = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "events", "list", "--limit", "10"],
    );
    assert!(
        events["data"]
            .as_array()
            .expect("events array")
            .iter()
            .any(|event| event["profile_id"] == imported_id)
    );

    let logs = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "logs", "tail", "--lines", "20"],
    );
    assert!(
        logs["data"]["lines"]
            .as_array()
            .expect("log lines")
            .iter()
            .any(|line| line.as_str().unwrap_or("").contains("switch"))
    );

    let diagnostics = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "diagnostics", "export"],
    );
    let archive_path = diagnostics["data"]["archive_path"]
        .as_str()
        .expect("archive path");
    assert!(Path::new(archive_path).exists());
}
