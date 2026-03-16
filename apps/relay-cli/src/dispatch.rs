use super::render::{
    render_agent_link_result, render_autoswitch_settings, render_codex_settings,
    render_diagnostics_export, render_doctor_report, render_failure_events, render_log_tail,
    render_probe_identity, render_profile_detail, render_profile_recovery_report,
    render_profile_summary, render_profiles_list, render_settings, render_status_report,
    render_switch_report, render_usage_detail, render_usage_list,
};
use super::*;

pub(super) async fn execute(cli: Cli) -> Result<Output, RelayError> {
    let app = RelayApp::bootstrap_with_mode(bootstrap_mode_for_command(&cli.command)).await?;
    dispatch(cli, app).await
}

fn bootstrap_mode_for_command(command: &Commands) -> BootstrapMode {
    match command {
        Commands::Doctor
        | Commands::Daemon(_)
        | Commands::Status
        | Commands::List
        | Commands::Show(_)
        | Commands::Settings(SettingsCommand {
            command: None | Some(SettingsSubcommand::Show),
        }) => BootstrapMode::ReadOnly,
        Commands::Settings(SettingsCommand {
            command: Some(SettingsSubcommand::Set(_)),
        }) => BootstrapMode::ReadWrite,
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
            | CodexSubcommand::Recover
            | CodexSubcommand::Relink(_) => BootstrapMode::ReadWrite,
        },
        Commands::Activity(command) => match &command.command {
            ActivitySubcommand::Events(_) | ActivitySubcommand::Logs(_) => BootstrapMode::ReadOnly,
            ActivitySubcommand::Diagnostics(_) => BootstrapMode::ReadWrite,
        },
    }
}

