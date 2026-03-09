use chrono::{DateTime, Local, Utc};
use clap::{Args, Parser, Subcommand};
use comfy_table::{
    Attribute, Cell, CellAlignment, ContentArrangement, Row, Table, modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL,
};
use relay_core::models::JsonResponse;
use relay_core::{
    AddProfileRequest, AgentKind, AgentLoginRequest, AuthMode, BootstrapMode, EditProfileRequest,
    ImportProfileRequest, Profile, RelayApp, RelayError, UsageConfidence,
    UsageSettingsUpdateRequest, UsageSnapshot, UsageSourceMode, UsageStatus, UsageWindow,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "relay",
    version,
    about = "Local coding agent profile orchestrator"
)]
struct Cli {
    #[arg(long, global = true, help = "Emit machine-readable JSON output")]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Doctor,
    Status,
    Usage(UsageCommand),
    Profiles(ProfilesCommand),
    Switch(SwitchCommand),
    AutoSwitch(AutoSwitchCommand),
    Events(EventsCommand),
    Logs(LogsCommand),
    Diagnostics(DiagnosticsCommand),
}

#[derive(Debug, Args)]
struct ProfilesCommand {
    #[command(subcommand)]
    command: ProfilesSubcommand,
}

#[derive(Debug, Subcommand)]
enum ProfilesSubcommand {
    List,
    Add(AddProfileArgs),
    Edit(EditProfileArgs),
    Remove(ProfileIdArgs),
    Enable(ProfileIdArgs),
    Disable(ProfileIdArgs),
    Import(ImportProfileArgs),
    Login(LoginProfileArgs),
    Relink(AgentProfileIdArgs),
}

