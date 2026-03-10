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

fn write_usage_cache(relay_home: &Path, snapshots: Vec<Value>) {
    fs::create_dir_all(relay_home).expect("relay home");
    fs::write(
        relay_home.join("usage.json"),
        serde_json::json!({ "snapshots": snapshots }).to_string(),
    )
    .expect("usage cache");
}

fn usage_snapshot(
    profile_id: &str,
    profile_name: &str,
    session_status: &str,
    weekly_status: &str,
    confidence: &str,
    stale: bool,
) -> Value {
    let auto_switch_reason = if session_status == "Exhausted" {
        Some("SessionExhausted")
    } else if weekly_status == "Exhausted" {
        Some("WeeklyExhausted")
    } else {
        None
    };

    serde_json::json!({
        "profile_id": profile_id,
        "profile_name": profile_name,
        "source": "Local",
        "confidence": confidence,
        "stale": stale,
        "last_refreshed_at": "2026-03-09T12:00:00Z",
        "next_reset_at": "2026-03-09T17:00:00Z",
        "session": {
            "used_percent": 95.0,
            "window_minutes": 300,
            "reset_at": "2026-03-09T17:00:00Z",
            "status": session_status,
            "exact": true
        },
        "weekly": {
            "used_percent": 25.0,
            "window_minutes": 10080,
            "reset_at": "2026-03-12T06:36:18Z",
            "status": weekly_status,
            "exact": true
        },
        "auto_switch_reason": auto_switch_reason,
        "can_auto_switch": auto_switch_reason.is_some(),
        "message": "fixture usage"
    })
}

fn write_oauth_auth(path: &Path, account_id: &str, email: &str) {
    let payload = match email {
        "imported@example.com" => "eyJlbWFpbCI6ImltcG9ydGVkQGV4YW1wbGUuY29tIn0",
        "live@example.com" => "eyJlbWFpbCI6ImxpdmVAZXhhbXBsZS5jb20ifQ",
        _ => "eyJlbWFpbCI6InVzZXJAZXhhbXBsZS5jb20ifQ",
    };
    fs::write(
        path.join("auth.json"),
        format!(
            "{{\"auth_mode\":\"oauth\",\"tokens\":{{\"access_token\":\"access-{account_id}\",\"refresh_token\":\"refresh-{account_id}\",\"id_token\":\"eyJhbGciOiJub25lIn0.{payload}.\",\"account_id\":\"{account_id}\"}}}}"
        ),
    )
    .expect("oauth auth");
}

fn run_json(relay_home: &Path, codex_home: &Path, args: &[&str]) -> Value {
    run_json_with_env(relay_home, codex_home, args, &[])
}

