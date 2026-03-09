use chrono::{DateTime, Local, Utc};
use clap::{Args, Parser, Subcommand};
use comfy_table::{
    Attribute, Cell, CellAlignment, ContentArrangement, Row, Table, modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL,
};
use relay_core::models::JsonResponse;
use relay_core::{
    ActiveState, ActivityEventsQuery, AddProfileRequest, AgentKind, AgentLinkResult,
    AgentLoginRequest, AppSettings, AuthMode, BootstrapMode, CodexSettings,
    CodexSettingsUpdateRequest, DiagnosticsExport, DoctorReport, EditProfileRequest, FailureEvent,
    FailureReason, ImportProfileRequest, LogTail, ProbeProvider, Profile, ProfileDetail,
    ProfileProbeIdentity, RelayApp, RelayError, SwitchOutcome, SwitchReport,
    SystemSettingsUpdateRequest, SystemStatusReport, UsageSnapshot, UsageSourceMode, UsageStatus,
    UsageWindow,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
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
    Settings(SettingsCommand),
    List,
    Show(ShowCommand),
    Edit(EditProfileArgs),
    Remove(ProfileIdArgs),
    Enable(ProfileIdArgs),
    Disable(ProfileIdArgs),
    Switch(SwitchCommand),
    Refresh(RefreshCommand),
    Autoswitch(AutoswitchCommand),
    Activity(ActivityCommand),
    Codex(CodexCommand),
}

