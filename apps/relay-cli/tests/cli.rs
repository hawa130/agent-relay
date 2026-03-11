use sea_orm::{ConnectionTrait, Database};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};
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

async fn create_legacy_relay_db(path: &Path) {
    let connection = Database::connect(format!("sqlite://{}?mode=rwc", path.to_string_lossy()))
        .await
        .expect("legacy db");
    connection
        .execute_unprepared("CREATE TABLE seaql_migrations (version TEXT PRIMARY KEY NOT NULL)")
        .await
        .expect("create legacy migrations table");
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

fn run_help(args: &[&str]) -> String {
    let output = Command::new(relay_bin())
        .args(args)
        .output()
        .expect("help output");
    assert!(
        output.status.success(),
        "help command failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 help output")
}

fn assert_order(text: &str, first: &str, second: &str) {
    let first_index = text
        .find(first)
        .unwrap_or_else(|| panic!("missing marker: {first}"));
    let second_index = text
        .find(second)
        .unwrap_or_else(|| panic!("missing marker: {second}"));
    assert!(
        first_index < second_index,
        "expected `{first}` before `{second}` in:\n{text}"
    );
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
  if [ -n "${RELAY_TEST_LOGIN_SLEEP:-}" ]; then
    sleep "${RELAY_TEST_LOGIN_SLEEP}"
  fi
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
fn top_level_help_lists_common_commands_first_with_descriptions() {
    let help = run_help(&["--help"]);

    assert!(help.contains("status      Show current relay state and active profile"));
    assert!(help.contains("list        List managed profiles with usage summaries"));
    assert!(help.contains("show        Inspect one profile, or the current profile when omitted"));
    assert!(help.contains("switch      Activate a profile, or switch to the next eligible one"));
    assert!(help.contains("refresh     Refresh usage data for one or more profiles"));
    assert!(help.contains("codex       Manage Codex profiles, login flows, and settings"));
    assert!(help.contains("doctor      Inspect relay environment, paths, and binary health"));
    assert!(help.contains("--json     Emit machine-readable JSON output"));

    assert_order(&help, "status", "codex");
    assert_order(&help, "codex", "doctor");
    assert_order(&help, "doctor", "edit");
    assert_order(&help, "edit", "remove");
}

#[test]
fn nested_help_lists_subcommands_with_descriptions() {
    let codex_help = run_help(&["codex", "--help"]);
    assert!(codex_help.contains("login     Create a new profile by signing in with Codex"));
    assert!(
        codex_help.contains("import    Import the current live Codex home as a managed profile")
    );
    assert!(
        codex_help.contains("add       Register an existing Codex home or config as a profile")
    );
    assert!(
        codex_help.contains("recover   Recover saved Codex profile snapshots into the database")
    );
    assert!(codex_help.contains("settings  Inspect or update Codex-wide settings"));
    assert_order(&codex_help, "login", "import");
    assert_order(&codex_help, "import", "add");
    assert_order(&codex_help, "add", "recover");
    assert_order(&codex_help, "recover", "relink");

    let activity_help = run_help(&["activity", "--help"]);
    assert!(activity_help.contains("events       Inspect recorded switch failures and cooldowns"));
    assert!(activity_help.contains("logs         Read relay log output"));
    assert!(activity_help.contains("diagnostics  Export a diagnostic bundle for debugging"));
    assert_order(&activity_help, "events", "logs");
    assert_order(&activity_help, "logs", "diagnostics");

    let autoswitch_help = run_help(&["autoswitch", "--help"]);
    assert!(autoswitch_help.contains("show     Show current automatic switching settings"));
    assert!(autoswitch_help.contains("enable   Turn automatic switching on"));
    assert!(autoswitch_help.contains("disable  Turn automatic switching off"));
    assert!(
        autoswitch_help
            .contains("set      Set automatic switching explicitly with a boolean value")
    );
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
fn settings_set_updates_network_query_concurrency() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");

    let updated = run_json(
        &relay_home,
        &live_codex_home,
        &[
            "--json",
            "settings",
            "set",
            "--network-query-concurrency",
            "12",
        ],
    );
    assert_eq!(updated["success"], true);
    assert_eq!(updated["data"]["network_query_concurrency"], 12);

    let loaded = run_json(&relay_home, &live_codex_home, &["--json", "settings"]);
    assert_eq!(loaded["success"], true);
    assert_eq!(loaded["data"]["network_query_concurrency"], 12);
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
    assert!(list.contains("% · "));
    assert!(!list.contains("Source"));
    assert!(!list.contains("Auth"));
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

#[test]
fn recover_rebuilds_profiles_from_saved_snapshots() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let recovered_home = relay_home.join("profiles").join("imported_1");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&recovered_home, "recovered");
    write_oauth_auth(&recovered_home, "acct-restored", "imported@example.com");
    fs::create_dir_all(&relay_home).expect("relay home");
    fs::write(
        relay_home.join("state.json"),
        serde_json::json!({
            "active_profile_id": "old-profile-id",
            "last_switch_at": null,
            "last_switch_result": "Success",
            "auto_switch_enabled": true
        })
        .to_string(),
    )
    .expect("stale state");

    let recovery = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "recover"],
    );
    assert_eq!(recovery["data"]["scanned_dirs"], 1);
    assert_eq!(
        recovery["data"]["recovered"]
            .as_array()
            .expect("recovered")
            .len(),
        1
    );
    assert_eq!(
        recovery["data"]["skipped"]
            .as_array()
            .expect("skipped")
            .len(),
        0
    );
    assert_eq!(
        recovery["data"]["recovered"][0]["profile"]["nickname"],
        "imported@example.com"
    );
    assert_eq!(
        recovery["data"]["recovered"][0]["probe_identity_restored"],
        true
    );

    let profiles = run_json(&relay_home, &live_codex_home, &["--json", "list"]);
    let items = profiles["data"].as_array().expect("profile list");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["profile"]["nickname"], "imported@example.com");

    let status = run_json(&relay_home, &live_codex_home, &["--json", "status"]);
    assert_eq!(
        status["data"]["active_state"]["active_profile_id"],
        serde_json::Value::Null
    );

    let second = run_json(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "recover"],
    );
    assert_eq!(
        second["data"]["recovered"]
            .as_array()
            .expect("recovered")
            .len(),
        0
    );
    assert_eq!(
        second["data"]["skipped"].as_array().expect("skipped").len(),
        1
    );
}

