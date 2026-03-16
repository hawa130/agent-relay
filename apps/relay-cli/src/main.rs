use chrono::{DateTime, Local, Utc};
use clap::{Args, Parser, Subcommand};
use comfy_table::{
    Attribute, Cell, CellAlignment, ContentArrangement, Row, Table, modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL,
};
use relay_core::models::JsonResponse;
use relay_core::{
    ActiveState, ActivityEventsQuery, AddProfileRequest, AgentKind, AgentLinkResult,
    AgentLoginMode, AgentLoginRequest, AppSettings, AuthMode, BootstrapMode, CodexSettings,
    CodexSettingsUpdateRequest, DiagnosticsExport, DoctorReport, EditProfileRequest, FailureEvent,
    FailureReason, ImportProfileRequest, LogTail, ProbeProvider, Profile, ProfileDetail,
    ProfileProbeIdentity, ProfileRecoveryReport, RelayApp, RelayError, StatusReport, SwitchOutcome,
    SwitchReport, SystemSettingsUpdateRequest, UsageSnapshot, UsageSourceMode, UsageStatus,
    UsageWindow,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tracing_subscriber::EnvFilter;

mod daemon;
mod dispatch;
mod render;

#[derive(Debug, Parser)]
#[command(
    name = "agrelay",
    version,
    about = "AgentRelay - local coding agent profile orchestrator"
)]
struct Cli {
    #[arg(long, global = true, help = "Emit machine-readable JSON output")]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Show current AgentRelay state and active profile")]
    Status,
    #[command(about = "List managed profiles with usage summaries")]
    List,
    #[command(about = "Inspect one profile, or the current profile when omitted")]
    Show(ShowCommand),
    #[command(about = "Activate a profile, or switch to the next eligible one")]
    Switch(SwitchCommand),
    #[command(about = "Refresh usage data for one or more profiles")]
    Refresh(RefreshCommand),
    #[command(about = "Manage Codex profiles, login flows, and settings")]
    Codex(CodexCommand),
    #[command(about = "Inspect AgentRelay events, logs, and diagnostics")]
    Activity(ActivityCommand),
    #[command(about = "Inspect AgentRelay environment, paths, and binary health")]
    Doctor,
    #[command(about = "Run the AgentRelay daemon over stdio JSON-RPC")]
    Daemon(DaemonCommand),
    #[command(about = "Inspect AgentRelay settings")]
    Settings(SettingsCommand),
    #[command(about = "Inspect or change automatic switching behavior")]
    Autoswitch(AutoswitchCommand),
    #[command(about = "Update profile metadata or managed paths")]
    Edit(EditProfileArgs),
    #[command(about = "Enable a profile for switching and usage refresh")]
    Enable(ProfileIdArgs),
    #[command(about = "Disable a profile from switching and usage refresh")]
    Disable(ProfileIdArgs),
    #[command(about = "Remove a managed profile")]
    Remove(ProfileIdArgs),
}

#[derive(Debug, Args)]
struct DaemonCommand {
    #[arg(long, required = true, help = "Serve the daemon over stdio JSON-RPC")]
    stdio: bool,
}

#[derive(Debug, Args)]
#[command(about = "Inspect AgentRelay settings")]
struct SettingsCommand {
    #[command(subcommand)]
    command: Option<SettingsSubcommand>,
}

#[derive(Debug, Subcommand)]
enum SettingsSubcommand {
    #[command(about = "Show current AgentRelay settings")]
    Show,
    #[command(about = "Update AgentRelay settings")]
    Set(SettingsSetArgs),
}

#[derive(Debug, Args)]
struct SettingsSetArgs {
    #[arg(long, help = "Enable or disable automatic switching")]
    auto_switch_enabled: Option<bool>,
    #[arg(long, help = "Set the cooldown in seconds")]
    cooldown_seconds: Option<i64>,
    #[arg(
        long,
        help = "Set automatic usage refresh interval in seconds, or 0 to disable"
    )]
    refresh_interval_seconds: Option<i64>,
    #[arg(long, help = "Set the maximum number of concurrent network queries")]
    network_query_concurrency: Option<i64>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ShowCommand {
    #[arg(help = "Profile ID to inspect. Uses the current profile when omitted")]
    target: Option<String>,
}