fn run_json_with_env(
    relay_home: &Path,
    codex_home: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Value {
    let output = Command::new(relay_bin())
        .args(args)
        .env("RELAY_HOME", relay_home)
        .env("CODEX_HOME", codex_home)
        .envs(envs.iter().copied())
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

fn run_text(relay_home: &Path, codex_home: &Path, args: &[&str]) -> String {
    run_text_with_env(relay_home, codex_home, args, &[])
}

fn run_text_with_env(
    relay_home: &Path,
    codex_home: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> String {
    let output = Command::new(relay_bin())
        .args(args)
        .env("RELAY_HOME", relay_home)
        .env("CODEX_HOME", codex_home)
        .envs(envs.iter().copied())
        .output()
        .expect("command output");
    assert!(
        output.status.success(),
        "command failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 output")
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
    run_json_with_stdin_env(relay_home, codex_home, args, stdin, &[])
}

fn run_json_with_stdin_env(
    relay_home: &Path,
    codex_home: &Path,
    args: &[&str],
    stdin: &str,
    envs: &[(&str, &str)],
) -> Value {
    let mut child = Command::new(relay_bin())
        .args(args)
        .env("RELAY_HOME", relay_home)
        .env("CODEX_HOME", codex_home)
        .envs(envs.iter().copied())
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

fn run_failure_with_stdin(
    relay_home: &Path,
    codex_home: &Path,
    args: &[&str],
    stdin: &str,
) -> Value {
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
    assert!(!output.status.success(), "command unexpectedly succeeded");
    serde_json::from_slice(&output.stdout).expect("json output")
}

fn make_fake_bin(root: &Path) -> std::path::PathBuf {
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    fs::write(
        bin_dir.join("codex"),
        r#"#!/bin/sh
set -eu
if [ "${1:-}" = "login" ]; then
  if [ "${2:-}" = "--device-auth" ]; then
    cat <<'EOF'
Welcome to Codex [v0.112.0]
Open this link in your browser:
https://auth.openai.com/codex/device

Enter this code:
031C-3FZ9S
EOF
  fi
  mkdir -p "${CODEX_HOME:?}"
  cat > "${CODEX_HOME}/auth.json" <<'EOF'
{"tokens":{"access_token":"access-token","refresh_token":"refresh-token","id_token":"id-token","account_id":"acct-123"}}
EOF
  exit 0
fi
if [ "${1:-}" = "--version" ]; then
  echo "codex-cli 0.107.0"
  exit 0
fi
exit 0
"#,
    )
    .expect("codex stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(bin_dir.join("codex"), fs::Permissions::from_mode(0o755))
            .expect("codex perms");
    }
    bin_dir
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
            "codex",
            "add",
            "--nickname",
            "alternate",
            "--agent-home",
            alternate_home.to_string_lossy().as_ref(),
        ],
    );
    let profile_id = add["data"]["id"].as_str().expect("profile id").to_string();

    let edit = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
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
        &["--json", "disable", &profile_id],
    );
    assert_eq!(disable["data"]["enabled"], false);

    let enable = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "enable", &profile_id],
    );
    assert_eq!(enable["data"]["enabled"], true);

    let auto_switch = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "autoswitch", "enable"],
    );
    assert_eq!(auto_switch["data"]["auto_switch_enabled"], true);

    let status = run_json(&relay_home, &live_codex_home, &["--json", "status"]);
    assert_eq!(status["data"]["settings"]["auto_switch_enabled"], true);

    let usage_list = run_json(&relay_home, &live_codex_home, &["--json", "list"]);
    let items = usage_list["data"].as_array().expect("profile list");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["profile"]["id"], profile_id);
    assert_eq!(items[0]["usage_summary"]["source"], "Fallback");
    assert_eq!(items[0]["usage_summary"]["session"]["status"], "Unknown");
    assert_eq!(items[0]["usage_summary"]["weekly"]["status"], "Unknown");
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
        &["--json", "codex", "import", "--nickname", "imported-live"],
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
            "codex",
            "add",
            "--nickname",
            "alternate",
            "--agent-home",
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

    write_usage_cache(
        &relay_home,
        vec![
            usage_snapshot(
                &alternate_id,
                "alternate",
                "Healthy",
                "Healthy",
                "High",
                false,
            ),
            usage_snapshot(
                &imported_id,
                "imported-live",
                "Healthy",
                "Healthy",
                "High",
                false,
            ),
        ],
    );

    let next = run_json(&relay_home, &live_codex_home, &["--json", "switch"]);
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
        &["--json", "activity", "events", "list", "--limit", "10"],
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
        &["--json", "activity", "logs", "tail", "--lines", "20"],
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
        &["--json", "activity", "diagnostics", "export"],
    );
    let archive_path = diagnostics["data"]["archive_path"]
        .as_str()
        .expect("archive path");
    assert!(Path::new(archive_path).exists());
}

#[test]
fn profile_show_and_activity_event_filters_work() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");
    write_oauth_auth(&live_codex_home, "acct-imported", "imported@example.com");

    let imported = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "import", "--nickname", "imported"],
    );
    let imported_id = imported["data"]["id"]
        .as_str()
        .expect("profile id")
        .to_string();

    write_usage_cache(
        &relay_home,
        vec![usage_snapshot(
            &imported_id,
            "imported",
            "Healthy",
            "Healthy",
            "High",
            false,
        )],
    );

    let imported_home = Path::new(
        imported["data"]["agent_home"]
            .as_str()
            .expect("imported agent home"),
    );
    fs::remove_file(imported_home.join("config.toml")).expect("remove imported config");
    let _ = run_failure(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &imported_id],
    );

    let detail = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "show", &imported_id],
    );
    assert_eq!(detail["data"]["profile"]["id"], imported_id);
    assert_eq!(detail["data"]["usage"]["profile_id"], imported_id);
    assert_eq!(detail["data"]["switch_eligible"], true);
    assert_eq!(
        detail["data"]["last_failure_event"]["reason"],
        "ValidationFailed"
    );

    let filtered = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "activity",
            "events",
            "list",
            "--profile-id",
            &imported_id,
            "--reason",
            "validation-failed",
            "--limit",
            "5",
        ],
    );
    let items = filtered["data"].as_array().expect("filtered events");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["profile_id"], imported_id);
    assert_eq!(items[0]["reason"], "ValidationFailed");
}