#[derive(Debug, Args)]
struct ProfileIdArgs {
    id: Option<String>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct SettingsCommand {
    #[command(subcommand)]
    command: Option<SettingsSubcommand>,
}

#[derive(Debug, Subcommand)]
enum SettingsSubcommand {
    Show,
}

#[derive(Debug, Args)]
struct ShowCommand {
    target: Option<String>,
}

#[derive(Debug, Args)]
struct SwitchCommand {
    target: Option<String>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct RefreshCommand {
    target: Option<String>,
    #[arg(long)]
    all: bool,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct AutoswitchCommand {
    #[command(subcommand)]
    command: Option<AutoswitchSubcommand>,
}

#[derive(Debug, Subcommand)]
enum AutoswitchSubcommand {
    Show,
    Enable,
    Disable,
    Set(AutoswitchSetArgs),
}

#[derive(Debug, Args)]
struct AutoswitchSetArgs {
    #[arg(long)]
    enabled: Option<bool>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ActivityCommand {
    #[command(subcommand)]
    command: ActivitySubcommand,
}

#[derive(Debug, Subcommand)]
enum ActivitySubcommand {
    Events(ActivityEventsCommand),
    Logs(LogsCommand),
    Diagnostics(DiagnosticsCommand),
}

#[derive(Debug, Args)]
struct ActivityEventsCommand {
    #[command(subcommand)]
    command: ActivityEventsSubcommand,
}

#[derive(Debug, Subcommand)]
enum ActivityEventsSubcommand {
    List(ActivityEventsListArgs),
}

#[derive(Debug, Args)]
struct LogsCommand {
    #[command(subcommand)]
    command: LogsSubcommand,
}

#[derive(Debug, Args)]
struct CodexCommand {
    #[command(subcommand)]
    command: CodexSubcommand,
}

#[derive(Debug, Subcommand)]
enum CodexSubcommand {
    Add(CodexAddArgs),
    Import(CodexImportArgs),
    Login(CodexLoginArgs),
    Relink(ProfileIdArgs),
    Settings(CodexSettingsCommand),
}

#[derive(Debug, Args)]
struct CodexSettingsCommand {
    #[command(subcommand)]
    command: Option<CodexSettingsSubcommand>,
}

#[derive(Debug, Subcommand)]
enum CodexSettingsSubcommand {
    Show,
    Set(CodexSettingsSetArgs),
}

#[derive(Debug, Args)]
struct CodexSettingsSetArgs {
    #[arg(long)]
    source_mode: Option<String>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct CodexAddArgs {
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
struct CodexImportArgs {
    #[arg(long)]
    nickname: Option<String>,
    #[arg(long)]
    priority: Option<i32>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct CodexLoginArgs {
    #[arg(long)]
    nickname: Option<String>,
    #[arg(long)]
    priority: Option<i32>,
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
struct ActivityEventsListArgs {
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    profile_id: Option<String>,
    #[arg(long)]
    reason: Option<String>,
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
        Commands::Doctor
        | Commands::Status
        | Commands::List
        | Commands::Show(_)
        | Commands::Settings(SettingsCommand {
            command: None | Some(SettingsSubcommand::Show),
        }) => BootstrapMode::ReadOnly,
        Commands::Edit(_)
        | Commands::Remove(_)
        | Commands::Enable(_)
        | Commands::Disable(_)
        | Commands::Switch(_)
        | Commands::Refresh(_)
        | Commands::Autoswitch(_) => BootstrapMode::ReadWrite,
        Commands::Codex(command) => match &command.command {
            CodexSubcommand::Settings(CodexSettingsCommand {
                command: None | Some(CodexSettingsSubcommand::Show),
            }) => BootstrapMode::ReadOnly,
            CodexSubcommand::Settings(CodexSettingsCommand {
                command: Some(CodexSettingsSubcommand::Set(_)),
            })
            | CodexSubcommand::Add(_)
            | CodexSubcommand::Import(_)
            | CodexSubcommand::Login(_)
            | CodexSubcommand::Relink(_) => BootstrapMode::ReadWrite,
        },
        Commands::Activity(command) => match &command.command {
            ActivitySubcommand::Events(_) | ActivitySubcommand::Logs(_) => BootstrapMode::ReadOnly,
            ActivitySubcommand::Diagnostics(_) => BootstrapMode::ReadWrite,
        },
    };
    let app = RelayApp::bootstrap_with_mode(bootstrap_mode)?;
    dispatch(cli, app)
}

fn dispatch(cli: Cli, app: RelayApp) -> Result<Output, RelayError> {
    match cli.command {
        Commands::Doctor => {
            let report = app.doctor_report()?;
            Ok(Output::success_rendered(
                "doctor completed",
                report.clone(),
                render_doctor_report(&report),
                cli.json,
            ))
        }
        Commands::Status => {
            let report = app.system_status()?;
            Ok(Output::success_rendered(
                "status loaded",
                report.clone(),
                render_status_report(&report),
                cli.json,
            ))
        }
        Commands::Settings(command) => match command.command {
            None | Some(SettingsSubcommand::Show) => {
                let settings = app.settings()?;
                Ok(Output::success_rendered(
                    "settings loaded",
                    settings.clone(),
                    render_settings(&settings),
                    cli.json,
                ))
            }
        },
        Commands::List => list_output(&app, cli.json),
        Commands::Show(command) => {
            let detail = match show_target_from_args(command)? {
                ShowTarget::Current => app.current_profile_detail()?,
                ShowTarget::Profile(id) => app.profile_detail(&id)?,
            };
            Ok(Output::success_rendered(
                "profile detail loaded",
                detail.clone(),
                render_profile_summary(&detail),
                cli.json,
            ))
        }
        Commands::Edit(args) => {
            let (id, request) = edit_profile_request_from_args(args)?;
            let profile = app.edit_profile(&id, request)?;
            Ok(Output::success_rendered(
                "profile updated",
                profile.clone(),
                render_profile_detail(&profile),
                cli.json,
            ))
        }
        Commands::Remove(args) => {
            let profile = app.remove_profile(&profile_id_from_args(args)?)?;
            Ok(Output::success_rendered(
                "profile removed",
                profile.clone(),
                render_profile_detail(&profile),
                cli.json,
            ))
        }
        Commands::Enable(args) => {
            let profile = app.set_profile_enabled(&profile_id_from_args(args)?, true)?;
            Ok(Output::success_rendered(
                "profile enabled",
                profile.clone(),
                render_profile_detail(&profile),
                cli.json,
            ))
        }
        Commands::Disable(args) => {
            let profile = app.set_profile_enabled(&profile_id_from_args(args)?, false)?;
            Ok(Output::success_rendered(
                "profile disabled",
                profile.clone(),
                render_profile_detail(&profile),
                cli.json,
            ))
        }
        Commands::Switch(command) => {
            let report = match switch_target_from_args(command)? {
                SwitchTarget::Next => app.switch_next_profile()?,
                SwitchTarget::Profile(id) => app.switch_to_profile(&id)?,
            };
            Ok(Output::success_rendered(
                "switch completed",
                report.clone(),
                render_switch_report(&report),
                cli.json,
            ))
        }
        Commands::Refresh(command) => match refresh_target_from_args(command)? {
            RefreshTarget::Profile(id) => {
                let snapshot = app.refresh_usage_profile(&id)?;
                Ok(Output::success_rendered(
                    "profile refreshed",
                    snapshot.clone(),
                    render_usage_detail(&snapshot),
                    cli.json,
                ))
            }
            RefreshTarget::Enabled => {
                let snapshots = app.refresh_enabled_usage_reports()?;
                let items = app.list_profiles_with_usage()?;
                Ok(Output::success_rendered(
                    "enabled profiles refreshed",
                    snapshots.clone(),
                    render_usage_list(&snapshots, &items),
                    cli.json,
                ))
            }
            RefreshTarget::All => {
                let snapshots = app.refresh_all_usage_reports()?;
                let items = app.list_profiles_with_usage()?;
                Ok(Output::success_rendered(
                    "all profiles refreshed",
                    snapshots.clone(),
                    render_usage_list(&snapshots, &items),
                    cli.json,
                ))
            }
        },
        Commands::Autoswitch(command) => match command.command {
            None | Some(AutoswitchSubcommand::Show) => {
                let settings = app.settings()?;
                Ok(Output::success_rendered(
                    "autoswitch status loaded",
                    settings.clone(),
                    render_autoswitch_settings(&settings),
                    cli.json,
                ))
            }
            Some(AutoswitchSubcommand::Enable) => {
                let settings = app.set_auto_switch_enabled(true)?;
                Ok(Output::success_rendered(
                    "autoswitch enabled",
                    settings.clone(),
                    render_autoswitch_settings(&settings),
                    cli.json,
                ))
            }
            Some(AutoswitchSubcommand::Disable) => {
                let settings = app.set_auto_switch_enabled(false)?;
                Ok(Output::success_rendered(
                    "autoswitch disabled",
                    settings.clone(),
                    render_autoswitch_settings(&settings),
                    cli.json,
                ))
            }
            Some(AutoswitchSubcommand::Set(args)) => {
                let settings =
                    app.update_system_settings(system_settings_request_from_args(args)?)?;
                Ok(Output::success_rendered(
                    "autoswitch updated",
                    settings.clone(),
                    render_autoswitch_settings(&settings),
                    cli.json,
                ))
            }
        },
        Commands::Activity(command) => match command.command {
            ActivitySubcommand::Events(command) => match command.command {
                ActivityEventsSubcommand::List(args) => {
                    let query = activity_events_query_from_args(args)?;
                    let events = app.list_activity_events(query)?;
                    let items = app.list_profiles_with_usage()?;
                    Ok(Output::success_rendered(
                        "activity events loaded",
                        events.clone(),
                        render_failure_events(&events, &items),
                        cli.json,
                    ))
                }
            },
            ActivitySubcommand::Logs(command) => match command.command {
                LogsSubcommand::Tail(args) => {
                    let logs = app.logs_tail(log_lines_from_args(args)?)?;
                    Ok(Output::success_rendered(
                        "activity logs loaded",
                        logs.clone(),
                        render_log_tail(&logs),
                        cli.json,
                    ))
                }
            },
            ActivitySubcommand::Diagnostics(command) => match command.command {
                DiagnosticsSubcommand::Export => {
                    let export = app.diagnostics_export()?;
                    Ok(Output::success_rendered(
                        "activity diagnostics exported",
                        export.clone(),
                        render_diagnostics_export(&export),
                        cli.json,
                    ))
                }
            },
        },
        Commands::Codex(command) => match command.command {
            CodexSubcommand::Add(args) => {
                let profile = app.add_profile(codex_add_request_from_args(args)?)?;
                Ok(Output::success_rendered(
                    "codex profile created",
                    profile.clone(),
                    render_profile_detail(&profile),
                    cli.json,
                ))
            }
            CodexSubcommand::Import(args) => {
                let payload = codex_import_request_from_args(args)?;
                let profile = app.import_profile(payload)?;
                Ok(Output::success_rendered(
                    "codex profile imported",
                    profile.clone(),
                    render_profile_detail(&profile),
                    cli.json,
                ))
            }
            CodexSubcommand::Login(args) => {
                let payload = codex_login_request_from_args(args)?;
                let result = app.login_profile(payload)?;
                Ok(Output::success_rendered(
                    "codex login profile created",
                    result.clone(),
                    render_agent_link_result(&result),
                    cli.json,
                ))
            }
            CodexSubcommand::Relink(args) => {
                let id = profile_id_from_args(args)?;
                let identity = app.relink_profile(AgentKind::Codex, &id)?;
                Ok(Output::success_rendered(
                    "codex profile relinked",
                    identity.clone(),
                    render_probe_identity(&identity),
                    cli.json,
                ))
            }
            CodexSubcommand::Settings(command) => match command.command {
                None | Some(CodexSettingsSubcommand::Show) => {
                    let settings = app.codex_settings()?;
                    Ok(Output::success_rendered(
                        "codex settings loaded",
                        settings.clone(),
                        render_codex_settings(&settings),
                        cli.json,
                    ))
                }
                Some(CodexSettingsSubcommand::Set(args)) => {
                    let settings =
                        app.update_codex_settings(codex_settings_request_from_args(args)?)?;
                    Ok(Output::success_rendered(
                        "codex settings updated",
                        settings.clone(),
                        render_codex_settings(&settings),
                        cli.json,
                    ))
                }
            },
        },
    }
}

fn list_output(app: &RelayApp, json: bool) -> Result<Output, RelayError> {
    let items = app.list_profiles_with_usage()?;
    Ok(Output::success_rendered(
        "profiles loaded",
        items.clone(),
        render_profiles_list(&items),
        json,
    ))
}

fn render_doctor_report(report: &DoctorReport) -> String {
    render_sections(vec![
        (
            "Environment",
            vec![
                ("Platform", report.platform.clone()),
                (
                    "Primary Agent",
                    agent_kind_label(&report.primary_agent).into(),
                ),
                (
                    "Agent Binary",
                    report.agent_binary.clone().unwrap_or_else(|| "-".into()),
                ),
                (
                    "Managed Files",
                    if report.managed_files.is_empty() {
                        "-".into()
                    } else {
                        report.managed_files.join(", ")
                    },
                ),
            ],
        ),
        (
            "Paths",
            vec![
                ("Relay Home", report.relay_home.clone()),
                ("Relay DB", report.relay_db_path.clone()),
                ("Relay Log", report.relay_log_path.clone()),
                ("Live Agent Home", report.live_agent_home.clone()),
                (
                    "Default Agent Home",
                    report
                        .default_agent_home
                        .clone()
                        .unwrap_or_else(|| "-".into()),
                ),
                (
                    "Default Home Exists",
                    yes_no(report.default_agent_home_exists).into(),
                ),
            ],
        ),
        (
            "Environment Overrides",
            vec![
                (
                    "Agent Home Env",
                    report
                        .agent_home_env_name
                        .clone()
                        .unwrap_or_else(|| "-".into()),
                ),
                (
                    "Agent Home Value",
                    report
                        .agent_home_env_value
                        .clone()
                        .unwrap_or_else(|| "-".into()),
                ),
            ],
        ),
    ])
}

fn render_status_report(report: &SystemStatusReport) -> String {
    render_sections(vec![
        (
            "Relay",
            vec![
                ("Relay Home", report.relay_home.clone()),
                ("Live Agent Home", report.live_agent_home.clone()),
                ("Profile Count", report.profile_count.to_string()),
            ],
        ),
        ("Active State", active_state_fields(&report.active_state)),
        ("Settings", app_settings_fields(&report.settings)),
    ])
}

fn render_usage_list(
    snapshots: &[UsageSnapshot],
    profiles: &[relay_core::ProfileListItem],
) -> String {
    let mut table = new_table();
    table.set_header(vec![
        "Profile",
        "State",
        "Source",
        "Session",
        "Weekly",
        "Next Reset",
    ]);

    for snapshot in snapshots {
        let enabled = snapshot
            .profile_id
            .as_deref()
            .and_then(|id| profiles.iter().find(|item| item.profile.id == id))
            .map(|item| item.profile.enabled)
            .unwrap_or(true);
        table.add_row(Row::from(vec![
            Cell::new(display_profile(snapshot)).set_alignment(CellAlignment::Left),
            styled_cell(
                profile_state_label(enabled, snapshot.stale),
                usage_tone(snapshot),
            ),
            styled_cell(usage_source_label(&snapshot.source), CellTone::Info),
            styled_cell(
                list_window_label(&snapshot.session),
                status_tone(snapshot.session.status.clone()),
            ),
            styled_cell(
                list_window_label(&snapshot.weekly),
                status_tone(snapshot.weekly.status.clone()),
            ),
            Cell::new(format_optional_datetime(snapshot.next_reset_at)),
        ]));
    }

    table.to_string()
}

fn render_usage_detail(snapshot: &UsageSnapshot) -> String {
    let mut fields = vec![
        ("Profile", display_profile(snapshot)),
        ("Source", usage_source_label(&snapshot.source).into()),
        (
            "Freshness",
            if snapshot.stale { "stale" } else { "fresh" }.into(),
        ),
        ("Updated", format_datetime(snapshot.last_refreshed_at)),
        ("Session", detail_window_line(&snapshot.session)),
        ("Weekly", detail_window_line(&snapshot.weekly)),
        (
            "Next Reset",
            format_optional_datetime(snapshot.next_reset_at),
        ),
        (
            "Auto-switch",
            if snapshot.can_auto_switch {
                snapshot
                    .auto_switch_reason
                    .as_ref()
                    .map(|reason| format!("eligible ({reason:?})"))
                    .unwrap_or_else(|| "eligible".into())
            } else {
                "not eligible".into()
            },
        ),
    ];
    if let Some(message) = user_facing_usage_note(snapshot) {
        fields.push(("Notes", message));
    }
    render_sections(vec![("Usage", fields)])
}

fn render_settings(settings: &AppSettings) -> String {
    render_sections(vec![("Settings", app_settings_fields(settings))])
}

fn render_codex_settings(settings: &CodexSettings) -> String {
    render_sections(vec![("Codex Settings", codex_settings_fields(settings))])
}

fn render_autoswitch_settings(settings: &AppSettings) -> String {
    render_sections(vec![("Autoswitch", autoswitch_fields(settings))])
}

fn render_profiles_list(items: &[relay_core::ProfileListItem]) -> String {
    let mut table = new_table();
    table.set_header(vec![
        "Current",
        "Nickname",
        "Profile ID",
        "Agent",
        "Priority",
        "Status",
        "Session",
        "Weekly",
        "Source",
        "Auth",
    ]);

    for item in items {
        let profile = &item.profile;
        let usage = item.usage_summary.as_ref();
        table.add_row(Row::from(vec![
            styled_cell(
                if item.is_active { "yes" } else { "-" },
                if item.is_active {
                    CellTone::Info
                } else {
                    CellTone::Muted
                },
            ),
            Cell::new(profile.nickname.as_str()),
            Cell::new(profile.id.as_str()),
            Cell::new(agent_kind_label(&profile.agent)),
            Cell::new(profile.priority),
            styled_cell(
                if profile.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                if profile.enabled {
                    CellTone::Good
                } else {
                    CellTone::Muted
                },
            ),
            styled_cell(
                usage
                    .map(|value| list_window_label(&value.session))
                    .unwrap_or_else(|| "-".into()),
                usage
                    .map(|value| status_tone(value.session.status.clone()))
                    .unwrap_or(CellTone::Muted),
            ),
            styled_cell(
                usage
                    .map(|value| list_window_label(&value.weekly))
                    .unwrap_or_else(|| "-".into()),
                usage
                    .map(|value| status_tone(value.weekly.status.clone()))
                    .unwrap_or(CellTone::Muted),
            ),
            Cell::new(
                usage
                    .map(|value| usage_source_label(&value.source).to_string())
                    .unwrap_or_else(|| "-".into()),
            ),
            Cell::new(auth_mode_label(&profile.auth_mode)),
        ]));
    }

    table.to_string()
}

fn render_profile_detail(profile: &Profile) -> String {
    render_sections(vec![(
        "Profile",
        vec![
            ("Nickname", profile.nickname.clone()),
            ("Profile ID", profile.id.clone()),
            ("Agent", agent_kind_label(&profile.agent).into()),
            ("Priority", profile.priority.to_string()),
            (
                "Status",
                if profile.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
                .into(),
            ),
            ("Auth Mode", auth_mode_label(&profile.auth_mode).into()),
            (
                "Agent Home",
                profile.agent_home.clone().unwrap_or_else(|| "-".into()),
            ),
            (
                "Config Path",
                profile.config_path.clone().unwrap_or_else(|| "-".into()),
            ),
            ("Created", profile.created_at.clone()),
            ("Updated", profile.updated_at.clone()),
        ],
    )])
}

fn render_profile_summary(detail: &ProfileDetail) -> String {
    let mut sections = vec![(
        "Profile",
        vec![
            ("Nickname", detail.profile.nickname.clone()),
            ("Profile ID", detail.profile.id.clone()),
            ("Agent", agent_kind_label(&detail.profile.agent).into()),
            ("Active", yes_no(detail.is_active).into()),
            (
                "Status",
                if detail.profile.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
                .into(),
            ),
            ("Switch Eligible", yes_no(detail.switch_eligible).into()),
            (
                "Eligibility Note",
                detail
                    .switch_ineligibility_reason
                    .clone()
                    .unwrap_or_else(|| "-".into()),
            ),
        ],
    )];

    if let Some(usage) = &detail.usage {
        sections.push(("Usage", usage_fields(usage)));
    }

    if let Some(event) = &detail.last_failure_event {
        sections.push((
            "Recent Failure",
            vec![
                ("Reason", failure_reason_label(&event.reason).into()),
                ("When", format_datetime(event.created_at)),
                ("Message", event.message.clone()),
            ],
        ));
    }

    render_sections(sections)
}

fn render_agent_link_result(result: &AgentLinkResult) -> String {
    let mut sections = vec![
        (
            "Profile",
            vec![
                ("Nickname", result.profile.nickname.clone()),
                ("Profile ID", result.profile.id.clone()),
                ("Agent", agent_kind_label(&result.profile.agent).into()),
                (
                    "Status",
                    if result.profile.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                    .into(),
                ),
            ],
        ),
        ("Link", vec![("Activated", yes_no(result.activated).into())]),
    ];
    sections.push((
        "Probe Identity",
        probe_identity_fields(&result.probe_identity),
    ));
    render_sections(sections)
}

fn render_probe_identity(identity: &ProfileProbeIdentity) -> String {
    render_sections(vec![("Probe Identity", probe_identity_fields(identity))])
}

fn render_switch_report(report: &SwitchReport) -> String {
    render_sections(vec![(
        "Switch",
        vec![
            ("Target Profile", report.profile_id.clone()),
            (
                "Previous Profile",
                report
                    .previous_profile_id
                    .clone()
                    .unwrap_or_else(|| "-".into()),
            ),
            ("Checkpoint", report.checkpoint_id.clone()),
            ("Rollback", yes_no(report.rollback_performed).into()),
            ("Switched At", format_datetime(report.switched_at)),
            ("Message", report.message.clone()),
        ],
    )])
}

fn render_failure_events(
    events: &[FailureEvent],
    profiles: &[relay_core::ProfileListItem],
) -> String {
    let by_profile = profiles
        .iter()
        .map(|item| (item.profile.id.as_str(), item.profile.nickname.as_str()))
        .collect::<HashMap<_, _>>();
    let mut table = new_table();
    table.set_header(vec![
        "When",
        "Profile",
        "Reason",
        "Cooldown Until",
        "Message",
    ]);

    for event in events {
        let profile_label = event
            .profile_id
            .as_deref()
            .map(|id| {
                by_profile
                    .get(id)
                    .map(|name| format!("{name} ({id})"))
                    .unwrap_or_else(|| id.into())
            })
            .unwrap_or_else(|| "-".into());
        table.add_row(Row::from(vec![
            Cell::new(format_datetime(event.created_at)),
            Cell::new(profile_label),
            styled_cell(
                failure_reason_label(&event.reason),
                failure_reason_tone(&event.reason),
            ),
            Cell::new(format_optional_datetime(event.cooldown_until)),
            Cell::new(event.message.as_str()),
        ]));
    }

    table.to_string()
}

fn render_log_tail(log: &LogTail) -> String {
    let mut lines = vec![
        format!("Path: {}", log.path),
        format!("Lines: {}", log.lines.len()),
        String::new(),
    ];
    if log.lines.is_empty() {
        lines.push("No log lines available.".into());
    } else {
        lines.extend(log.lines.iter().cloned());
    }
    lines.join("\n")
}

fn render_diagnostics_export(export: &DiagnosticsExport) -> String {
    render_sections(vec![(
        "Diagnostics",
        vec![
            ("Archive Path", export.archive_path.clone()),
            ("Bundle Dir", export.bundle_dir.clone()),
            ("Created At", format_datetime(export.created_at)),
        ],
    )])
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

fn list_window_label(window: &UsageWindow) -> String {
    match (window.used_percent, window.reset_at) {
        (Some(percent), Some(reset_at)) => {
            format!("{percent:.0}% · {}", compact_reset_label(reset_at))
        }
        (Some(percent), None) => format!("{percent:.0}%"),
        (None, _) => "-".into(),
    }
}

fn compact_reset_label(reset_at: DateTime<Utc>) -> String {
    let interval = reset_at.signed_duration_since(Utc::now());
    if interval.num_seconds() <= 0 {
        return "now".into();
    }

    let total_minutes = ((interval.num_seconds() + 59) / 60).max(1);
    let days = total_minutes / (24 * 60);
    let hours = (total_minutes % (24 * 60)) / 60;
    let minutes = total_minutes % 60;
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 || parts.is_empty() {
        parts.push(format!("{minutes}m"));
    }
    parts.join(" ")
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

fn usage_fields(snapshot: &UsageSnapshot) -> Vec<(&'static str, String)> {
    let mut fields = vec![
        ("Source", usage_source_label(&snapshot.source).into()),
        (
            "Freshness",
            if snapshot.stale { "stale" } else { "fresh" }.into(),
        ),
        ("Updated", format_datetime(snapshot.last_refreshed_at)),
        ("Session", detail_window_line(&snapshot.session)),
        ("Weekly", detail_window_line(&snapshot.weekly)),
        (
            "Next Reset",
            format_optional_datetime(snapshot.next_reset_at),
        ),
    ];
    if let Some(message) = user_facing_usage_note(snapshot) {
        fields.push(("Notes", message));
    }
    fields
}

fn render_sections(sections: Vec<(&'static str, Vec<(&'static str, String)>)>) -> String {
    let mut blocks = Vec::new();
    for (title, fields) in sections {
        blocks.push(render_section(title, &fields));
    }
    blocks.join("\n\n")
}

fn render_section(title: &str, fields: &[(&str, String)]) -> String {
    let label_width = fields
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);
    let mut lines = vec![title.to_string()];
    for (label, value) in fields {
        lines.push(format!(
            "  {:width$}: {}",
            label,
            value,
            width = label_width
        ));
    }
    lines.join("\n")
}

fn new_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
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
    if !styles_enabled() {
        return cell;
    }
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

fn styles_enabled() -> bool {
    io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none()
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

fn status_tone(status: UsageStatus) -> CellTone {
    match status {
        UsageStatus::Healthy => CellTone::Good,
        UsageStatus::Warning => CellTone::Warn,
        UsageStatus::Exhausted => CellTone::Bad,
        UsageStatus::Unknown => CellTone::Muted,
    }
}

fn failure_reason_tone(reason: &FailureReason) -> CellTone {
    match reason {
        FailureReason::SessionExhausted | FailureReason::WeeklyExhausted => CellTone::Bad,
        FailureReason::QuotaExhausted | FailureReason::RateLimited | FailureReason::AuthInvalid => {
            CellTone::Warn
        }
        FailureReason::CommandFailed | FailureReason::ValidationFailed | FailureReason::Unknown => {
            CellTone::Muted
        }
    }
}

fn usage_source_label(source: &relay_core::UsageSource) -> &'static str {
    match source {
        relay_core::UsageSource::Local => "Local",
        relay_core::UsageSource::Fallback => "Fallback",
        relay_core::UsageSource::WebEnhanced => "WebEnhanced",
    }
}

fn usage_source_mode_label(mode: &UsageSourceMode) -> &'static str {
    match mode {
        UsageSourceMode::Auto => "Auto",
        UsageSourceMode::Local => "Local",
        UsageSourceMode::WebEnhanced => "WebEnhanced",
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

fn failure_reason_label(reason: &FailureReason) -> &'static str {
    match reason {
        FailureReason::SessionExhausted => "SessionExhausted",
        FailureReason::WeeklyExhausted => "WeeklyExhausted",
        FailureReason::AuthInvalid => "AuthInvalid",
        FailureReason::QuotaExhausted => "QuotaExhausted",
        FailureReason::RateLimited => "RateLimited",
        FailureReason::CommandFailed => "CommandFailed",
        FailureReason::ValidationFailed => "ValidationFailed",
        FailureReason::Unknown => "Unknown",
    }
}

fn auth_mode_label(mode: &AuthMode) -> &'static str {
    match mode {
        AuthMode::ConfigFilesystem => "ConfigFilesystem",
        AuthMode::EnvReference => "EnvReference",
        AuthMode::KeychainReference => "KeychainReference",
    }
}

fn agent_kind_label(kind: &AgentKind) -> &'static str {
    match kind {
        AgentKind::Codex => "Codex",
    }
}

fn probe_provider_label(provider: &ProbeProvider) -> &'static str {
    match provider {
        ProbeProvider::CodexOfficial => "CodexOfficial",
    }
}

fn switch_outcome_label(outcome: &SwitchOutcome) -> &'static str {
    match outcome {
        SwitchOutcome::NotRun => "NotRun",
        SwitchOutcome::Success => "Success",
        SwitchOutcome::Failed => "Failed",
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn user_facing_usage_note(snapshot: &UsageSnapshot) -> Option<String> {
    snapshot
        .message
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn active_state_fields(state: &ActiveState) -> Vec<(&'static str, String)> {
    vec![
        (
            "Active Profile",
            state
                .active_profile_id
                .clone()
                .unwrap_or_else(|| "-".into()),
        ),
        (
            "Last Switch At",
            format_optional_datetime(state.last_switch_at),
        ),
        (
            "Last Switch Result",
            switch_outcome_label(&state.last_switch_result).into(),
        ),
        (
            "Auto-switch Enabled",
            yes_no(state.auto_switch_enabled).into(),
        ),
        (
            "Last Error",
            state.last_error.clone().unwrap_or_else(|| "-".into()),
        ),
    ]
}

fn app_settings_fields(settings: &AppSettings) -> Vec<(&'static str, String)> {
    vec![
        (
            "Auto-switch Enabled",
            yes_no(settings.auto_switch_enabled).into(),
        ),
        ("Cooldown Seconds", settings.cooldown_seconds.to_string()),
    ]
}

fn autoswitch_fields(settings: &AppSettings) -> Vec<(&'static str, String)> {
    vec![
        (
            "Auto-switch Enabled",
            yes_no(settings.auto_switch_enabled).into(),
        ),
        ("Cooldown Seconds", settings.cooldown_seconds.to_string()),
    ]
}

fn codex_settings_fields(settings: &CodexSettings) -> Vec<(&'static str, String)> {
    vec![(
        "Usage Source Mode",
        usage_source_mode_label(&settings.usage_source_mode).into(),
    )]
}

fn probe_identity_fields(identity: &ProfileProbeIdentity) -> Vec<(&'static str, String)> {
    vec![
        ("Profile ID", identity.profile_id.clone()),
        ("Provider", probe_provider_label(&identity.provider).into()),
        (
            "Principal ID",
            identity.principal_id.clone().unwrap_or_else(|| "-".into()),
        ),
        (
            "Display Name",
            identity.display_name.clone().unwrap_or_else(|| "-".into()),
        ),
        ("Account ID", identity.account_id().unwrap_or("-").into()),
        ("Email", identity.email().unwrap_or("-").into()),
        ("Plan", identity.plan_hint().unwrap_or("-").into()),
        ("Created", identity.created_at.clone()),
        ("Updated", identity.updated_at.clone()),
    ]
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
            &[args.nickname.is_some(), args.priority.is_some()],
        )?;
        let payload: LoginProfileInput = read_json_input(input_json)?;
        return Ok(AgentLoginRequest {
            agent: AgentKind::Codex,
            nickname: payload.nickname,
            priority: payload.priority,
        });
    }

    Ok(AgentLoginRequest {
        agent: AgentKind::Codex,
        nickname: args.nickname,
        priority: args.priority.unwrap_or(100),
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
        });
    }

    Ok(SystemSettingsUpdateRequest {
        auto_switch_enabled: Some(require_field(
            args.enabled,
            "autoswitch enabled value is required",
        )?),
    })
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
    match value {
        "session-exhausted" | "SessionExhausted" => Ok(FailureReason::SessionExhausted),
        "weekly-exhausted" | "WeeklyExhausted" => Ok(FailureReason::WeeklyExhausted),
        "auth-invalid" | "AuthInvalid" => Ok(FailureReason::AuthInvalid),
        "quota-exhausted" | "QuotaExhausted" => Ok(FailureReason::QuotaExhausted),
        "rate-limited" | "RateLimited" => Ok(FailureReason::RateLimited),
        "command-failed" | "CommandFailed" => Ok(FailureReason::CommandFailed),
        "validation-failed" | "ValidationFailed" => Ok(FailureReason::ValidationFailed),
        "unknown" | "Unknown" => Ok(FailureReason::Unknown),
        other => Err(RelayError::InvalidInput(format!(
            "unsupported failure reason: {other}"
        ))),
    }
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

struct Output {
    json: bool,
    text: String,
    body: String,
    rendered_body: Option<String>,
}

impl Output {
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
            print!(
                "{}",
                self.rendered_body.as_deref().unwrap_or(self.body.as_str())
            );
            Ok(())
        }
    }
}