async fn dispatch(cli: Cli, app: RelayApp) -> Result<Output, RelayError> {
    match cli.command {
        Commands::Daemon(_) => unreachable!("daemon command is handled before CLI dispatch entry"),
        Commands::Doctor => {
            let report = app.doctor_report()?;
            let rendered = render_doctor_report(&report);
            Ok(Output::success_rendered(
                "doctor completed",
                report,
                rendered,
                cli.json,
            ))
        }
        Commands::Status => {
            let report = app.status_report().await?;
            let rendered = render_status_report(&report);
            Ok(Output::success_rendered(
                "status loaded",
                report,
                rendered,
                cli.json,
            ))
        }
        Commands::Settings(command) => match command.command {
            None | Some(SettingsSubcommand::Show) => {
                let settings = app.settings().await?;
                let rendered = render_settings(&settings);
                Ok(Output::success_rendered(
                    "settings loaded",
                    settings,
                    rendered,
                    cli.json,
                ))
            }
            Some(SettingsSubcommand::Set(args)) => {
                let settings = app
                    .update_system_settings(settings_request_from_args(args)?)
                    .await?;
                let rendered = render_settings(&settings);
                Ok(Output::success_rendered(
                    "settings updated",
                    settings,
                    rendered,
                    cli.json,
                ))
            }
        },
        Commands::List => list_output(&app, cli.json).await,
        Commands::Show(command) => {
            let detail = match show_target_from_args(command)? {
                ShowTarget::Current => app.current_profile_detail().await?,
                ShowTarget::Profile(id) => app.profile_detail(&id).await?,
            };
            let rendered = render_profile_summary(&detail);
            Ok(Output::success_rendered(
                "profile detail loaded",
                detail,
                rendered,
                cli.json,
            ))
        }
        Commands::Edit(args) => {
            let (id, request) = edit_profile_request_from_args(args)?;
            let profile = app.edit_profile(&id, request).await?;
            let rendered = render_profile_detail(&profile);
            Ok(Output::success_rendered(
                "profile updated",
                profile,
                rendered,
                cli.json,
            ))
        }
        Commands::Remove(args) => {
            let profile = app.remove_profile(&profile_id_from_args(args)?).await?;
            let rendered = render_profile_detail(&profile);
            Ok(Output::success_rendered(
                "profile removed",
                profile,
                rendered,
                cli.json,
            ))
        }
        Commands::Enable(args) => {
            let profile = app
                .set_profile_enabled(&profile_id_from_args(args)?, true)
                .await?;
            let rendered = render_profile_detail(&profile);
            Ok(Output::success_rendered(
                "profile enabled",
                profile,
                rendered,
                cli.json,
            ))
        }
        Commands::Disable(args) => {
            let profile = app
                .set_profile_enabled(&profile_id_from_args(args)?, false)
                .await?;
            let rendered = render_profile_detail(&profile);
            Ok(Output::success_rendered(
                "profile disabled",
                profile,
                rendered,
                cli.json,
            ))
        }
        Commands::Switch(command) => {
            let report = match switch_target_from_args(command)? {
                SwitchTarget::Next => app.switch_next_profile().await?,
                SwitchTarget::Profile(id) => app.switch_to_profile(&id).await?,
            };
            let rendered = render_switch_report(&report);
            Ok(Output::success_rendered(
                "switch completed",
                report,
                rendered,
                cli.json,
            ))
        }
        Commands::Refresh(command) => match refresh_target_from_args(command)? {
            RefreshTarget::Profile(id) => {
                let snapshot = app.refresh_usage_profile(&id).await?;
                let rendered = render_usage_detail(&snapshot);
                Ok(Output::success_rendered(
                    "profile refreshed",
                    snapshot,
                    rendered,
                    cli.json,
                ))
            }
            RefreshTarget::Enabled => {
                let snapshots = app.refresh_enabled_usage_reports().await?;
                let items = app.list_profiles_with_usage().await?;
                let rendered = render_usage_list(&snapshots, &items);
                Ok(Output::success_rendered(
                    "enabled profiles refreshed",
                    snapshots,
                    rendered,
                    cli.json,
                ))
            }
            RefreshTarget::All => {
                let snapshots = app.refresh_all_usage_reports().await?;
                let items = app.list_profiles_with_usage().await?;
                let rendered = render_usage_list(&snapshots, &items);
                Ok(Output::success_rendered(
                    "all profiles refreshed",
                    snapshots,
                    rendered,
                    cli.json,
                ))
            }
        },
        Commands::Autoswitch(command) => match command.command {
            None | Some(AutoswitchSubcommand::Show) => {
                let settings = app.settings().await?;
                let rendered = render_autoswitch_settings(&settings);
                Ok(Output::success_rendered(
                    "autoswitch status loaded",
                    settings,
                    rendered,
                    cli.json,
                ))
            }
            Some(AutoswitchSubcommand::Enable) => {
                let settings = app.set_auto_switch_enabled(true).await?;
                let rendered = render_autoswitch_settings(&settings);
                Ok(Output::success_rendered(
                    "autoswitch enabled",
                    settings,
                    rendered,
                    cli.json,
                ))
            }
            Some(AutoswitchSubcommand::Disable) => {
                let settings = app.set_auto_switch_enabled(false).await?;
                let rendered = render_autoswitch_settings(&settings);
                Ok(Output::success_rendered(
                    "autoswitch disabled",
                    settings,
                    rendered,
                    cli.json,
                ))
            }
            Some(AutoswitchSubcommand::Set(args)) => {
                let settings = app
                    .update_system_settings(system_settings_request_from_args(args)?)
                    .await?;
                let rendered = render_autoswitch_settings(&settings);
                Ok(Output::success_rendered(
                    "autoswitch updated",
                    settings,
                    rendered,
                    cli.json,
                ))
            }
        },
        Commands::Activity(command) => match command.command {
            ActivitySubcommand::Events(command) => match command.command {
                ActivityEventsSubcommand::List(args) => {
                    let query = activity_events_query_from_args(args)?;
                    let events = app.list_activity_events(query).await?;
                    let items = app.list_profiles_with_usage().await?;
                    let rendered = render_failure_events(&events, &items);
                    Ok(Output::success_rendered(
                        "activity events loaded",
                        events,
                        rendered,
                        cli.json,
                    ))
                }
            },
            ActivitySubcommand::Logs(command) => match command.command {
                LogsSubcommand::Tail(args) => {
                    let logs = app.logs_tail(log_lines_from_args(args)?).await?;
                    let rendered = render_log_tail(&logs);
                    Ok(Output::success_rendered(
                        "activity logs loaded",
                        logs,
                        rendered,
                        cli.json,
                    ))
                }
            },
            ActivitySubcommand::Diagnostics(command) => match command.command {
                DiagnosticsSubcommand::Export => {
                    let export = app.diagnostics_export().await?;
                    let rendered = render_diagnostics_export(&export);
                    Ok(Output::success_rendered(
                        "activity diagnostics exported",
                        export,
                        rendered,
                        cli.json,
                    ))
                }
            },
        },
        Commands::Codex(command) => match command.command {
            CodexSubcommand::Add(args) => {
                let profile = app.add_profile(codex_add_request_from_args(args)?).await?;
                let rendered = render_profile_detail(&profile);
                Ok(Output::success_rendered(
                    "codex profile created",
                    profile,
                    rendered,
                    cli.json,
                ))
            }
            CodexSubcommand::Import(args) => {
                let payload = codex_import_request_from_args(args)?;
                let profile = app.import_profile(payload).await?;
                let rendered = render_profile_detail(&profile);
                Ok(Output::success_rendered(
                    "codex profile imported",
                    profile,
                    rendered,
                    cli.json,
                ))
            }
            CodexSubcommand::Login(args) => {
                let payload = codex_login_request_from_args(args)?;
                let result = app.login_profile(payload).await?;
                let rendered = render_agent_link_result(&result);
                Ok(Output::success_rendered(
                    "codex login profile created",
                    result,
                    rendered,
                    cli.json,
                ))
            }
            CodexSubcommand::Recover => {
                let report = app.recover_profiles(AgentKind::Codex).await?;
                let rendered = render_profile_recovery_report(&report);
                Ok(Output::success_rendered(
                    "codex profiles recovered",
                    report,
                    rendered,
                    cli.json,
                ))
            }
            CodexSubcommand::Relink(args) => {
                let id = profile_id_from_args(args)?;
                let identity = app.relink_profile(AgentKind::Codex, &id).await?;
                let rendered = render_probe_identity(&identity);
                Ok(Output::success_rendered(
                    "codex profile relinked",
                    identity,
                    rendered,
                    cli.json,
                ))
            }
            CodexSubcommand::Settings(command) => match command.command {
                None | Some(CodexSettingsSubcommand::Show) => {
                    let settings = app.codex_settings().await?;
                    let rendered = render_codex_settings(&settings);
                    Ok(Output::success_rendered(
                        "codex settings loaded",
                        settings,
                        rendered,
                        cli.json,
                    ))
                }
                Some(CodexSettingsSubcommand::Set(args)) => {
                    let settings = app
                        .update_codex_settings(codex_settings_request_from_args(args)?)
                        .await?;
                    let rendered = render_codex_settings(&settings);
                    Ok(Output::success_rendered(
                        "codex settings updated",
                        settings,
                        rendered,
                        cli.json,
                    ))
                }
            },
        },
    }
}

async fn list_output(app: &RelayApp, json: bool) -> Result<Output, RelayError> {
    let items = app.list_profiles_with_usage().await?;
    let rendered = render_profiles_list(&items);
    Ok(Output::success_rendered(
        "profiles loaded",
        items,
        rendered,
        json,
    ))
}

pub(super) struct Output {
    json: bool,
    text: String,
    data: serde_json::Value,
    rendered_body: String,
}

impl Output {
    fn success_rendered<T: Serialize>(
        message: &str,
        data: T,
        rendered_body: String,
        json: bool,
    ) -> Self {
        let data = if json {
            serde_json::to_value(&data).unwrap_or(serde_json::Value::Object(Default::default()))
        } else {
            serde_json::Value::Null
        };
        Self {
            json,
            text: message.to_string(),
            data,
            rendered_body,
        }
    }

    pub(super) fn write(self) -> Result<(), RelayError> {
        if self.json {
            JsonResponse::success(self.text, self.data).write_json()?;
            Ok(())
        } else if !self.rendered_body.is_empty() {
            print!("{}", self.rendered_body);
            Ok(())
        } else {
            Ok(())
        }
    }
}