#[derive(Debug, Args)]
struct AddProfileArgs {
    agent: Option<String>,
    #[arg(long)]
    nickname: Option<String>,
    #[arg(long)]
    priority: Option<i32>,
    #[arg(long)]
    config_path: Option<PathBuf>,
    #[arg(long)]
    agent_home: Option<PathBuf>,
    #[arg(long)]
    auth_mode: Option<String>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ProfileIdArgs {
    id: Option<String>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct AgentProfileIdArgs {
    agent: Option<String>,
    id: Option<String>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct SwitchCommand {
    target: Option<String>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct AutoSwitchCommand {
    #[command(subcommand)]
    command: AutoSwitchSubcommand,
}

#[derive(Debug, Subcommand)]
enum AutoSwitchSubcommand {
    Enable,
    Disable,
    Set(AutoSwitchSetArgs),
}

#[derive(Debug, Args)]
struct AutoSwitchSetArgs {
    #[arg(long)]
    enabled: Option<bool>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct EventsCommand {
    #[command(subcommand)]
    command: EventsSubcommand,
}

#[derive(Debug, Subcommand)]
enum EventsSubcommand {
    List(ListArgs),
}

#[derive(Debug, Args)]
struct LogsCommand {
    #[command(subcommand)]
    command: LogsSubcommand,
}

#[derive(Debug, Args)]
struct UsageCommand {
    #[command(subcommand)]
    command: Option<UsageSubcommand>,
}

#[derive(Debug, Subcommand)]
enum UsageSubcommand {
    Current,
    Profile(ProfileIdArgs),
    List,
    Refresh(UsageRefreshArgs),
    Config(UsageConfigCommand),
}

#[derive(Debug, Args)]
struct UsageRefreshArgs {
    id: Option<String>,
    #[arg(long)]
    enabled: bool,
    #[arg(long)]
    all: bool,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct UsageConfigCommand {
    #[command(subcommand)]
    command: Option<UsageConfigSubcommand>,
}

#[derive(Debug, Subcommand)]
enum UsageConfigSubcommand {
    Set(UsageConfigSetArgs),
}

#[derive(Debug, Args)]
struct UsageConfigSetArgs {
    #[arg(long)]
    source_mode: Option<String>,
    #[arg(long)]
    menu_open_refresh_stale_after_seconds: Option<i64>,
    #[arg(long)]
    background_refresh_enabled: Option<bool>,
    #[arg(long)]
    background_refresh_interval_seconds: Option<i64>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum LogsSubcommand {
    Tail(TailArgs),
}

#[derive(Debug, Args)]
struct DiagnosticsCommand {
    #[command(subcommand)]
    command: DiagnosticsSubcommand,
}

#[derive(Debug, Subcommand)]
enum DiagnosticsSubcommand {
    Export,
}

#[derive(Debug, Args)]
struct ImportProfileArgs {
    agent: Option<String>,
    #[arg(long)]
    nickname: Option<String>,
    #[arg(long)]
    priority: Option<i32>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct LoginProfileArgs {
    agent: Option<String>,
    #[arg(long)]
    nickname: Option<String>,
    #[arg(long)]
    priority: Option<i32>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct EditProfileArgs {
    id: Option<String>,
    #[arg(long)]
    nickname: Option<String>,
    #[arg(long)]
    priority: Option<i32>,
    #[arg(long)]
    config_path: Option<PathBuf>,
    #[arg(long)]
    clear_config_path: bool,
    #[arg(long)]
    agent_home: Option<PathBuf>,
    #[arg(long)]
    clear_agent_home: bool,
    #[arg(long)]
    auth_mode: Option<String>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct TailArgs {
    #[arg(long)]
    lines: Option<usize>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

fn main() -> ExitCode {
    if let Err(error) = init_tracing() {
        eprintln!("failed to initialize tracing: {error}");
        return ExitCode::FAILURE;
    }

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn init_tracing() -> Result<(), String> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .try_init()
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn run() -> Result<(), RelayError> {
    let cli = Cli::parse();
    let json = cli.json;

    match execute(cli) {
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

fn execute(cli: Cli) -> Result<Output, RelayError> {
    let bootstrap_mode = match &cli.command {
        Commands::Doctor | Commands::Status => BootstrapMode::ReadOnly,
        Commands::Usage(command) => match command.command {
            None
            | Some(UsageSubcommand::Current)
            | Some(UsageSubcommand::Profile(_))
            | Some(UsageSubcommand::List)
            | Some(UsageSubcommand::Config(UsageConfigCommand { command: None })) => {
                BootstrapMode::ReadOnly
            }
            Some(UsageSubcommand::Refresh(_))
            | Some(UsageSubcommand::Config(UsageConfigCommand {
                command: Some(UsageConfigSubcommand::Set(_)),
            })) => BootstrapMode::ReadWrite,
        },
        Commands::Profiles(command) => match command.command {
            ProfilesSubcommand::List => BootstrapMode::ReadOnly,
            _ => BootstrapMode::ReadWrite,
        },
        Commands::Events(command) => match command.command {
            EventsSubcommand::List(_) => BootstrapMode::ReadOnly,
        },
        Commands::Logs(command) => match command.command {
            LogsSubcommand::Tail(_) => BootstrapMode::ReadOnly,
        },
        Commands::Switch(_) | Commands::AutoSwitch(_) | Commands::Diagnostics(_) => {
            BootstrapMode::ReadWrite
        }
    };
    let app = RelayApp::bootstrap_with_mode(bootstrap_mode)?;
    dispatch(cli, app)
}

fn dispatch(cli: Cli, app: RelayApp) -> Result<Output, RelayError> {
    match cli.command {
        Commands::Doctor => Ok(Output::success(
            "doctor completed",
            app.doctor_report()?,
            cli.json,
        )),
        Commands::Status => Ok(Output::success(
            "status loaded",
            app.status_report()?,
            cli.json,
        )),
        Commands::Usage(command) => match command.command {
            None => usage_list_output(&app, "usage loaded", cli.json),
            Some(UsageSubcommand::Current) => {
                let snapshot = app.usage_report()?;
                Ok(Output::success_rendered(
                    "current usage loaded",
                    snapshot.clone(),
                    render_usage_detail(&snapshot),
                    cli.json,
                ))
            }
            Some(UsageSubcommand::Profile(args)) => {
                let snapshot = app.profile_usage_report(&profile_id_from_args(args)?)?;
                Ok(Output::success_rendered(
                    "profile usage loaded",
                    snapshot.clone(),
                    render_usage_detail(&snapshot),
                    cli.json,
                ))
            }
            Some(UsageSubcommand::List) => usage_list_output(&app, "usage list loaded", cli.json),
            Some(UsageSubcommand::Refresh(args)) => {
                let target = usage_refresh_target_from_args(args)?;
                match target {
                    UsageRefreshTarget::Profile(id) => {
                        let snapshot = app.refresh_usage_profile(&id)?;
                        Ok(Output::success_rendered(
                            "usage refreshed",
                            snapshot.clone(),
                            render_usage_detail(&snapshot),
                            cli.json,
                        ))
                    }
                    UsageRefreshTarget::Enabled => {
                        let snapshots = app.refresh_enabled_usage_reports()?;
                        let profiles = app.list_profiles()?;
                        Ok(Output::success_rendered(
                            "enabled profile usage refreshed",
                            snapshots.clone(),
                            render_usage_list(&snapshots, &profiles),
                            cli.json,
                        ))
                    }
                    UsageRefreshTarget::All => {
                        let snapshots = app.refresh_all_usage_reports()?;
                        let profiles = app.list_profiles()?;
                        Ok(Output::success_rendered(
                            "all profile usage refreshed",
                            snapshots.clone(),
                            render_usage_list(&snapshots, &profiles),
                            cli.json,
                        ))
                    }
                }
            }
            Some(UsageSubcommand::Config(command)) => match command.command {
                None => Ok(Output::success(
                    "usage settings loaded",
                    app.settings()?,
                    cli.json,
                )),
                Some(UsageConfigSubcommand::Set(args)) => Ok(Output::success(
                    "usage settings updated",
                    app.update_usage_settings(usage_settings_request_from_args(args)?)?,
                    cli.json,
                )),
            },
        },
        Commands::Profiles(command) => match command.command {
            ProfilesSubcommand::List => Ok(Output::success(
                "profiles loaded",
                app.list_profiles()?,
                cli.json,
            )),
            ProfilesSubcommand::Add(args) => {
                let request = add_profile_request_from_args(args)?;
                Ok(Output::success(
                    "profile created",
                    app.add_profile(request)?,
                    cli.json,
                ))
            }
            ProfilesSubcommand::Edit(args) => {
                let (id, request) = edit_profile_request_from_args(args)?;
                Ok(Output::success(
                    "profile updated",
                    app.edit_profile(&id, request)?,
                    cli.json,
                ))
            }
            ProfilesSubcommand::Remove(args) => Ok(Output::success(
                "profile removed",
                app.remove_profile(&profile_id_from_args(args)?)?,
                cli.json,
            )),
            ProfilesSubcommand::Enable(args) => Ok(Output::success(
                "profile enabled",
                app.set_profile_enabled(&profile_id_from_args(args)?, true)?,
                cli.json,
            )),
            ProfilesSubcommand::Disable(args) => Ok(Output::success(
                "profile disabled",
                app.set_profile_enabled(&profile_id_from_args(args)?, false)?,
                cli.json,
            )),
            ProfilesSubcommand::Import(args) => {
                let payload = import_profile_request_from_args(args)?;
                Ok(Output::success(
                    "profile imported",
                    app.import_profile(payload)?,
                    cli.json,
                ))
            }
            ProfilesSubcommand::Login(args) => {
                let payload = login_profile_request_from_args(args)?;
                Ok(Output::success(
                    "profile login created",
                    app.login_profile(payload)?,
                    cli.json,
                ))
            }
            ProfilesSubcommand::Relink(args) => {
                let (agent, id) = agent_profile_id_from_args(args)?;
                Ok(Output::success(
                    "profile relinked",
                    app.relink_profile(agent, &id)?,
                    cli.json,
                ))
            }
        },
        Commands::Switch(command) => {
            let target = switch_target_from_args(command)?;
            if target == "next" {
                Ok(Output::success(
                    "switch completed",
                    app.switch_next_profile()?,
                    cli.json,
                ))
            } else {
                Ok(Output::success(
                    "switch completed",
                    app.switch_to_profile(&target)?,
                    cli.json,
                ))
            }
        }
        Commands::AutoSwitch(command) => match command.command {
            AutoSwitchSubcommand::Enable => Ok(Output::success(
                "auto-switch enabled",
                app.set_auto_switch_enabled(true)?,
                cli.json,
            )),
            AutoSwitchSubcommand::Disable => Ok(Output::success(
                "auto-switch disabled",
                app.set_auto_switch_enabled(false)?,
                cli.json,
            )),
            AutoSwitchSubcommand::Set(args) => {
                let enabled = auto_switch_enabled_from_args(args)?;
                Ok(Output::success(
                    if enabled {
                        "auto-switch enabled"
                    } else {
                        "auto-switch disabled"
                    },
                    app.set_auto_switch_enabled(enabled)?,
                    cli.json,
                ))
            }
        },
        Commands::Events(command) => match command.command {
            EventsSubcommand::List(args) => {
                let limit = events_limit_from_args(args)?;
                Ok(Output::success(
                    "events loaded",
                    app.list_failure_events(limit)?,
                    cli.json,
                ))
            }
        },
        Commands::Logs(command) => match command.command {
            LogsSubcommand::Tail(args) => {
                let lines = log_lines_from_args(args)?;
                Ok(Output::success(
                    "logs loaded",
                    app.logs_tail(lines)?,
                    cli.json,
                ))
            }
        },
        Commands::Diagnostics(command) => match command.command {
            DiagnosticsSubcommand::Export => Ok(Output::success(
                "diagnostics exported",
                app.diagnostics_export()?,
                cli.json,
            )),
        },
    }
}

fn usage_list_output(app: &RelayApp, message: &str, json: bool) -> Result<Output, RelayError> {
    let snapshots = app.list_usage_reports()?;
    let profiles = app.list_profiles()?;
    Ok(Output::success_rendered(
        message,
        snapshots.clone(),
        render_usage_list(&snapshots, &profiles),
        json,
    ))
}

fn render_usage_list(snapshots: &[UsageSnapshot], profiles: &[Profile]) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        "Profile",
        "State",
        "Source",
        "Confidence",
        "Session",
        "Weekly",
        "Next Reset",
        "Notes",
    ]);

    for snapshot in snapshots {
        let enabled = snapshot
            .profile_id
            .as_deref()
            .and_then(|id| profiles.iter().find(|profile| profile.id == id))
            .map(|profile| profile.enabled)
            .unwrap_or(true);
        table.add_row(Row::from(vec![
            Cell::new(display_profile(snapshot)).set_alignment(CellAlignment::Left),
            styled_cell(
                profile_state_label(enabled, snapshot.stale),
                usage_tone(snapshot),
            ),
            Cell::new(usage_source_label(&snapshot.source)),
            styled_cell(
                usage_confidence_label(&snapshot.confidence),
                confidence_tone(snapshot.confidence.clone()),
            ),
            styled_cell(
                window_label(&snapshot.session),
                status_tone(snapshot.session.status.clone()),
            ),
            styled_cell(
                window_label(&snapshot.weekly),
                status_tone(snapshot.weekly.status.clone()),
            ),
            Cell::new(format_optional_datetime(snapshot.next_reset_at)),
            Cell::new(snapshot.message.clone().unwrap_or_else(|| "-".into())),
        ]));
    }

    table.to_string()
}

fn render_usage_detail(snapshot: &UsageSnapshot) -> String {
    let mut lines = Vec::new();
    lines.push(format!("Profile: {}", display_profile(snapshot)));
    lines.push(format!(
        "Source: {} | Confidence: {} | {}",
        usage_source_label(&snapshot.source),
        usage_confidence_label(&snapshot.confidence),
        if snapshot.stale { "stale" } else { "fresh" }
    ));
    lines.push(format!(
        "Updated: {}",
        format_datetime(snapshot.last_refreshed_at)
    ));
    lines.push(format!(
        "Session: {}",
        detail_window_line(&snapshot.session)
    ));
    lines.push(format!("Weekly: {}", detail_window_line(&snapshot.weekly)));
    lines.push(format!(
        "Next reset: {}",
        format_optional_datetime(snapshot.next_reset_at)
    ));
    lines.push(format!(
        "Auto-switch: {}",
        if snapshot.can_auto_switch {
            snapshot
                .auto_switch_reason
                .as_ref()
                .map(|reason| format!("eligible ({reason:?})"))
                .unwrap_or_else(|| "eligible".into())
        } else {
            "not eligible".into()
        }
    ));
    if let Some(message) = &snapshot.message {
        lines.push(format!("Notes: {message}"));
    }
    lines.join("\n")
}

fn display_profile(snapshot: &UsageSnapshot) -> String {
    match (&snapshot.profile_name, &snapshot.profile_id) {
        (Some(name), Some(id)) => format!("{name} ({id})"),
        (Some(name), None) => name.clone(),
        (None, Some(id)) => id.clone(),
        (None, None) => "current".into(),
    }
}

fn profile_state_label(enabled: bool, stale: bool) -> &'static str {
    match (enabled, stale) {
        (true, false) => "enabled",
        (true, true) => "enabled/stale",
        (false, false) => "disabled",
        (false, true) => "disabled/stale",
    }
}

fn window_label(window: &UsageWindow) -> String {
    match window.used_percent {
        Some(percent) => format!("{} ({percent:.0}%)", usage_status_label(&window.status)),
        None => usage_status_label(&window.status).into(),
    }
}

fn detail_window_line(window: &UsageWindow) -> String {
    let mut parts = vec![window_label(window)];
    if let Some(minutes) = window.window_minutes {
        parts.push(format!("{minutes}m"));
    }
    if let Some(reset_at) = window.reset_at {
        parts.push(format!("resets {}", format_datetime(reset_at)));
    }
    if !window.exact {
        parts.push("estimated".into());
    }
    parts.join(" | ")
}

fn format_optional_datetime(value: Option<DateTime<Utc>>) -> String {
    value.map(format_datetime).unwrap_or_else(|| "-".into())
}

fn format_datetime(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

fn styled_cell(value: impl Into<String>, tone: CellTone) -> Cell {
    let mut cell = Cell::new(value.into()).set_alignment(CellAlignment::Left);
    match tone {
        CellTone::Info => {
            cell = cell.fg(comfy_table::Color::Cyan);
        }
        CellTone::Good => {
            cell = cell.fg(comfy_table::Color::Green);
        }
        CellTone::Warn => {
            cell = cell.fg(comfy_table::Color::Yellow);
        }
        CellTone::Bad => {
            cell = cell
                .fg(comfy_table::Color::Red)
                .add_attribute(Attribute::Bold);
        }
        CellTone::Muted => {
            cell = cell.fg(comfy_table::Color::DarkGrey);
        }
    }
    cell
}

fn usage_tone(snapshot: &UsageSnapshot) -> CellTone {
    if !snapshot.can_auto_switch && snapshot.stale {
        CellTone::Warn
    } else if snapshot.auto_switch_reason.is_some()
        || snapshot.session.status == UsageStatus::Exhausted
        || snapshot.weekly.status == UsageStatus::Exhausted
    {
        CellTone::Bad
    } else {
        CellTone::Info
    }
}

fn confidence_tone(confidence: UsageConfidence) -> CellTone {
    match confidence {
        UsageConfidence::High => CellTone::Good,
        UsageConfidence::Medium => CellTone::Warn,
        UsageConfidence::Low => CellTone::Muted,
    }
}

fn status_tone(status: UsageStatus) -> CellTone {
    match status {
        UsageStatus::Healthy => CellTone::Good,
        UsageStatus::Warning => CellTone::Warn,
        UsageStatus::Exhausted => CellTone::Bad,
        UsageStatus::Unknown => CellTone::Muted,
    }
}

fn usage_source_label(source: &relay_core::UsageSource) -> &'static str {
    match source {
        relay_core::UsageSource::Local => "Local",
        relay_core::UsageSource::Fallback => "Fallback",
        relay_core::UsageSource::WebEnhanced => "WebEnhanced",
    }
}

fn usage_confidence_label(confidence: &UsageConfidence) -> &'static str {
    match confidence {
        UsageConfidence::High => "High",
        UsageConfidence::Medium => "Medium",
        UsageConfidence::Low => "Low",
    }
}

fn usage_status_label(status: &UsageStatus) -> &'static str {
    match status {
        UsageStatus::Healthy => "Healthy",
        UsageStatus::Warning => "Warning",
        UsageStatus::Exhausted => "Exhausted",
        UsageStatus::Unknown => "Unknown",
    }
}

#[derive(Clone, Copy)]
enum CellTone {
    Info,
    Good,
    Warn,
    Bad,
    Muted,
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

fn parse_agent_kind(value: &str) -> Result<AgentKind, RelayError> {
    match value {
        "codex" | "Codex" => Ok(AgentKind::Codex),
        other => Err(RelayError::InvalidInput(format!(
            "unsupported agent: {other}"
        ))),
    }
}

fn add_profile_request_from_args(args: AddProfileArgs) -> Result<AddProfileRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.agent.is_some(),
                args.nickname.is_some(),
                args.priority.is_some(),
                args.config_path.is_some(),
                args.agent_home.is_some(),
                args.auth_mode.is_some(),
            ],
        )?;
        let payload: AddProfileInput = read_json_input(input_json)?;
        return Ok(AddProfileRequest {
            agent: payload.agent,
            nickname: payload.nickname,
            priority: payload.priority,
            config_path: payload.config_path,
            agent_home: payload.agent_home,
            auth_mode: payload.auth_mode,
        });
    }

    Ok(AddProfileRequest {
        agent: parse_agent_kind(&require_field(args.agent, "agent is required")?)?,
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

fn import_profile_request_from_args(
    args: ImportProfileArgs,
) -> Result<ImportProfileRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.agent.is_some(),
                args.nickname.is_some(),
                args.priority.is_some(),
            ],
        )?;
        let payload: ImportProfileInput = read_json_input(input_json)?;
        return Ok(ImportProfileRequest {
            agent: payload.agent,
            nickname: payload.nickname,
            priority: payload.priority,
        });
    }

    Ok(ImportProfileRequest {
        agent: parse_agent_kind(&require_field(args.agent, "agent is required")?)?,
        nickname: args.nickname,
        priority: args.priority.unwrap_or(100),
    })
}

fn login_profile_request_from_args(
    args: LoginProfileArgs,
) -> Result<AgentLoginRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.agent.is_some(),
                args.nickname.is_some(),
                args.priority.is_some(),
            ],
        )?;
        let payload: LoginProfileInput = read_json_input(input_json)?;
        return Ok(AgentLoginRequest {
            agent: payload.agent,
            nickname: payload.nickname,
            priority: payload.priority,
        });
    }

