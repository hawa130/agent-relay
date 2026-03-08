use clap::{Args, Parser, Subcommand};
use relay_core::models::JsonResponse;
use relay_core::{
    AddProfileRequest, AuthMode, BootstrapMode, CodexLoginRequest, EditProfileRequest, RelayApp,
    RelayError, UsageSettingsUpdateRequest, UsageSourceMode,
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
    ImportCodex(ImportCodexArgs),
    LoginCodex(LoginCodexArgs),
    RelinkCodex(ProfileIdArgs),
}

#[derive(Debug, Args)]
struct AddProfileArgs {
    #[arg(long)]
    nickname: Option<String>,
    #[arg(long)]
    priority: Option<i32>,
    #[arg(long)]
    config_path: Option<PathBuf>,
    #[arg(long)]
    codex_home: Option<PathBuf>,
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
struct ImportCodexArgs {
    #[arg(long)]
    nickname: Option<String>,
    #[arg(long)]
    priority: Option<i32>,
    #[arg(long)]
    input_json: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct LoginCodexArgs {
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
    codex_home: Option<PathBuf>,
    #[arg(long)]
    clear_codex_home: bool,
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
            None => Ok(Output::success(
                "usage loaded",
                app.usage_report()?,
                cli.json,
            )),
            Some(UsageSubcommand::Profile(args)) => Ok(Output::success(
                "profile usage loaded",
                app.profile_usage_report(&profile_id_from_args(args)?)?,
                cli.json,
            )),
            Some(UsageSubcommand::List) => Ok(Output::success(
                "usage list loaded",
                app.list_usage_reports()?,
                cli.json,
            )),
            Some(UsageSubcommand::Refresh(args)) => {
                let target = usage_refresh_target_from_args(args)?;
                match target {
                    UsageRefreshTarget::Profile(id) => Ok(Output::success(
                        "usage refreshed",
                        app.refresh_usage_profile(&id)?,
                        cli.json,
                    )),
                    UsageRefreshTarget::Enabled => Ok(Output::success(
                        "enabled profile usage refreshed",
                        app.refresh_enabled_usage_reports()?,
                        cli.json,
                    )),
                    UsageRefreshTarget::All => Ok(Output::success(
                        "all profile usage refreshed",
                        app.refresh_all_usage_reports()?,
                        cli.json,
                    )),
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
            ProfilesSubcommand::ImportCodex(args) => {
                let payload = import_codex_input_from_args(args)?;
                Ok(Output::success(
                    "codex profile imported",
                    app.import_codex_profile(payload.nickname, payload.priority)?,
                    cli.json,
                ))
            }
            ProfilesSubcommand::LoginCodex(args) => {
                let payload = login_codex_input_from_args(args)?;
                Ok(Output::success(
                    "codex login profile created",
                    app.login_codex_profile(CodexLoginRequest {
                        nickname: payload.nickname,
                        priority: payload.priority,
                    })?,
                    cli.json,
                ))
            }
            ProfilesSubcommand::RelinkCodex(args) => Ok(Output::success(
                "codex profile relinked",
                app.relink_codex_profile(&profile_id_from_args(args)?)?,
                cli.json,
            )),
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

fn add_profile_request_from_args(args: AddProfileArgs) -> Result<AddProfileRequest, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[
                args.nickname.is_some(),
                args.priority.is_some(),
                args.config_path.is_some(),
                args.codex_home.is_some(),
                args.auth_mode.is_some(),
            ],
        )?;
        let payload: AddProfileInput = read_json_input(input_json)?;
        return Ok(AddProfileRequest {
            nickname: payload.nickname,
            priority: payload.priority,
            config_path: payload.config_path,
            agent_home: payload.agent_home,
            auth_mode: payload.auth_mode,
        });
    }

    Ok(AddProfileRequest {
        nickname: require_field(args.nickname, "profile nickname is required")?,
        priority: args.priority.unwrap_or(100),
        config_path: args.config_path,
        agent_home: args.codex_home,
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
                args.codex_home.is_some(),
                args.clear_codex_home,
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
            agent_home: if args.clear_codex_home {
                Some(None)
            } else {
                args.codex_home.map(Some)
            },
            auth_mode,
        },
    ))
}

fn import_codex_input_from_args(args: ImportCodexArgs) -> Result<ImportCodexInput, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[args.nickname.is_some(), args.priority.is_some()],
        )?;
        return read_json_input(input_json);
    }

    Ok(ImportCodexInput {
        nickname: args.nickname,
        priority: args.priority.unwrap_or(100),
    })
}

fn login_codex_input_from_args(args: LoginCodexArgs) -> Result<LoginCodexInput, RelayError> {
    if let Some(input_json) = args.input_json.as_ref() {
        ensure_json_input_is_exclusive(
            input_json,
            &[args.nickname.is_some(), args.priority.is_some()],
        )?;
        return read_json_input(input_json);
    }

    Ok(LoginCodexInput {
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
    nickname: String,
    #[serde(default = "default_priority")]
    priority: i32,
    config_path: Option<PathBuf>,
    #[serde(alias = "codex_home")]
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
    #[serde(alias = "codex_home")]
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
struct LoginCodexInput {
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

#[derive(Debug, Deserialize)]
struct ImportCodexInput {
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
}

impl Output {
    fn success<T: Serialize>(message: &str, data: T, json: bool) -> Self {
        let body = serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string());
        Self {
            json,
            text: message.to_string(),
            body,
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
            println!("{}", self.body);
            Ok(())
        }
    }
}