#[derive(Debug, Args)]
struct SwitchCommand {
    #[arg(help = "Profile ID to activate. Uses the next eligible profile when omitted")]
    target: Option<String>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct RefreshCommand {
    #[arg(help = "Profile ID to refresh. Refreshes enabled profiles when omitted")]
    target: Option<String>,
    #[arg(long, help = "Refresh every profile, including disabled ones")]
    all: bool,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
#[command(about = "Inspect or change automatic switching behavior")]
struct AutoswitchCommand {
    #[command(subcommand)]
    command: Option<AutoswitchSubcommand>,
}

#[derive(Debug, Subcommand)]
enum AutoswitchSubcommand {
    #[command(about = "Show current automatic switching settings")]
    Show,
    #[command(about = "Turn automatic switching on")]
    Enable,
    #[command(about = "Turn automatic switching off")]
    Disable,
    #[command(about = "Set automatic switching explicitly with a boolean value")]
    Set(AutoswitchSetArgs),
}

#[derive(Debug, Args)]
struct ProfileIdArgs {
    #[arg(help = "Profile ID to operate on")]
    id: Option<String>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct AutoswitchSetArgs {
    #[arg(long, help = "Enable or disable automatic switching")]
    enabled: Option<bool>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct SettingsSetInput {
    auto_switch_enabled: Option<bool>,
    cooldown_seconds: Option<i64>,
    refresh_interval_seconds: Option<i64>,
    network_query_concurrency: Option<i64>,
}

#[derive(Debug, Args)]
#[command(about = "Inspect AgentRelay events, logs, and diagnostics")]
struct ActivityCommand {
    #[command(subcommand)]
    command: ActivitySubcommand,
}

#[derive(Debug, Subcommand)]
enum ActivitySubcommand {
    #[command(about = "Inspect recorded switch failures and cooldowns")]
    Events(ActivityEventsCommand),
    #[command(about = "Read AgentRelay log output")]
    Logs(LogsCommand),
    #[command(about = "Export a diagnostic bundle for debugging")]
    Diagnostics(DiagnosticsCommand),
}

#[derive(Debug, Args)]
#[command(about = "Inspect recorded switch failures and cooldowns")]
struct ActivityEventsCommand {
    #[command(subcommand)]
    command: ActivityEventsSubcommand,
}

#[derive(Debug, Subcommand)]
enum ActivityEventsSubcommand {
    #[command(about = "List recent failure events")]
    List(ActivityEventsListArgs),
}

#[derive(Debug, Args)]
#[command(about = "Read AgentRelay log output")]
struct LogsCommand {
    #[command(subcommand)]
    command: LogsSubcommand,
}

#[derive(Debug, Args)]
#[command(about = "Manage Codex profiles, login flows, and settings")]
struct CodexCommand {
    #[command(subcommand)]
    command: CodexSubcommand,
}

#[derive(Debug, Subcommand)]
enum CodexSubcommand {
    #[command(about = "Create a new profile by signing in with Codex")]
    Login(CodexLoginArgs),
    #[command(about = "Import the current live Codex home as a managed profile")]
    Import(CodexImportArgs),
    #[command(about = "Register an existing Codex home or config as a profile")]
    Add(CodexAddArgs),
    #[command(about = "Recover saved Codex profile snapshots into the database")]
    Recover,
    #[command(about = "Refresh a profile's linked Codex identity from the live home")]
    Relink(ProfileIdArgs),
    #[command(about = "Inspect or update Codex-wide settings")]
    Settings(CodexSettingsCommand),
}

#[derive(Debug, Args)]
#[command(about = "Inspect or update Codex-wide settings")]
struct CodexSettingsCommand {
    #[command(subcommand)]
    command: Option<CodexSettingsSubcommand>,
}

#[derive(Debug, Subcommand)]
enum CodexSettingsSubcommand {
    #[command(about = "Show current Codex settings")]
    Show,
    #[command(about = "Update Codex settings")]
    Set(CodexSettingsSetArgs),
}

#[derive(Debug, Args)]
struct CodexSettingsSetArgs {
    #[arg(long, help = "Usage source mode for Codex profiles")]
    source_mode: Option<String>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct CodexAddArgs {
    #[arg(long, help = "Display name for the new profile")]
    nickname: Option<String>,
    #[arg(long, help = "Lower numbers are preferred during switching")]
    priority: Option<i32>,
    #[arg(long, help = "Path to the Codex config file to manage")]
    config_path: Option<PathBuf>,
    #[arg(long, help = "Path to the Codex home directory to manage")]
    agent_home: Option<PathBuf>,
    #[arg(long, help = "Authentication storage mode for the profile")]
    auth_mode: Option<String>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct CodexImportArgs {
    #[arg(long, help = "Display name for the imported profile")]
    nickname: Option<String>,
    #[arg(long, help = "Lower numbers are preferred during switching")]
    priority: Option<i32>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct CodexLoginArgs {
    #[arg(long, help = "Display name for the new profile")]
    nickname: Option<String>,
    #[arg(long, help = "Lower numbers are preferred during switching")]
    priority: Option<i32>,
    #[arg(
        long,
        visible_alias = "headless",
        help = "Use device code auth instead of opening a browser"
    )]
    device_auth: bool,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum LogsSubcommand {
    #[command(about = "Show the latest log lines")]
    Tail(TailArgs),
}

#[derive(Debug, Args)]
#[command(about = "Export a diagnostic bundle for debugging")]
struct DiagnosticsCommand {
    #[command(subcommand)]
    command: DiagnosticsSubcommand,
}

#[derive(Debug, Subcommand)]
enum DiagnosticsSubcommand {
    #[command(about = "Create a diagnostics archive")]
    Export,
}

#[derive(Debug, Args)]
struct EditProfileArgs {
    #[arg(help = "Profile ID to edit")]
    id: Option<String>,
    #[arg(long, help = "New display name for the profile")]
    nickname: Option<String>,
    #[arg(long, help = "Lower numbers are preferred during switching")]
    priority: Option<i32>,
    #[arg(long, help = "New path to the managed Codex config file")]
    config_path: Option<PathBuf>,
    #[arg(long, help = "Clear the saved config path")]
    clear_config_path: bool,
    #[arg(long, help = "New path to the managed Codex home directory")]
    agent_home: Option<PathBuf>,
    #[arg(long, help = "Clear the saved agent home path")]
    clear_agent_home: bool,
    #[arg(long, help = "New authentication storage mode for the profile")]
    auth_mode: Option<String>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ActivityEventsListArgs {
    #[arg(long, help = "Maximum number of events to return")]
    limit: Option<usize>,
    #[arg(long, help = "Filter events to a specific profile ID")]
    profile_id: Option<String>,
    #[arg(long, help = "Filter events to a failure reason")]
    reason: Option<String>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct TailArgs {
    #[arg(long, help = "Number of log lines to show")]
    lines: Option<usize>,
    #[arg(long, help = "Read command arguments from JSON file or stdin (-)")]
    input_json: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = init_tracing() {
        eprintln!("failed to initialize tracing: {error}");
        return ExitCode::FAILURE;
    }

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn init_tracing() -> Result<(), String> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("agrelay=info,relay_core=info,warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init()
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn run() -> Result<(), RelayError> {
    let cli = Cli::parse();
    if let Commands::Daemon(command) = &cli.command {
        return daemon::run(command).await;
    }
    let json = cli.json;

    match dispatch::execute(cli).await {
        Ok(handled) => handled.write(),
        Err(error) => {
            if json {
                JsonResponse::<()>::error(&error).write_json()?;
            } else {
                eprintln!("{error}");
            }
            Err(error)
        }
    }
}

fn parse_auth_mode(value: &str) -> Result<AuthMode, RelayError> {
    match value {
        "config-filesystem" => Ok(AuthMode::ConfigFilesystem),
        "env-reference" => Ok(AuthMode::EnvReference),
        "keychain-reference" => Ok(AuthMode::KeychainReference),
        other => Err(RelayError::InvalidInput(format!(
            "unsupported auth mode: {other}"
        ))),
    }
}

fn codex_add_request_from_args(args: CodexAddArgs) -> Result<AddProfileRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.nickname.is_some(),
                args.priority.is_some(),
                args.config_path.is_some(),
                args.agent_home.is_some(),
                args.auth_mode.is_some(),
            ],
        )?;
        let payload: CodexAddInput = read_json_input(input_json)?;
        return Ok(AddProfileRequest {
            agent: AgentKind::Codex,
            nickname: payload.nickname,
            priority: payload.priority,
            config_path: payload.config_path,
            agent_home: payload.agent_home,
            auth_mode: payload.auth_mode,
        });
    }

    Ok(AddProfileRequest {
        agent: AgentKind::Codex,
        nickname: require_field(args.nickname, "profile nickname is required")?,
        priority: args.priority.unwrap_or(100),
        config_path: args.config_path,
        agent_home: args.agent_home,
        auth_mode: args
            .auth_mode
            .as_deref()
            .map(parse_auth_mode)
            .transpose()?
            .unwrap_or(AuthMode::ConfigFilesystem),
    })
}

fn edit_profile_request_from_args(
    args: EditProfileArgs,
) -> Result<(String, EditProfileRequest), RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.id.is_some(),
                args.nickname.is_some(),
                args.priority.is_some(),
                args.config_path.is_some(),
                args.clear_config_path,
                args.agent_home.is_some(),
                args.clear_agent_home,
                args.auth_mode.is_some(),
            ],
        )?;
        let payload: EditProfileInput = read_json_input(input_json)?;
        return Ok((
            payload.id,
            EditProfileRequest {
                nickname: payload.nickname,
                priority: payload.priority,
                config_path: payload.config_path,
                agent_home: payload.agent_home,
                auth_mode: payload.auth_mode,
            },
        ));
    }

    let auth_mode = args.auth_mode.as_deref().map(parse_auth_mode).transpose()?;
    Ok((
        require_field(args.id, "profile id is required")?,
        EditProfileRequest {
            nickname: args.nickname,
            priority: args.priority,
            config_path: if args.clear_config_path {
                Some(None)
            } else {
                args.config_path.map(Some)
            },
            agent_home: if args.clear_agent_home {
                Some(None)
            } else {
                args.agent_home.map(Some)
            },
            auth_mode,
        },
    ))
}

fn codex_import_request_from_args(
    args: CodexImportArgs,
) -> Result<ImportProfileRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[args.nickname.is_some(), args.priority.is_some()],
        )?;
        let payload: ImportProfileInput = read_json_input(input_json)?;
        return Ok(ImportProfileRequest {
            agent: AgentKind::Codex,
            nickname: payload.nickname,
            priority: payload.priority,
        });
    }

    Ok(ImportProfileRequest {
        agent: AgentKind::Codex,
        nickname: args.nickname,
        priority: args.priority.unwrap_or(100),
    })
}