#[test]
fn switch_next_skips_profiles_with_exhausted_usage() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let first_home = temp.path().join("first");
    let second_home = temp.path().join("second");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&first_home, "first");
    make_codex_home(&second_home, "second");

    let first = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "first",
            "--priority",
            "10",
            "--agent-home",
            first_home.to_string_lossy().as_ref(),
        ],
    );
    let first_id = first["data"]["id"].as_str().expect("first id").to_string();

    let second = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "second",
            "--priority",
            "20",
            "--agent-home",
            second_home.to_string_lossy().as_ref(),
        ],
    );
    let second_id = second["data"]["id"]
        .as_str()
        .expect("second id")
        .to_string();

    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &first_id],
    );

    write_usage_cache(
        &relay_home,
        vec![
            usage_snapshot(&first_id, "first", "Exhausted", "Healthy", "High", false),
            usage_snapshot(&second_id, "second", "Healthy", "Healthy", "High", false),
        ],
    );

    let next = run_json(&relay_home, &live_codex_home, &["--json", "switch"]);
    assert_eq!(next["data"]["profile_id"], second_id);
}

#[test]
fn switch_next_returns_conflict_when_all_enabled_profiles_are_exhausted() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let first_home = temp.path().join("first");
    let second_home = temp.path().join("second");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&first_home, "first");
    make_codex_home(&second_home, "second");

    let first = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "first",
            "--priority",
            "10",
            "--agent-home",
            first_home.to_string_lossy().as_ref(),
        ],
    );
    let first_id = first["data"]["id"].as_str().expect("first id").to_string();

    let second = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "second",
            "--priority",
            "20",
            "--agent-home",
            second_home.to_string_lossy().as_ref(),
        ],
    );
    let second_id = second["data"]["id"]
        .as_str()
        .expect("second id")
        .to_string();

    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &first_id],
    );

    write_usage_cache(
        &relay_home,
        vec![
            usage_snapshot(&first_id, "first", "Exhausted", "Healthy", "High", false),
            usage_snapshot(&second_id, "second", "Healthy", "Exhausted", "High", false),
        ],
    );

    let failure = run_failure(&relay_home, &live_codex_home, &["--json", "switch"]);
    assert_eq!(failure["error_code"], "RELAY_CONFLICT");
    assert_eq!(
        failure["message"],
        "all enabled profiles are exhausted or unavailable for auto-switch"
    );
}

#[test]
fn import_codex_defaults_nickname_to_live_email() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");
    write_oauth_auth(&live_codex_home, "acct-imported", "imported@example.com");

    let imported = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "import"],
    );

    assert_eq!(imported["data"]["nickname"], "imported@example.com");
    let current = run_json(&relay_home, &live_codex_home, &["--json", "show"]);
    assert_eq!(current["data"]["profile"]["id"], imported["data"]["id"]);
}

#[test]
fn removing_active_profile_switches_to_remaining_enabled_profile() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let first_home = temp.path().join("first");
    let second_home = temp.path().join("second");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&first_home, "first");
    make_codex_home(&second_home, "second");

    let first = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "first",
            "--priority",
            "10",
            "--agent-home",
            first_home.to_string_lossy().as_ref(),
        ],
    );
    let first_id = first["data"]["id"].as_str().expect("first id").to_string();

    let second = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "second",
            "--priority",
            "20",
            "--agent-home",
            second_home.to_string_lossy().as_ref(),
        ],
    );
    let second_id = second["data"]["id"]
        .as_str()
        .expect("second id")
        .to_string();

    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &first_id],
    );
    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "remove", &first_id],
    );

    let current = run_json(&relay_home, &live_codex_home, &["--json", "show"]);
    assert_eq!(current["data"]["profile"]["id"], second_id);
}