#[tokio::test]
async fn legacy_database_returns_schema_incompatible_error() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    fs::create_dir_all(&relay_home).expect("relay home");
    make_codex_home(&live_codex_home, "live");
    create_legacy_relay_db(&relay_home.join("relay.db")).await;

    let failure = run_failure(&relay_home, &live_codex_home, &["--json", "doctor"]);
    assert_eq!(failure["success"], false);
    assert_eq!(failure["error_code"], "RELAY_SCHEMA_INCOMPATIBLE");
    assert!(
        failure["message"]
            .as_str()
            .expect("message")
            .contains("remove the existing database")
    );
}

struct DaemonHarness {
    child: Child,
    stdin: ChildStdin,
    stdout_rx: Receiver<Value>,
}

impl DaemonHarness {
    fn spawn(relay_home: &Path, codex_home: &Path) -> Self {
        Self::spawn_with_env(relay_home, codex_home, &[])
    }

    fn spawn_with_env(relay_home: &Path, codex_home: &Path, envs: &[(&str, &str)]) -> Self {
        let mut child = Command::new(relay_bin())
            .args(["daemon", "--stdio"])
            .env("RELAY_HOME", relay_home)
            .env("CODEX_HOME", codex_home)
            .envs(envs.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn daemon");
        let stdin = child.stdin.take().expect("daemon stdin");
        let stdout = child.stdout.take().expect("daemon stdout");
        let (stdout_tx, stdout_rx) = mpsc::channel();
        thread::spawn(move || {
            use std::io::{BufRead, BufReader};

            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };
                let Ok(value) = serde_json::from_str(line.trim()) else {
                    continue;
                };
                if stdout_tx.send(value).is_err() {
                    break;
                }
            }
        });
        Self { child, stdin, stdout_rx }
    }

    fn send_request(&mut self, id: &str, method: &str, params: Value) {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_raw(&payload.to_string());
    }

    fn send_raw(&mut self, line: &str) {
        self.stdin
            .write_all(line.as_bytes())
            .expect("write daemon stdin");
        self.stdin.write_all(b"\n").expect("write newline");
        self.stdin.flush().expect("flush daemon stdin");
    }

    fn read_message(&mut self) -> Value {
        self.stdout_rx.recv().expect("daemon stdout closed unexpectedly")
    }

    fn read_message_timeout(&mut self, timeout: Duration) -> Option<Value> {
        self.stdout_rx.recv_timeout(timeout).ok()
    }

    fn shutdown(mut self) {
        self.send_request("shutdown", "shutdown", serde_json::json!({}));
        let deadline = Instant::now() + Duration::from_secs(10);
        let response = loop {
            let Some(message) = self.read_message_timeout(Duration::from_millis(500)) else {
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting for shutdown response"
                );
                continue;
            };
            if message["id"] == "shutdown" {
                break message;
            }
        };
        assert_eq!(response["id"], "shutdown");
        assert_eq!(response["result"]["accepted"], true);
        let exit_deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("poll daemon exit") {
                break status;
            }
            assert!(
                Instant::now() < exit_deadline,
                "timed out waiting for daemon exit"
            );
            thread::sleep(Duration::from_millis(50));
        };
        assert!(status.success(), "daemon exit status: {status}");
    }
}