fn codex_login_request_from_args(args: CodexLoginArgs) -> Result<AgentLoginRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.nickname.is_some(),
                args.priority.is_some(),
                args.device_auth,
            ],
        )?;
        let payload: LoginProfileInput = read_json_input(input_json)?;
        return Ok(AgentLoginRequest {
            agent: AgentKind::Codex,
            nickname: payload.nickname,
            priority: payload.priority,
            mode: if payload.device_auth {
                AgentLoginMode::DeviceAuth
            } else {
                AgentLoginMode::Browser
            },
        });
    }

    Ok(AgentLoginRequest {
        agent: AgentKind::Codex,
        nickname: args.nickname,
        priority: args.priority.unwrap_or(100),
        mode: if args.device_auth {
            AgentLoginMode::DeviceAuth
        } else {
            AgentLoginMode::Browser
        },
    })
}

fn profile_id_from_args(args: ProfileIdArgs) -> Result<String, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.id.is_some()])?;
        let payload: ProfileIdInput = read_json_input(input_json)?;
        return Ok(payload.id);
    }

    require_field(args.id, "profile id is required")
}

fn system_settings_request_from_args(
    args: AutoswitchSetArgs,
) -> Result<SystemSettingsUpdateRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.enabled.is_some()])?;
        let payload: AutoSwitchInput = read_json_input(input_json)?;
        return Ok(SystemSettingsUpdateRequest {
            auto_switch_enabled: Some(payload.enabled),
            cooldown_seconds: None,
            refresh_interval_seconds: None,
            network_query_concurrency: None,
        });
    }

    Ok(SystemSettingsUpdateRequest {
        auto_switch_enabled: Some(require_field(
            args.enabled,
            "autoswitch enabled value is required",
        )?),
        cooldown_seconds: None,
        refresh_interval_seconds: None,
        network_query_concurrency: None,
    })
}