#[test]
fn removing_last_active_profile_clears_active_state() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let only_home = temp.path().join("only");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&only_home, "only");

    let only = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "only",
            "--priority",
            "10",
            "--agent-home",
            only_home.to_string_lossy().as_ref(),
        ],
    );
    let only_id = only["data"]["id"].as_str().expect("only id").to_string();

    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &only_id],
    );
    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "remove", &only_id],
    );

    let current = run_failure(&relay_home, &live_codex_home, &["--json", "show"]);
    assert_eq!(current["error_code"], "RELAY_NOT_FOUND");
    assert_eq!(current["message"], "no active profile");
}

#[test]
fn usage_profile_list_refresh_and_config_work() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let alternate_home = temp.path().join("alternate");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&alternate_home, "alternate");

    let imported = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "import", "--nickname", "live"],
    );
    let active_id = imported["data"]["id"]
        .as_str()
        .expect("active id")
        .to_string();
    let added = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "alternate",
            "--agent-home",
            alternate_home.to_string_lossy().as_ref(),
        ],
    );
    let alternate_id = added["data"]["id"]
        .as_str()
        .expect("alternate id")
        .to_string();

    let refreshed = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "refresh", &alternate_id],
    );
    assert_eq!(refreshed["data"]["profile_id"], alternate_id);

    let profile_usage = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "show", &alternate_id],
    );
    assert_eq!(profile_usage["data"]["profile"]["id"], alternate_id);
    assert_eq!(profile_usage["data"]["usage"]["profile_id"], alternate_id);
    assert_eq!(
        profile_usage["data"]["usage"]["session"]["used_percent"],
        41.0
    );

    let list = run_json(&relay_home, &live_codex_home, &["--json", "list"]);
    let items = list["data"].as_array().expect("profile array");
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|item| item["profile"]["id"] == active_id));
    assert!(
        items
            .iter()
            .any(|item| item["profile"]["id"] == alternate_id)
    );

    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &active_id],
    );
    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "refresh", &active_id],
    );
    let current = run_json(&relay_home, &live_codex_home, &["--json", "show"]);
    assert_eq!(current["data"]["profile"]["id"], active_id);
    assert_eq!(current["data"]["usage"]["source"], "Local");

    let global_settings = run_json(&relay_home, &live_codex_home, &["--json", "settings"]);
    assert!(global_settings["data"].get("usage_source_mode").is_none());

    let updated_settings = run_json_with_stdin(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "settings", "set", "--input-json", "-"],
        r#"{"source_mode":"web-enhanced"}"#,
    );
    assert_eq!(updated_settings["data"]["usage_source_mode"], "WebEnhanced");
}

#[test]
fn usage_settings_reject_removed_refresh_keys() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");

    let invalid_flag = run_failure_raw(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "settings",
            "set",
            "--menu-open-refresh-stale-after-seconds",
            "5",
        ],
    );
    assert!(
        !invalid_flag.status.success(),
        "command unexpectedly succeeded"
    );
    assert!(
        String::from_utf8_lossy(&invalid_flag.stderr)
            .contains("--menu-open-refresh-stale-after-seconds")
    );

    let invalid_json = run_failure_with_stdin(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "settings", "set", "--input-json", "-"],
        r#"{"menu_open_refresh_stale_after_seconds":5}"#,
    );
    assert_eq!(invalid_json["error_code"], "RELAY_INVALID_INPUT");
}

