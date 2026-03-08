use clap::{Args, Parser, Subcommand};
use relay_core::models::JsonResponse;
use relay_core::{AddProfileRequest, AuthMode, EditProfileRequest, RelayApp, RelayError};
use serde::Serialize;
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
}

#[derive(Debug, Args)]
struct AddProfileArgs {
    #[arg(long)]
    nickname: String,
    #[arg(long, default_value_t = 100)]
    priority: i32,
    #[arg(long)]
    config_path: Option<PathBuf>,
    #[arg(long)]
    codex_home: Option<PathBuf>,
    #[arg(long, default_value = "config-filesystem")]
    auth_mode: String,
}

#[derive(Debug, Args)]
struct ProfileIdArgs {
    id: String,
}

#[derive(Debug, Args)]
struct SwitchCommand {
    target: String,
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
    #[arg(long, default_value_t = 100)]
    priority: i32,
}

#[derive(Debug, Args)]
struct EditProfileArgs {
    id: String,
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
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long, default_value_t = 50)]
    limit: usize,
}

#[derive(Debug, Args)]
struct TailArgs {
    #[arg(long, default_value_t = 50)]
    lines: usize,
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
    let app = RelayApp::bootstrap()?;
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
        Commands::Profiles(command) => match command.command {
            ProfilesSubcommand::List => Ok(Output::success(
                "profiles loaded",
                app.list_profiles()?,
                cli.json,
            )),
            ProfilesSubcommand::Add(args) => {
                let auth_mode = parse_auth_mode(&args.auth_mode)?;
                let request = AddProfileRequest {
                    nickname: args.nickname,
                    priority: args.priority,
                    config_path: args.config_path,
                    codex_home: args.codex_home,
                    auth_mode,
                };
                Ok(Output::success(
                    "profile created",
                    app.add_profile(request)?,
                    cli.json,
                ))
            }
            ProfilesSubcommand::Edit(args) => {
                let auth_mode = args.auth_mode.as_deref().map(parse_auth_mode).transpose()?;
                let request = EditProfileRequest {
                    nickname: args.nickname,
                    priority: args.priority,
                    config_path: if args.clear_config_path {
                        Some(None)
                    } else {
                        args.config_path.map(Some)
                    },
                    codex_home: if args.clear_codex_home {
                        Some(None)
                    } else {
                        args.codex_home.map(Some)
                    },
                    auth_mode,
                };
                Ok(Output::success(
                    "profile updated",
                    app.edit_profile(&args.id, request)?,
                    cli.json,
                ))
            }
            ProfilesSubcommand::Remove(args) => Ok(Output::success(
                "profile removed",
                app.remove_profile(&args.id)?,
                cli.json,
            )),
            ProfilesSubcommand::Enable(args) => Ok(Output::success(
                "profile enabled",
                app.set_profile_enabled(&args.id, true)?,
                cli.json,
            )),
            ProfilesSubcommand::Disable(args) => Ok(Output::success(
                "profile disabled",
                app.set_profile_enabled(&args.id, false)?,
                cli.json,
            )),
            ProfilesSubcommand::ImportCodex(args) => Ok(Output::success(
                "codex profile imported",
                app.import_codex_profile(args.nickname, args.priority)?,
                cli.json,
            )),
        },
        Commands::Switch(command) => {
            if command.target == "next" {
                Ok(Output::success(
                    "switch completed",
                    app.switch_next_profile()?,
                    cli.json,
                ))
            } else {
                Ok(Output::success(
                    "switch completed",
                    app.switch_to_profile(&command.target)?,
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
        },
        Commands::Events(command) => match command.command {
            EventsSubcommand::List(args) => Ok(Output::success(
                "events loaded",
                app.list_failure_events(args.limit)?,
                cli.json,
            )),
        },
        Commands::Logs(command) => match command.command {
            LogsSubcommand::Tail(args) => Ok(Output::success(
                "logs loaded",
                app.logs_tail(args.lines)?,
                cli.json,
            )),
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