fn settings_request_from_args(
    args: SettingsSetArgs,
) -> Result<SystemSettingsUpdateRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.auto_switch_enabled.is_some(),
                args.cooldown_seconds.is_some(),
                args.refresh_interval_seconds.is_some(),
                args.network_query_concurrency.is_some(),
            ],
        )?;
        let payload: SettingsSetInput = read_json_input(input_json)?;
        return Ok(SystemSettingsUpdateRequest {
            auto_switch_enabled: payload.auto_switch_enabled,
            cooldown_seconds: payload.cooldown_seconds,
            refresh_interval_seconds: payload.refresh_interval_seconds,
            network_query_concurrency: payload.network_query_concurrency,
        });
    }

    let request = SystemSettingsUpdateRequest {
        auto_switch_enabled: args.auto_switch_enabled,
        cooldown_seconds: args.cooldown_seconds,
        refresh_interval_seconds: args.refresh_interval_seconds,
        network_query_concurrency: args.network_query_concurrency,
    };
    if request.auto_switch_enabled.is_none()
        && request.cooldown_seconds.is_none()
        && request.refresh_interval_seconds.is_none()
        && request.network_query_concurrency.is_none()
    {
        return Err(RelayError::InvalidInput(
            "at least one settings field must be provided".into(),
        ));
    }
    Ok(request)
}