#[test]
fn codex_login_and_remote_usage_probe_work() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let fake_bin = make_fake_bin(temp.path());
    let usage_fixture = temp.path().join("official-usage.json");
    let refresh_fixture = temp.path().join("official-refresh.json");
    make_codex_home(&live_codex_home, "live");
    fs::write(
        &usage_fixture,
        r#"{"plan_type":"team","rate_limit":{"primary_window":{"used_percent":18.0,"limit_window_seconds":18000,"reset_after_seconds":2700},"secondary_window":{"used_percent":28.0,"limit_window_seconds":604800,"reset_after_seconds":302400}}}"#,
    )
    .expect("usage fixture");
    fs::write(
        &refresh_fixture,
        r#"{"access_token":"new-access-token","refresh_token":"new-refresh-token","id_token":"new-id-token"}"#,
    )
    .expect("refresh fixture");
    let path_env = std::env::join_paths(
        [fake_bin.as_path(), Path::new("/usr/bin"), Path::new("/bin")].into_iter(),
    )
    .expect("path env");
    let path_env_owned = path_env.to_string_lossy().into_owned();
    let usage_url = format!("file://{}", usage_fixture.display());
    let refresh_url = format!("file://{}", refresh_fixture.display());
    let envs = [
        ("PATH", path_env_owned.as_str()),
        ("RELAY_OFFICIAL_USAGE_URL", usage_url.as_str()),
        ("RELAY_OFFICIAL_REFRESH_URL", refresh_url.as_str()),
    ];

    let logged_in = run_json_with_env(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "login", "--nickname", "browser"],
        &envs,
    );
    let profile_id = logged_in["data"]["profile"]["id"]
        .as_str()
        .expect("profile id")
        .to_string();
    assert_eq!(logged_in["data"]["activated"], false);
    assert_eq!(
        logged_in["data"]["probe_identity"]["principal_id"],
        "acct-123"
    );
    assert_eq!(
        logged_in["data"]["probe_identity"]["credentials"]["account_id"],
        "acct-123"
    );
    let status = run_json_with_env(&relay_home, &live_codex_home, &["--json", "status"], &envs);
    assert_eq!(
        status["data"]["active_state"]["active_profile_id"],
        serde_json::Value::Null
    );

    let refreshed = run_json_with_env(
        &relay_home,
        &live_codex_home,
        &["--json", "refresh", &profile_id],
        &envs,
    );
    assert_eq!(refreshed["data"]["source"], "WebEnhanced");
    assert_eq!(refreshed["data"]["session"]["used_percent"], 18.0);
    assert_eq!(refreshed["data"]["weekly"]["used_percent"], 28.0);

    write_oauth_auth(&live_codex_home, "acct-live", "live@example.com");
    let relinked = run_json_with_env(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "relink", &profile_id],
        &envs,
    );
    assert_eq!(relinked["data"]["principal_id"], "acct-live");
    assert_eq!(relinked["data"]["metadata"]["email"], "live@example.com");
}

#[test]
fn codex_login_device_auth_streams_instructions_to_stderr() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let fake_bin = make_fake_bin(temp.path());
    make_codex_home(&live_codex_home, "live");

    let path_env = std::env::join_paths(
        [fake_bin.as_path(), Path::new("/usr/bin"), Path::new("/bin")].into_iter(),
    )
    .expect("path env");

    let output = Command::new(relay_bin())
        .args([
            "--json",
            "codex",
            "login",
            "--device-auth",
            "--nickname",
            "browser",
        ])
        .env("RELAY_HOME", &relay_home)
        .env("CODEX_HOME", &live_codex_home)
        .env("PATH", path_env)
        .output()
        .expect("command output");

    assert!(
        output.status.success(),
        "command failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let logged_in: Value = serde_json::from_slice(&output.stdout).expect("json output");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert_eq!(logged_in["data"]["activated"], false);
    assert!(stderr.contains("https://auth.openai.com/codex/device"));
    assert!(stderr.contains("031C-3FZ9S"));
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
            "codex",
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
            "edit",
            "--input-json",
            edit_payload_path.to_string_lossy().as_ref(),
        ],
    );
    assert_eq!(edit["data"]["nickname"], "json-edited");

    let switched = run_json_with_stdin(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", "--input-json", "-"],
        &format!("{{\"id\":\"{}\"}}", profile_id),
    );
    assert_eq!(switched["data"]["profile_id"], profile_id);

    let auto_switch_payload_path = temp.path().join("auto-switch.json");
    fs::write(&auto_switch_payload_path, "{\"enabled\":true}").expect("auto-switch payload");
    let auto_switch = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "autoswitch",
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
            "codex",
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
fn json_commands_still_emit_json_when_write_bootstrap_fails() {
    let temp = tempdir().expect("tempdir");
    let relay_home_file = temp.path().join("relay-home-file");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");
    fs::write(&relay_home_file, "not a directory").expect("relay home file");

    let output = run_failure_raw(
        &relay_home_file,
        &live_codex_home,
        &["--json", "autoswitch", "set", "--enabled", "true"],
    );
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

#[test]
fn read_only_commands_do_not_create_relay_home() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");

    let status = run_json(&relay_home, &live_codex_home, &["--json", "status"]);
    assert_eq!(status["data"]["profile_count"], 0);
    assert!(
        !relay_home.exists(),
        "read-only status should not create relay home"
    );

    let list = run_json(&relay_home, &live_codex_home, &["--json", "list"]);
    assert_eq!(list["data"], serde_json::json!([]));
    assert!(
        !relay_home.exists(),
        "read-only list should not create relay home or usage cache"
    );
}