#[test]
fn daemon_stdio_initialize_subscribe_refresh_and_shutdown_work() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let profile_home = temp.path().join("profile-home");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&profile_home, "profile");

    let add_payload = format!(
        "{{\"nickname\":\"daemon-profile\",\"priority\":50,\"agent_home\":\"{}\",\"auth_mode\":\"ConfigFilesystem\"}}",
        profile_home.to_string_lossy()
    );
    let add = run_json_with_stdin(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "add", "--input-json", "-"],
        &add_payload,
    );
    assert_eq!(add["data"]["nickname"], "daemon-profile");
    let profile_id = add["data"]["id"].as_str().expect("profile id");

    let mut daemon = DaemonHarness::spawn(&relay_home, &live_codex_home);
    daemon.send_request(
        "1",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": true,
                "supports_health_updates": true
            }
        }),
    );
    let initialize = daemon.read_message();
    assert_eq!(initialize["id"], "1");
    assert_eq!(initialize["result"]["protocol_version"], "1");
    assert_eq!(
        initialize["result"]["initial_state"]["status"]["profile_count"],
        1
    );

    daemon.send_request(
        "2",
        "session/subscribe",
        serde_json::json!({
            "topics": [
                "usage.updated",
                "query_state.updated",
                "active_state.updated",
                "settings.updated",
                "profiles.updated",
                "activity.events.updated",
                "activity.logs.updated",
                "doctor.updated",
                "health.updated"
            ]
        }),
    );
    let expected_topics = HashSet::from([
        "usage.updated".to_string(),
        "query_state.updated".to_string(),
        "active_state.updated".to_string(),
        "settings.updated".to_string(),
        "profiles.updated".to_string(),
        "activity.events.updated".to_string(),
        "activity.logs.updated".to_string(),
        "doctor.updated".to_string(),
        "health.updated".to_string(),
    ]);
    let mut startup_topics = HashSet::new();
    let mut saw_subscribe_response = false;
    let mut saw_startup_usage_refresh = false;
    let mut saw_startup_active_state = false;
    let mut saw_startup_query_pending = false;
    let mut saw_startup_query_clear = false;
    let subscribe_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < subscribe_deadline
        && (startup_topics.len() < expected_topics.len() || !saw_subscribe_response)
    {
        let message = daemon
            .read_message_timeout(Duration::from_millis(500))
            .expect("subscribe response or snapshot");
        if message["id"] == "2" {
            saw_subscribe_response = true;
            continue;
        }
        let topic = message["params"]["topic"].as_str().expect("topic");
        startup_topics.insert(topic.to_string());
        if topic == "usage.updated" && message["params"]["payload"]["trigger"] == "Startup" {
            saw_startup_usage_refresh = true;
        }
        if topic == "active_state.updated" {
            saw_startup_active_state = true;
        }
        if topic == "query_state.updated" {
            let states = message["params"]["payload"]["states"]
                .as_array()
                .expect("query states");
            if states.iter().any(|state| {
                state["key"]["kind"] == "UsageProfile"
                    && state["key"]["profile_id"] == profile_id
                    && state["status"] == "Pending"
                    && state["trigger"] == "Startup"
            }) {
                saw_startup_query_pending = true;
            }
            if saw_startup_query_pending && states.is_empty() {
                saw_startup_query_clear = true;
            }
        }
    }
    assert!(saw_subscribe_response, "expected subscribe response");
    assert_eq!(startup_topics, expected_topics);
    for topic in &expected_topics {
        assert!(startup_topics.contains(topic), "missing startup topic: {topic}");
    }

    let startup_refresh_deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < startup_refresh_deadline
        && (!saw_startup_usage_refresh
            || !saw_startup_active_state
            || !saw_startup_query_pending
            || !saw_startup_query_clear)
    {
        let message = daemon
            .read_message_timeout(Duration::from_millis(500))
            .expect("startup refresh update");
        match message["params"]["topic"].as_str().expect("topic") {
            "usage.updated" if message["params"]["payload"]["trigger"] == "Startup" => {
                saw_startup_usage_refresh = true;
            }
            "active_state.updated" => {
                saw_startup_active_state = true;
            }
            "query_state.updated" => {
                let states = message["params"]["payload"]["states"]
                    .as_array()
                    .expect("query states");
                if states.iter().any(|state| {
                    state["key"]["kind"] == "UsageProfile"
                        && state["key"]["profile_id"] == profile_id
                        && state["status"] == "Pending"
                        && state["trigger"] == "Startup"
                }) {
                    saw_startup_query_pending = true;
                }
                if saw_startup_query_pending && states.is_empty() {
                    saw_startup_query_clear = true;
                }
            }
            _ => {}
        }
    }
    assert!(saw_startup_usage_refresh, "expected startup usage refresh notification");
    assert!(saw_startup_active_state, "expected startup active_state notification");
    assert!(saw_startup_query_pending, "expected startup query pending");
    assert!(saw_startup_query_clear, "expected startup query clear");

    daemon.send_request(
        "3",
        "relay/usage/refresh",
        serde_json::json!({ "include_disabled": false }),
    );
    let mut saw_manual_usage_update = false;
    let mut saw_manual_active_state = false;
    let mut saw_manual_query_pending = false;
    let mut saw_manual_query_clear = false;
    let refresh_response_deadline = Instant::now() + Duration::from_secs(20);
    let refresh = loop {
        let Some(message) = daemon.read_message_timeout(Duration::from_millis(500)) else {
            assert!(
                Instant::now() < refresh_response_deadline,
                "timed out waiting for refresh response"
            );
            continue;
        };
        match message["params"]["topic"].as_str() {
            Some("usage.updated") if message["params"]["payload"]["trigger"] == "Manual" => {
                saw_manual_usage_update = true;
            }
            Some("query_state.updated") => {
                let states = message["params"]["payload"]["states"]
                    .as_array()
                    .expect("query states");
                if states.iter().any(|state| {
                    state["key"]["kind"] == "UsageProfile"
                        && state["key"]["profile_id"] == profile_id
                        && state["status"] == "Pending"
                        && state["trigger"] == "Manual"
                }) {
                    saw_manual_query_pending = true;
                }
                if saw_manual_query_pending && states.is_empty() {
                    saw_manual_query_clear = true;
                }
            }
            Some("active_state.updated") => {
                saw_manual_active_state = true;
            }
            _ => {}
        }
        if message["id"] == "3" {
            break message;
        }
    };
    assert_eq!(refresh["id"], "3");
    assert_eq!(
        refresh["result"]["snapshots"]
            .as_array()
            .expect("snapshots")
            .len(),
        1
    );
    let manual_refresh_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < manual_refresh_deadline
        && (!saw_manual_usage_update
            || !saw_manual_active_state
            || !saw_manual_query_pending
            || !saw_manual_query_clear)
    {
        let message = daemon
            .read_message_timeout(Duration::from_millis(500))
            .expect("manual refresh update");
        match message["params"]["topic"].as_str().expect("topic") {
            "usage.updated" if message["params"]["payload"]["trigger"] == "Manual" => {
                saw_manual_usage_update = true;
            }
            "query_state.updated" => {
                let states = message["params"]["payload"]["states"]
                    .as_array()
                    .expect("query states");
                if states.iter().any(|state| {
                    state["key"]["kind"] == "UsageProfile"
                        && state["key"]["profile_id"] == profile_id
                        && state["status"] == "Pending"
                        && state["trigger"] == "Manual"
                }) {
                    saw_manual_query_pending = true;
                }
                if saw_manual_query_pending && states.is_empty() {
                    saw_manual_query_clear = true;
                }
            }
            "active_state.updated" => {
                saw_manual_active_state = true;
            }
            _ => {}
        }
    }
    assert!(saw_manual_usage_update, "expected manual usage update");
    assert!(saw_manual_active_state, "expected manual active_state update");
    assert!(saw_manual_query_pending, "expected manual query pending");
    assert!(saw_manual_query_clear, "expected manual query clear");

    daemon.shutdown();
}