enum ShowTarget {
    Current,
    Profile(String),
}

fn show_target_from_args(args: ShowCommand) -> Result<ShowTarget, RelayError> {
    match args.target.as_deref() {
        None | Some("current") => Ok(ShowTarget::Current),
        Some(id) => Ok(ShowTarget::Profile(id.to_string())),
    }
}

enum SwitchTarget {
    Next,
    Profile(String),
}

fn switch_target_from_args(args: SwitchCommand) -> Result<SwitchTarget, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.target.is_some()])?;
        let payload: ProfileIdInput = read_json_input(input_json)?;
        return Ok(SwitchTarget::Profile(payload.id));
    }

    match args.target.as_deref() {
        None | Some("next") => Ok(SwitchTarget::Next),
        Some(id) => Ok(SwitchTarget::Profile(id.to_string())),
    }
}

enum RefreshTarget {
    Profile(String),
    Enabled,
    All,
}

fn refresh_target_from_args(args: RefreshCommand) -> Result<RefreshTarget, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.target.is_some(), args.all])?;
        let payload: RefreshInput = read_json_input(input_json)?;
        return refresh_target_from_input(payload);
    }

    refresh_target_from_input(RefreshInput {
        id: args.target,
        all: args.all,
    })
}

fn refresh_target_from_input(input: RefreshInput) -> Result<RefreshTarget, RelayError> {
    if input.all && input.id.is_some() {
        return Err(RelayError::InvalidInput(
            "refresh cannot combine a profile id with --all".into(),
        ));
    }

    if input.all {
        Ok(RefreshTarget::All)
    } else if let Some(id) = input.id {
        Ok(RefreshTarget::Profile(id))
    } else {
        Ok(RefreshTarget::Enabled)
    }
}

fn parse_usage_source_mode(value: &str) -> Result<UsageSourceMode, RelayError> {
    match value {
        "auto" | "Auto" => Ok(UsageSourceMode::Auto),
        "local" | "Local" => Ok(UsageSourceMode::Local),
        "web-enhanced" | "web_enhanced" | "web" | "WebEnhanced" => Ok(UsageSourceMode::WebEnhanced),
        other => Err(RelayError::InvalidInput(format!(
            "unsupported usage source mode: {other}"
        ))),
    }
}

fn parse_failure_reason(value: &str) -> Result<FailureReason, RelayError> {
    value
        .parse::<FailureReason>()
        .map_err(RelayError::InvalidInput)
}

fn codex_settings_request_from_args(
    args: CodexSettingsSetArgs,
) -> Result<CodexSettingsUpdateRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.source_mode.is_some()])?;
        let payload: CodexSettingsSetInput = read_json_input(input_json)?;
        return Ok(CodexSettingsUpdateRequest {
            usage_source_mode: payload.source_mode,
        });
    }

    Ok(CodexSettingsUpdateRequest {
        usage_source_mode: args
            .source_mode
            .as_deref()
            .map(parse_usage_source_mode)
            .transpose()?,
    })
}