    Ok(AgentLoginRequest {
        agent: parse_agent_kind(&require_field(args.agent, "agent is required")?)?,
        nickname: args.nickname,
        priority: args.priority.unwrap_or(100),
    })
}

fn agent_profile_id_from_args(args: AgentProfileIdArgs) -> Result<(AgentKind, String), RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.agent.is_some(), args.id.is_some()])?;
        let payload: AgentProfileIdInput = read_json_input(input_json)?;
        return Ok((payload.agent, payload.id));
    }

    Ok((
        parse_agent_kind(&require_field(args.agent, "agent is required")?)?,
        require_field(args.id, "profile id is required")?,
    ))
}

fn profile_id_from_args(args: ProfileIdArgs) -> Result<String, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.id.is_some()])?;
        let payload: ProfileIdInput = read_json_input(input_json)?;
        return Ok(payload.id);
    }

    require_field(args.id, "profile id is required")
}

fn switch_target_from_args(args: SwitchCommand) -> Result<String, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.target.is_some()])?;
        let payload: SwitchInput = read_json_input(input_json)?;
        return Ok(payload.target);
    }

    require_field(args.target, "switch target is required")
}

fn auto_switch_enabled_from_args(args: AutoSwitchSetArgs) -> Result<bool, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.enabled.is_some()])?;
        let payload: AutoSwitchInput = read_json_input(input_json)?;
        return Ok(payload.enabled);
    }

    require_field(args.enabled, "auto-switch enabled value is required")
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