#[test]
fn daemon_stdio_settings_update_persists_all_app_fields_across_restart() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");

    let mut daemon = DaemonHarness::spawn(&relay_home, &live_codex_home);
    daemon.send_request(
        "1",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": false,
                "supports_health_updates": false
            }
        }),
    );
    let _ = daemon.read_message();

    daemon.send_request(
        "sub",
        "session/subscribe",
        serde_json::json!({
            "topics": ["settings.updated"]
        }),
    );
    let subscribe = daemon.read_message();
    assert_eq!(subscribe["id"], "sub");
    let settings_snapshot = daemon.read_message();
    assert_eq!(settings_snapshot["params"]["topic"], "settings.updated");
    assert_eq!(
        settings_snapshot["params"]["payload"]["settings"]["app"]["refresh_interval_seconds"],
        60
    );
    assert_eq!(
        settings_snapshot["params"]["payload"]["settings"]["app"]["network_query_concurrency"],
        10
    );

    daemon.send_request(
        "2",
        "relay/settings/update",
        serde_json::json!({
            "app": {
                "auto_switch_enabled": true,
                "cooldown_seconds": 321,
                "refresh_interval_seconds": 120,
                "network_query_concurrency": 16
            }
        }),
    );
    let updated = daemon.read_message();
    assert_eq!(updated["id"], "2");
    assert_eq!(updated["result"]["app"]["auto_switch_enabled"], true);
    assert_eq!(updated["result"]["app"]["cooldown_seconds"], 321);
    assert_eq!(updated["result"]["app"]["refresh_interval_seconds"], 120);
    assert_eq!(updated["result"]["app"]["network_query_concurrency"], 16);
    let settings_updated = daemon.read_message();
    assert_eq!(settings_updated["params"]["topic"], "settings.updated");
    assert_eq!(
        settings_updated["params"]["payload"]["settings"]["app"]["auto_switch_enabled"],
        true
    );
    assert_eq!(
        settings_updated["params"]["payload"]["settings"]["app"]["cooldown_seconds"],
        321
    );
    assert_eq!(
        settings_updated["params"]["payload"]["settings"]["app"]["refresh_interval_seconds"],
        120
    );
    assert_eq!(
        settings_updated["params"]["payload"]["settings"]["app"]["network_query_concurrency"],
        16
    );

    daemon.send_request("3", "relay/settings/get", serde_json::json!({}));
    let loaded = daemon.read_message();
    assert_eq!(loaded["id"], "3");
    assert_eq!(loaded["result"]["app"]["auto_switch_enabled"], true);
    assert_eq!(loaded["result"]["app"]["cooldown_seconds"], 321);
    assert_eq!(loaded["result"]["app"]["refresh_interval_seconds"], 120);
    assert_eq!(loaded["result"]["app"]["network_query_concurrency"], 16);

    daemon.shutdown();

    let mut restarted = DaemonHarness::spawn(&relay_home, &live_codex_home);
    restarted.send_request(
        "4",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": false,
                "supports_health_updates": false
            }
        }),
    );
    let _ = restarted.read_message();
    restarted.send_request("5", "relay/settings/get", serde_json::json!({}));
    let reloaded = restarted.read_message();
    assert_eq!(reloaded["id"], "5");
    assert_eq!(reloaded["result"]["app"]["auto_switch_enabled"], true);
    assert_eq!(reloaded["result"]["app"]["cooldown_seconds"], 321);
    assert_eq!(reloaded["result"]["app"]["refresh_interval_seconds"], 120);
    assert_eq!(reloaded["result"]["app"]["network_query_concurrency"], 16);

    restarted.shutdown();
}