fn activity_events_query_from_args(
    args: ActivityEventsListArgs,
) -> Result<ActivityEventsQuery, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.limit.is_some(),
                args.profile_id.is_some(),
                args.reason.is_some(),
            ],
        )?;
        let payload: EventsListInput = read_json_input(input_json)?;
        return Ok(ActivityEventsQuery {
            limit: payload.limit,
            profile_id: payload.profile_id,
            reason: payload.reason,
        });
    }

    Ok(ActivityEventsQuery {
        limit: args.limit.unwrap_or(50),
        profile_id: args.profile_id,
        reason: args
            .reason
            .as_deref()
            .map(parse_failure_reason)
            .transpose()?,
    })
}

fn log_lines_from_args(args: TailArgs) -> Result<usize, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.lines.is_some()])?;
        let payload: LogsTailInput = read_json_input(input_json)?;
        return Ok(payload.lines);
    }

    Ok(args.lines.unwrap_or(50))
}

fn ensure_json_input_is_exclusive(path: &Path, conflicts: &[bool]) -> Result<(), RelayError> {
    if conflicts.iter().any(|value| *value) {
        return Err(RelayError::InvalidInput(format!(
            "--input-json {} cannot be combined with inline command arguments",
            path.display()
        )));
    }
    Ok(())
}

fn read_json_input<T>(path: &Path) -> Result<T, RelayError>
where
    T: for<'de> Deserialize<'de>,
{
    let body = if path.as_os_str() == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|error| RelayError::Io(error.to_string()))?;
        buffer
    } else {
        fs::read_to_string(path)?
    };

    serde_json::from_str(&body).map_err(|error| {
        RelayError::InvalidInput(format!("invalid JSON input in {}: {error}", path.display()))
    })
}

fn require_field<T>(value: Option<T>, message: &str) -> Result<T, RelayError> {
    value.ok_or_else(|| RelayError::InvalidInput(message.into()))
}

fn default_priority() -> i32 {
    100
}

fn default_auth_mode() -> AuthMode {
    AuthMode::ConfigFilesystem
}

#[derive(Debug, Deserialize)]
struct CodexAddInput {
    nickname: String,
    #[serde(default = "default_priority")]
    priority: i32,
    config_path: Option<PathBuf>,
    agent_home: Option<PathBuf>,
    #[serde(default = "default_auth_mode")]
    auth_mode: AuthMode,
}

#[derive(Debug, Deserialize)]
struct EditProfileInput {
    id: String,
    nickname: Option<String>,
    priority: Option<i32>,
    config_path: Option<Option<PathBuf>>,
    agent_home: Option<Option<PathBuf>>,
    auth_mode: Option<AuthMode>,
}

#[derive(Debug, Deserialize)]
struct ProfileIdInput {
    id: String,
}

#[derive(Debug, Deserialize)]
struct AutoSwitchInput {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexSettingsSetInput {
    #[serde(default, deserialize_with = "deserialize_usage_source_mode_option")]
    source_mode: Option<UsageSourceMode>,
}

#[derive(Debug, Deserialize)]
struct LoginProfileInput {
    nickname: Option<String>,
    #[serde(default = "default_priority")]
    priority: i32,
    #[serde(default)]
    device_auth: bool,
}

fn deserialize_usage_source_mode_option<'de, D>(
    deserializer: D,
) -> Result<Option<UsageSourceMode>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .as_deref()
        .map(parse_usage_source_mode)
        .transpose()
        .map_err(serde::de::Error::custom)
}

fn deserialize_failure_reason_option<'de, D>(
    deserializer: D,
) -> Result<Option<FailureReason>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .as_deref()
        .map(parse_failure_reason)
        .transpose()
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct ImportProfileInput {
    nickname: Option<String>,
    #[serde(default = "default_priority")]
    priority: i32,
}

#[derive(Debug, Deserialize)]
struct RefreshInput {
    id: Option<String>,
    #[serde(default)]
    all: bool,
}

#[derive(Debug, Deserialize)]
struct EventsListInput {
    #[serde(default = "default_list_limit")]
    limit: usize,
    profile_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_failure_reason_option")]
    reason: Option<FailureReason>,
}

#[derive(Debug, Deserialize)]
struct LogsTailInput {
    #[serde(default = "default_tail_lines")]
    lines: usize,
}

fn default_list_limit() -> usize {
    50
}

fn default_tail_lines() -> usize {
    50
}