enum UsageRefreshTarget {
    Profile(String),
    Enabled,
    All,
}

fn usage_refresh_target_from_args(
    args: UsageRefreshArgs,
) -> Result<UsageRefreshTarget, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.id.is_some(), args.enabled, args.all])?;
        let payload: UsageRefreshInput = read_json_input(input_json)?;
        return usage_refresh_target_from_input(payload);
    }

    usage_refresh_target_from_input(UsageRefreshInput {
        id: args.id,
        enabled: args.enabled,
        all: args.all,
    })
}

fn usage_refresh_target_from_input(
    input: UsageRefreshInput,
) -> Result<UsageRefreshTarget, RelayError> {
    let selectors = [input.id.is_some(), input.enabled, input.all];
    if selectors.into_iter().filter(|value| *value).count() != 1 {
        return Err(RelayError::InvalidInput(
            "usage refresh requires exactly one selector: profile id, --enabled, or --all".into(),
        ));
    }

    if input.all {
        Ok(UsageRefreshTarget::All)
    } else if input.enabled {
        Ok(UsageRefreshTarget::Enabled)
    } else {
        Ok(UsageRefreshTarget::Profile(input.id.ok_or_else(|| {
            RelayError::InvalidInput("profile id is required".into())
        })?))
    }
}

fn usage_settings_request_from_args(
    args: UsageConfigSetArgs,
) -> Result<UsageSettingsUpdateRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.source_mode.is_some(),
                args.menu_open_refresh_stale_after_seconds.is_some(),
                args.background_refresh_enabled.is_some(),
                args.background_refresh_interval_seconds.is_some(),
            ],
        )?;
        let payload: UsageConfigSetInput = read_json_input(input_json)?;
        return Ok(UsageSettingsUpdateRequest {
            source_mode: payload.source_mode,
            menu_open_refresh_stale_after_seconds: payload.menu_open_refresh_stale_after_seconds,
            background_refresh_enabled: payload.background_refresh_enabled,
            background_refresh_interval_seconds: payload.background_refresh_interval_seconds,
        });
    }

    Ok(UsageSettingsUpdateRequest {
        source_mode: args
            .source_mode
            .as_deref()
            .map(parse_usage_source_mode)
            .transpose()?,
        menu_open_refresh_stale_after_seconds: args.menu_open_refresh_stale_after_seconds,
        background_refresh_enabled: args.background_refresh_enabled,
        background_refresh_interval_seconds: args.background_refresh_interval_seconds,
    })
}