#[test]
fn daemon_stdio_activity_and_doctor_refresh_publish_snapshot_updates() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");

    let mut daemon = DaemonHarness::spawn(&relay_home, &live_codex_home);
    daemon.send_request(
        "1",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": true,
                "supports_health_updates": true
            }
        }),
    );
    let _ = daemon.read_message();

    daemon.send_request(
        "sub",
        "session/subscribe",
        serde_json::json!({
            "topics": ["activity.events.updated", "activity.logs.updated", "doctor.updated"]
        }),
    );
    let subscribe = daemon.read_message();
    assert_eq!(subscribe["id"], "sub");

    let mut startup_topics = HashSet::new();
    while startup_topics.len() < 3 {
        let message = daemon.read_message();
        startup_topics.insert(
            message["params"]["topic"]
                .as_str()
                .expect("topic")
                .to_string(),
        );
    }
    assert!(startup_topics.contains("activity.events.updated"));
    assert!(startup_topics.contains("activity.logs.updated"));
    assert!(startup_topics.contains("doctor.updated"));

    daemon.send_request("activity", "relay/activity/refresh", serde_json::json!({}));
    let activity = daemon.read_message();
    assert_eq!(activity["id"], "activity");
    let mut saw_events = false;
    let mut saw_logs = false;
    while !saw_events || !saw_logs {
        let message = daemon.read_message();
        match message["params"]["topic"].as_str().expect("topic") {
            "activity.events.updated" => saw_events = true,
            "activity.logs.updated" => saw_logs = true,
            _ => {}
        }
    }

    daemon.send_request("doctor", "relay/doctor/refresh", serde_json::json!({}));
    let doctor = daemon.read_message();
    assert_eq!(doctor["id"], "doctor");
    let doctor_updated = daemon.read_message();
    assert_eq!(doctor_updated["params"]["topic"], "doctor.updated");

    daemon.shutdown();
}