#[test]
fn usage_text_output_renders_table_and_detail_views() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let alternate_home = temp.path().join("alternate");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&alternate_home, "alternate");

    let imported = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "import", "--nickname", "live"],
    );
    let active_id = imported["data"]["id"]
        .as_str()
        .expect("active id")
        .to_string();
    let added = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "alternate",
            "--agent-home",
            alternate_home.to_string_lossy().as_ref(),
        ],
    );
    let alternate_id = added["data"]["id"]
        .as_str()
        .expect("alternate id")
        .to_string();

    run_json(&relay_home, &live_codex_home, &["--json", "refresh"]);
    run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &active_id],
    );

    let list = run_text(&relay_home, &live_codex_home, &["list"]);
    assert!(list.contains("Nickname"));
    assert!(list.contains("Session"));
    assert!(list.contains(&active_id));
    assert!(list.contains(&alternate_id));
    assert!(!list.contains("Notes"));
    assert!(!list.contains("Healthy ("));

    let detail = run_text(&relay_home, &live_codex_home, &["show"]);
    assert!(detail.contains("Session"));
    assert!(detail.contains("Weekly"));
    assert!(!detail.contains("Confidence"));
}

#[test]
fn human_readable_outputs_cover_core_command_families() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let alternate_home = temp.path().join("alternate");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&alternate_home, "alternate");

    let doctor = run_text_with_env(
        &relay_home,
        &live_codex_home,
        &["doctor"],
        &[("NO_COLOR", "1")],
    );
    assert!(doctor.contains("Environment"));
    assert!(!doctor.contains("\u{1b}["));

    let imported = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "import", "--nickname", "live"],
    );
    let imported_id = imported["data"]["id"]
        .as_str()
        .expect("imported id")
        .to_string();
    let added = run_text(
        &relay_home,
        &live_codex_home,
        &[
            "codex",
            "add",
            "--nickname",
            "alternate",
            "--agent-home",
            alternate_home.to_string_lossy().as_ref(),
        ],
    );
    assert!(added.contains("Profile"));
    assert!(added.contains("Nickname"));

    let profiles = run_text(&relay_home, &live_codex_home, &["list"]);
    assert!(profiles.contains("Nickname"));
    assert!(profiles.contains("Profile ID"));
    assert!(!profiles.contains("Notes"));

    let status = run_text(&relay_home, &live_codex_home, &["status"]);
    assert!(status.contains("Active State"));
    assert!(status.contains("Settings"));

    let switched = run_text(&relay_home, &live_codex_home, &["switch", &imported_id]);
    assert!(switched.contains("Checkpoint"));

    let auto_switch = run_text(&relay_home, &live_codex_home, &["autoswitch", "enable"]);
    assert!(auto_switch.contains("Auto-switch Enabled"));

    let active_home = Path::new(imported["data"]["agent_home"].as_str().expect("agent home"));
    fs::remove_file(active_home.join("config.toml")).expect("remove config");
    let _ = run_failure(
        &relay_home,
        &live_codex_home,
        &["--json", "switch", &imported_id],
    );

    let events = run_text(
        &relay_home,
        &live_codex_home,
        &["activity", "events", "list", "--limit", "10"],
    );
    assert!(events.contains("Reason"));
    assert!(events.contains("ValidationFailed"));

    let logs = run_text(
        &relay_home,
        &live_codex_home,
        &["activity", "logs", "tail", "--lines", "10"],
    );
    assert!(logs.contains("Path:"));

    let diagnostics = run_text(
        &relay_home,
        &live_codex_home,
        &["activity", "diagnostics", "export"],
    );
    assert!(diagnostics.contains("Archive Path"));
}

#[test]
fn failed_add_does_not_persist_invalid_profile() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let invalid_home = temp.path().join("invalid-home");
    make_codex_home(&live_codex_home, "live");
    fs::create_dir_all(&invalid_home).expect("invalid home");

    let failure = run_failure(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "codex",
            "add",
            "--nickname",
            "broken",
            "--agent-home",
            invalid_home.to_string_lossy().as_ref(),
        ],
    );
    assert_eq!(failure["success"], false);
    assert_eq!(failure["error_code"], "RELAY_VALIDATION");

    let list = run_json(&relay_home, &live_codex_home, &["--json", "list"]);
    assert_eq!(list["data"], serde_json::json!([]));
}