fn events_limit_from_args(args: ListArgs) -> Result<usize, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.limit.is_some()])?;
        let payload: EventsListInput = read_json_input(input_json)?;
        return Ok(payload.limit);
    }

    Ok(args.limit.unwrap_or(50))
}

fn log_lines_from_args(args: TailArgs) -> Result<usize, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(input_json, &[args.lines.is_some()])?;
        let payload: LogsTailInput = read_json_input(input_json)?;
        return Ok(payload.lines);
    }

    Ok(args.lines.unwrap_or(50))
}

fn ensure_json_input_is_exclusive(path: &PathBuf, conflicts: &[bool]) -> Result<(), RelayError> {
    if conflicts.iter().any(|value| *value) {
        return Err(RelayError::InvalidInput(format!(
            "--input-json {} cannot be combined with inline command arguments",
            path.display()
        )));
    }
    Ok(())
}

fn read_json_input<T>(path: &PathBuf) -> Result<T, RelayError>
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
struct AddProfileInput {
    #[serde(deserialize_with = "deserialize_agent_kind")]
    agent: AgentKind,
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
struct SwitchInput {
    target: String,
}

#[derive(Debug, Deserialize)]
struct AutoSwitchInput {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct UsageRefreshInput {
    id: Option<String>,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    all: bool,
}

#[derive(Debug, Deserialize)]
struct UsageConfigSetInput {
    #[serde(default, deserialize_with = "deserialize_usage_source_mode_option")]
    source_mode: Option<UsageSourceMode>,
    menu_open_refresh_stale_after_seconds: Option<i64>,
    background_refresh_enabled: Option<bool>,
    background_refresh_interval_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AgentProfileIdInput {
    #[serde(deserialize_with = "deserialize_agent_kind")]
    agent: AgentKind,
    id: String,
}

#[derive(Debug, Deserialize)]
struct LoginProfileInput {
    #[serde(deserialize_with = "deserialize_agent_kind")]
    agent: AgentKind,
    nickname: Option<String>,
    #[serde(default = "default_priority")]
    priority: i32,
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

fn deserialize_agent_kind<'de, D>(deserializer: D) -> Result<AgentKind, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    parse_agent_kind(&value).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct ImportProfileInput {
    #[serde(deserialize_with = "deserialize_agent_kind")]
    agent: AgentKind,
    nickname: Option<String>,
    #[serde(default = "default_priority")]
    priority: i32,
}

#[derive(Debug, Deserialize)]
struct EventsListInput {
    #[serde(default = "default_list_limit")]
    limit: usize,
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

struct Output {
    json: bool,
    text: String,
    body: String,
    rendered_body: Option<String>,
}

impl Output {
    fn success<T: Serialize>(message: &str, data: T, json: bool) -> Self {
        Self::success_rendered(message, data, String::new(), json)
    }

    fn success_rendered<T: Serialize>(
        message: &str,
        data: T,
        rendered_body: String,
        json: bool,
    ) -> Self {
        let body = serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string());
        Self {
            json,
            text: message.to_string(),
            body,
            rendered_body: (!rendered_body.is_empty()).then_some(rendered_body),
        }
    }

    fn write(self) -> Result<(), RelayError> {
        if self.json {
            let value: serde_json::Value = serde_json::from_str(&self.body)
                .map_err(|error| RelayError::Internal(error.to_string()))?;
            JsonResponse::success(self.text, value).write_json()?;
            Ok(())
        } else {
            println!("{}", self.text);
            println!(
                "{}",
                self.rendered_body.as_deref().unwrap_or(self.body.as_str())
            );
            Ok(())
        }
    }
}