#[test]
fn daemon_stdio_settings_update_recomputes_interval_deadline_immediately() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let profile_home = temp.path().join("profile-home");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&profile_home, "profile");

    let add_payload = format!(
        "{{\"nickname\":\"daemon-profile\",\"priority\":50,\"agent_home\":\"{}\",\"auth_mode\":\"ConfigFilesystem\"}}",
        profile_home.to_string_lossy()
    );
    let add = run_json_with_stdin(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "add", "--input-json", "-"],
        &add_payload,
    );
    let profile_id = add["data"]["id"].as_str().expect("profile id");

    let mut daemon = DaemonHarness::spawn(&relay_home, &live_codex_home);
    daemon.send_request(
        "1",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": true,
                "supports_health_updates": true
            }
        }),
    );
    let _ = daemon.read_message();

    daemon.send_request(
        "sub",
        "session/subscribe",
        serde_json::json!({
            "topics": ["query_state.updated"]
        }),
    );
    let subscribe = daemon.read_message();
    assert_eq!(subscribe["id"], "sub");
    let initial_query_snapshot = daemon.read_message();
    assert_eq!(initial_query_snapshot["params"]["topic"], "query_state.updated");
    assert_eq!(
        initial_query_snapshot["params"]["payload"]["states"]
            .as_array()
            .expect("query state snapshot")
            .len(),
        0
    );

    let mut saw_startup_pending = false;
    let startup_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < startup_deadline {
        let message = daemon
            .read_message_timeout(Duration::from_millis(500))
            .expect("startup refresh message");
        if message["params"]["topic"] != "query_state.updated" {
            continue;
        }

        let states = message["params"]["payload"]["states"]
            .as_array()
            .expect("query states");
        if states.iter().any(|state| {
            state["key"]["kind"] == "UsageProfile"
                && state["key"]["profile_id"] == profile_id
                && state["status"] == "Pending"
                && state["trigger"] == "Startup"
        }) {
            saw_startup_pending = true;
            continue;
        }
        if saw_startup_pending && states.is_empty() {
            break;
        }
    }
    assert!(saw_startup_pending, "expected startup refresh pending state");

    daemon.send_request(
        "2",
        "relay/settings/update",
        serde_json::json!({
            "app": {
                "refresh_interval_seconds": 15
            }
        }),
    );
    let updated = daemon.read_message();
    assert_eq!(updated["id"], "2");
    assert_eq!(updated["result"]["app"]["refresh_interval_seconds"], 15);

    let interval_start = Instant::now();
    let mut saw_interval_pending = false;
    while interval_start.elapsed() < Duration::from_secs(25) {
        let Some(message) = daemon.read_message_timeout(Duration::from_secs(1)) else {
            continue;
        };
        if message["params"]["topic"] != "query_state.updated" {
            continue;
        }

        let states = message["params"]["payload"]["states"]
            .as_array()
            .expect("query states");
        if states.iter().any(|state| {
            state["key"]["kind"] == "UsageProfile"
                && state["key"]["profile_id"] == profile_id
                && state["status"] == "Pending"
                && state["trigger"] == "Interval"
        }) {
            saw_interval_pending = true;
            break;
        }
    }

    assert!(
        saw_interval_pending,
        "expected interval refresh pending within 25 seconds after updating refresh interval"
    );

    daemon.shutdown();
}

#[test]
fn daemon_stdio_refresh_interval_off_disables_automatic_refresh_but_keeps_manual_refresh() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    let profile_home = temp.path().join("profile-home");
    make_codex_home(&live_codex_home, "live");
    make_codex_home(&profile_home, "profile");

    let add_payload = format!(
        "{{\"nickname\":\"daemon-profile\",\"priority\":50,\"agent_home\":\"{}\",\"auth_mode\":\"ConfigFilesystem\"}}",
        profile_home.to_string_lossy()
    );
    let add = run_json_with_stdin(
        &relay_home,
        &live_codex_home,
        &["--json", "codex", "add", "--input-json", "-"],
        &add_payload,
    );
    let profile_id = add["data"]["id"].as_str().expect("profile id");

    let mut daemon = DaemonHarness::spawn(&relay_home, &live_codex_home);
    daemon.send_request(
        "1",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": true,
                "supports_health_updates": true
            }
        }),
    );
    let _ = daemon.read_message();

    daemon.send_request(
        "2",
        "relay/settings/update",
        serde_json::json!({
            "app": {
                "refresh_interval_seconds": 0
            }
        }),
    );
    let updated = daemon.read_message();
    assert_eq!(updated["id"], "2");
    assert_eq!(updated["result"]["app"]["refresh_interval_seconds"], 0);

    daemon.send_request(
        "sub",
        "session/subscribe",
        serde_json::json!({
            "topics": ["query_state.updated"]
        }),
    );
    let subscribe = daemon.read_message();
    assert_eq!(subscribe["id"], "sub");
    let initial_query_snapshot = daemon.read_message();
    assert_eq!(initial_query_snapshot["params"]["topic"], "query_state.updated");
    assert_eq!(
        initial_query_snapshot["params"]["payload"]["states"]
            .as_array()
            .expect("query state snapshot")
            .len(),
        0
    );

    let unexpected = daemon.read_message_timeout(Duration::from_secs(2));
    assert!(
        unexpected.is_none(),
        "did not expect automatic refresh notifications when refresh interval is off"
    );

    daemon.send_request(
        "3",
        "relay/usage/refresh",
        serde_json::json!({
            "profile_id": profile_id
        }),
    );

    let mut saw_manual_pending = false;
    let mut saw_manual_clear = false;
    let manual_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < manual_deadline {
        let message = daemon
            .read_message_timeout(Duration::from_millis(500))
            .expect("manual refresh message");
        if message["id"] == "3" {
            continue;
        }
        if message["params"]["topic"] != "query_state.updated" {
            continue;
        }

        let states = message["params"]["payload"]["states"]
            .as_array()
            .expect("query states");
        if states.iter().any(|state| {
            state["key"]["kind"] == "UsageProfile"
                && state["key"]["profile_id"] == profile_id
                && state["status"] == "Pending"
                && state["trigger"] == "Manual"
        }) {
            saw_manual_pending = true;
            continue;
        }
        if saw_manual_pending && states.is_empty() {
            saw_manual_clear = true;
            break;
        }
    }

    assert!(saw_manual_pending, "expected manual refresh pending state");
    assert!(saw_manual_clear, "expected manual refresh clear state");

    daemon.shutdown();
}

