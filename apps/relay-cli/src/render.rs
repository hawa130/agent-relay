use super::*;

pub(super) fn render_doctor_report(report: &DoctorReport) -> String {
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

pub(super) fn render_status_report(report: &SystemStatusReport) -> String {
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

pub(super) fn render_usage_list(
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

pub(super) fn render_usage_detail(snapshot: &UsageSnapshot) -> String {
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

pub(super) fn render_settings(settings: &AppSettings) -> String {
    render_sections(vec![("Settings", app_settings_fields(settings))])
}

pub(super) fn render_codex_settings(settings: &CodexSettings) -> String {
    render_sections(vec![("Codex Settings", codex_settings_fields(settings))])
}

pub(super) fn render_autoswitch_settings(settings: &AppSettings) -> String {
    render_sections(vec![("Autoswitch", autoswitch_fields(settings))])
}

pub(super) fn render_profiles_list(items: &[relay_core::ProfileListItem]) -> String {
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

pub(super) fn render_profile_detail(profile: &Profile) -> String {
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

pub(super) fn render_profile_summary(detail: &ProfileDetail) -> String {
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

pub(super) fn render_agent_link_result(result: &AgentLinkResult) -> String {
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

pub(super) fn render_probe_identity(identity: &ProfileProbeIdentity) -> String {
    render_sections(vec![("Probe Identity", probe_identity_fields(identity))])
}

pub(super) fn render_profile_recovery_report(report: &ProfileRecoveryReport) -> String {
    let mut sections = vec![(
        "Recovery",
        vec![
            ("Scanned Dirs", report.scanned_dirs.to_string()),
            ("Recovered", report.recovered.len().to_string()),
            ("Skipped", report.skipped.len().to_string()),
        ],
    )];

    if !report.recovered.is_empty() {
        sections.push((
            "Recovered Profiles",
            report
                .recovered
                .iter()
                .map(recovered_profile_field)
                .collect(),
        ));
    }

    if !report.skipped.is_empty() {
        sections.push((
            "Skipped Profiles",
            report.skipped.iter().map(skipped_profile_field).collect(),
        ));
    }

    render_sections(sections)
}

fn recovered_profile_field(profile: &relay_core::RecoveredProfile) -> (&'static str, String) {
    (
        "Recovered",
        format!(
            "{} ({}) [{}]",
            profile.profile.nickname,
            profile.profile.id,
            if profile.probe_identity_restored {
                "identity restored"
            } else {
                "identity unavailable"
            }
        ),
    )
}

fn skipped_profile_field(profile: &relay_core::SkippedRecoveredProfile) -> (&'static str, String) {
    (
        "Skipped",
        format!("{} ({})", profile.source_dir, profile.reason),
    )
}

pub(super) fn render_switch_report(report: &SwitchReport) -> String {
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

pub(super) fn render_failure_events(
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

pub(super) fn render_log_tail(log: &LogTail) -> String {
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

pub(super) fn render_diagnostics_export(export: &DiagnosticsExport) -> String {
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
