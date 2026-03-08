use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::tempdir;

fn relay_bin() -> &'static str {
    env!("CARGO_BIN_EXE_relay")
}

fn make_codex_home(path: &Path, label: &str) {
    fs::create_dir_all(path).expect("codex home");
    fs::write(path.join("config.toml"), format!("model = \"{label}\"")).expect("config");
    fs::write(path.join("auth.json"), format!("{{\"token\":\"{label}\"}}")).expect("auth");
    fs::write(path.join("version.json"), "{\"version\":\"1\"}").expect("version");
    let sessions_dir = path.join("sessions/2026/03/08");
    fs::create_dir_all(&sessions_dir).expect("sessions");
    fs::write(
        sessions_dir.join("rollout.jsonl"),
        "{\"timestamp\":\"2026-03-08T12:39:47.628Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"rate_limits\":{\"primary\":{\"used_percent\":41.0,\"window_minutes\":300,\"resets_at\":1772979934},\"secondary\":{\"used_percent\":12.0,\"window_minutes\":10080,\"resets_at\":1773566734}}}}}\n",
    )
    .expect("usage session");
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

fn run_failure_raw(relay_home: &Path, codex_home: &Path, args: &[&str]) -> std::process::Output {
    Command::new(relay_bin())
        .args(args)
        .env("RELAY_HOME", relay_home)
        .env("CODEX_HOME", codex_home)
        .output()
        .expect("command output")
}

fn run_json_with_stdin(relay_home: &Path, codex_home: &Path, args: &[&str], stdin: &str) -> Value {
    let mut child = Command::new(relay_bin())
        .args(args)
        .env("RELAY_HOME", relay_home)
        .env("CODEX_HOME", codex_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn command");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait output");
    assert!(
        output.status.success(),
        "command failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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

    let usage = run_json(&relay_home, &live_codex_home, &["--json", "usage"]);
    assert_eq!(usage["data"]["source"], "Local");
    assert_eq!(usage["data"]["session"]["used_percent"], 41.0);
    assert_eq!(usage["data"]["weekly"]["used_percent"], 12.0);
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
        imported["data"]["agent_home"]
            .as_str()
            .expect("imported agent home"),
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

#[test]
fn json_input_mutations_and_stdin_work() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let alternate_home = temp.path().join("alternate");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&alternate_home, "alternate");

    let add_payload_path = temp.path().join("add.json");
    fs::write(
        &add_payload_path,
        format!(
            "{{\"nickname\":\"json-added\",\"priority\":5,\"agent_home\":\"{}\",\"auth_mode\":\"ConfigFilesystem\"}}",
            alternate_home.to_string_lossy()
        ),
    )
    .expect("add payload");

    let add = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "profiles",
            "add",
            "--input-json",
            add_payload_path.to_string_lossy().as_ref(),
        ],
    );
    let profile_id = add["data"]["id"].as_str().expect("profile id").to_string();

    let edit_payload_path = temp.path().join("edit.json");
    fs::write(
        &edit_payload_path,
        format!(
            "{{\"id\":\"{}\",\"nickname\":\"json-edited\",\"priority\":7}}",
            profile_id
        ),
    )
    .expect("edit payload");
    let edit = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "profiles",
            "edit",
            "--input-json",
            edit_payload_path.to_string_lossy().as_ref(),
        ],
    );
    assert_eq!(edit["data"]["nickname"], "json-edited");

    let switch_payload = format!("{{\"target\":\"{}\"}}", profile_id);
    let switched = run_json_with_stdin(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", "--input-json", "-"],
        &switch_payload,
    );
    assert_eq!(switched["data"]["profile_id"], profile_id);

    let auto_switch_payload_path = temp.path().join("auto-switch.json");
    fs::write(&auto_switch_payload_path, "{\"enabled\":true}").expect("auto-switch payload");
    let auto_switch = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "auto-switch",
            "set",
            "--input-json",
            auto_switch_payload_path.to_string_lossy().as_ref(),
        ],
    );
    assert_eq!(auto_switch["data"]["auto_switch_enabled"], true);

    let mixed = run_failure(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "profiles",
            "add",
            "--nickname",
            "bad",
            "--input-json",
            add_payload_path.to_string_lossy().as_ref(),
        ],
    );
    assert_eq!(mixed["error_code"], "RELAY_INVALID_INPUT");
}

#[test]
fn json_commands_still_emit_json_when_bootstrap_fails() {
    let temp = tempdir().expect("tempdir");
    let relay_home_file = temp.path().join("relay-home-file");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");
    fs::write(&relay_home_file, "not a directory").expect("relay home file");

    let output = run_failure_raw(&relay_home_file, &live_codex_home, &["--json", "status"]);
    assert!(!output.status.success(), "command unexpectedly succeeded");

    let response: Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(response["success"], false);
    assert_eq!(response["error_code"], "RELAY_IO");
    assert!(
        response["message"]
            .as_str()
            .unwrap_or("")
            .contains("Not a directory")
            || response["message"]
                .as_str()
                .unwrap_or("")
                .contains("File exists")
    );
}