#[test]
fn daemon_stdio_rejects_malformed_json_requests() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");

    let mut daemon = DaemonHarness::spawn(&relay_home, &live_codex_home);
    daemon.send_raw("{not-json");
    let error = daemon.read_message();
    assert!(error.get("id").is_none() || error["id"].is_null());
    assert_eq!(error["error"]["code"], -32600);

    daemon.shutdown();
}

#[test]
fn daemon_stdio_handles_high_concurrency_read_requests() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");

    let mut daemon = DaemonHarness::spawn(&relay_home, &live_codex_home);
    daemon.send_request(
        "init",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": false,
                "supports_health_updates": false
            }
        }),
    );
    let initialize = daemon.read_message();
    assert_eq!(initialize["id"], "init");

    const READ_REQUESTS: usize = 120;
    let mut expected_ids = HashSet::new();
    for idx in 0..READ_REQUESTS {
        let id = format!("read-{idx}");
        expected_ids.insert(id.clone());
        match idx % 3 {
            0 => daemon.send_request(&id, "relay/status/get", serde_json::json!({})),
            1 => daemon.send_request(&id, "relay/usage/get", serde_json::json!({})),
            _ => daemon.send_request(&id, "relay/profiles/list", serde_json::json!({})),
        }
    }

    let mut seen_ids = HashSet::new();
    for _ in 0..READ_REQUESTS {
        let response = daemon.read_message();
        assert!(
            response.get("error").is_none(),
            "read request failed: {response}"
        );
        let id = response["id"].as_str().expect("response id").to_string();
        assert!(expected_ids.contains(&id), "unexpected response id: {id}");
        assert!(seen_ids.insert(id), "duplicate response id");
    }

    assert_eq!(seen_ids.len(), READ_REQUESTS);
    daemon.shutdown();
}

#[test]
fn daemon_stdio_handles_high_concurrency_profile_switch_writes() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");

    let mut profile_ids = Vec::new();
    for idx in 0..3 {
        let profile_home = temp.path().join(format!("switch-profile-{idx}"));
        make_codex_home(&profile_home, &format!("profile-{idx}"));
        let add_payload = format!(
            "{{\"nickname\":\"switch-{idx}\",\"priority\":{},\"agent_home\":\"{}\",\"auth_mode\":\"ConfigFilesystem\"}}",
            100 + idx,
            profile_home.to_string_lossy()
        );
        let add = run_json_with_stdin(
            &relay_home,
            &live_codex_home,
            &["--json", "codex", "add", "--input-json", "-"],
            &add_payload,
        );
        profile_ids.push(
            add["data"]["id"]
                .as_str()
                .expect("profile id")
                .to_string(),
        );
    }

    let mut daemon = DaemonHarness::spawn(&relay_home, &live_codex_home);
    daemon.send_request(
        "init",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": false,
                "supports_health_updates": false
            }
        }),
    );
    let initialize = daemon.read_message();
    assert_eq!(initialize["id"], "init");

    const WRITE_REQUESTS: usize = 72;
    let mut expected_targets = HashMap::new();
    for idx in 0..WRITE_REQUESTS {
        let id = format!("switch-{idx}");
        let target = profile_ids[idx % profile_ids.len()].clone();
        expected_targets.insert(id.clone(), target.clone());
        daemon.send_request(
            &id,
            "relay/switch/activate",
            serde_json::json!({
                "profile_id": target,
            }),
        );
    }

    for _ in 0..WRITE_REQUESTS {
        let response = daemon.read_message();
        assert!(
            response.get("error").is_none(),
            "write request failed: {response}"
        );
        let id = response["id"].as_str().expect("response id").to_string();
        let expected_target = expected_targets
            .remove(&id)
            .unwrap_or_else(|| panic!("unexpected response id: {id}"));
        assert_eq!(response["result"]["profile_id"], expected_target);
    }
    assert!(
        expected_targets.is_empty(),
        "missing responses: {}",
        expected_targets.len()
    );

    daemon.send_request("status", "relay/status/get", serde_json::json!({}));
    let status = daemon.read_message();
    assert_eq!(status["id"], "status");
    let last_target = profile_ids[(WRITE_REQUESTS - 1) % profile_ids.len()].clone();
    assert_eq!(
        status["result"]["active_state"]["active_profile_id"],
        last_target
    );

    daemon.shutdown();
}

#[test]
fn daemon_stdio_long_profile_login_does_not_block_settings_update() {
    let temp = tempdir().expect("tempdir");
    let relay_home = temp.path().join("relay");
    let live_codex_home = temp.path().join("live-codex");
    make_codex_home(&live_codex_home, "live");
    let fake_bin = make_fake_bin(temp.path());
    let path_value = format!(
        "{}:{}",
        fake_bin.to_string_lossy(),
        std::env::var("PATH").unwrap_or_default()
    );

    let mut daemon = DaemonHarness::spawn_with_env(
        &relay_home,
        &live_codex_home,
        &[("PATH", path_value.as_str()), ("RELAY_TEST_LOGIN_SLEEP", "2")],
    );
    daemon.send_request(
        "init",
        "initialize",
        serde_json::json!({
            "protocol_version": "1",
            "client_info": { "name": "relay-cli-test", "version": "1.0.0" },
            "capabilities": {
                "supports_subscriptions": true,
                "supports_health_updates": false
            }
        }),
    );
    let initialize = daemon.read_message();
    assert_eq!(initialize["id"], "init");

    daemon.send_request(
        "sub",
        "session/subscribe",
        serde_json::json!({
            "topics": ["task.updated"]
        }),
    );
    let subscribe = daemon.read_message();
    assert_eq!(subscribe["id"], "sub");

    daemon.send_request(
        "login-start",
        "relay/profiles/login/start",
        serde_json::json!({
            "request": {
                "agent": "Codex",
                "nickname": "slow-login",
                "priority": 50,
                "mode": "Browser"
            }
        }),
    );
    let start_deadline = Instant::now() + Duration::from_secs(2);
    let mut task_id: Option<String> = None;
    let mut saw_pending = false;
    let mut pending_task_id: Option<String> = None;
    while Instant::now() < start_deadline && (task_id.is_none() || !saw_pending) {
        let message = daemon
            .read_message_timeout(Duration::from_millis(250))
            .expect("expected login start response or pending notification");
        if message["id"] == "login-start" {
            assert_eq!(message["result"]["accepted"], true);
            task_id = Some(
                message["result"]["task_id"]
                    .as_str()
                    .expect("task id")
                    .to_string(),
            );
            if pending_task_id.as_ref() == task_id.as_ref() {
                saw_pending = true;
            }
            continue;
        }
        if message["params"]["topic"] == "task.updated" {
            if message["params"]["payload"]["task"]["status"] == "Pending" {
                let update_task_id = message["params"]["payload"]["task"]["task_id"]
                    .as_str()
                    .expect("pending task id")
                    .to_string();
                pending_task_id = Some(update_task_id.clone());
                if task_id.as_deref() == Some(update_task_id.as_str()) {
                    saw_pending = true;
                }
            }
        }
    }
    let task_id = task_id.expect("task id");
    assert!(saw_pending, "expected pending task notification");

    daemon.send_request(
        "settings",
        "relay/settings/update",
        serde_json::json!({
            "app": {
                "refresh_interval_seconds": 15
            }
        }),
    );

    let first_response = daemon
        .read_message_timeout(Duration::from_secs(1))
        .expect("expected first response");
    assert_eq!(
        first_response["id"], "settings",
        "settings/update should not wait for slow profile login"
    );
    assert_eq!(first_response["result"]["app"]["refresh_interval_seconds"], 15);

    daemon.send_request(
        "cancel",
        "relay/tasks/cancel",
        serde_json::json!({
            "task_id": task_id
        }),
    );
    let cancel_response = daemon
        .read_message_timeout(Duration::from_secs(1))
        .expect("expected cancel response");
    assert_eq!(cancel_response["id"], "cancel");
    assert_eq!(cancel_response["result"]["accepted"], true);

    let cancelled = daemon
        .read_message_timeout(Duration::from_secs(5))
        .expect("expected cancelled task notification");
    assert_eq!(cancelled["params"]["topic"], "task.updated");
    assert_eq!(cancelled["params"]["payload"]["task"]["task_id"], task_id);
    assert_eq!(cancelled["params"]["payload"]["task"]["status"], "Cancelled");

    daemon.shutdown();
}
