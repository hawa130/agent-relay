use super::*;

pub(super) fn render_sections(
    sections: Vec<(&'static str, Vec<(&'static str, String)>)>,
) -> String {
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

pub(super) fn new_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

pub(super) fn format_optional_datetime(value: Option<DateTime<Utc>>) -> String {
    value.map(format_datetime).unwrap_or_else(|| "-".into())
}

pub(super) fn format_datetime(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

pub(super) fn styled_cell(value: impl Into<String>, tone: CellTone) -> Cell {
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

pub(super) fn usage_tone(snapshot: &UsageSnapshot) -> CellTone {
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

pub(super) fn status_tone(status: UsageStatus) -> CellTone {
    match status {
        UsageStatus::Healthy => CellTone::Good,
        UsageStatus::Warning => CellTone::Warn,
        UsageStatus::Exhausted => CellTone::Bad,
        UsageStatus::Unknown => CellTone::Muted,
    }
}

pub(super) fn failure_reason_tone(reason: &FailureReason) -> CellTone {
    match reason {
        FailureReason::SessionExhausted | FailureReason::WeeklyExhausted => CellTone::Bad,
        FailureReason::AccountUnavailable
        | FailureReason::QuotaExhausted
        | FailureReason::RateLimited
        | FailureReason::AuthInvalid => CellTone::Warn,
        FailureReason::CommandFailed | FailureReason::ValidationFailed | FailureReason::Unknown => {
            CellTone::Muted
        }
    }
}

pub(super) fn usage_source_label(source: &relay_core::UsageSource) -> &'static str {
    match source {
        relay_core::UsageSource::Local => "Local",
        relay_core::UsageSource::Fallback => "Fallback",
        relay_core::UsageSource::WebEnhanced => "WebEnhanced",
    }
}

pub(super) fn usage_source_mode_label(mode: &UsageSourceMode) -> &'static str {
    match mode {
        UsageSourceMode::Auto => "Auto",
        UsageSourceMode::Local => "Local",
        UsageSourceMode::WebEnhanced => "WebEnhanced",
    }
}

pub(super) fn usage_status_label(status: &UsageStatus) -> &'static str {
    match status {
        UsageStatus::Healthy => "Healthy",
        UsageStatus::Warning => "Warning",
        UsageStatus::Exhausted => "Exhausted",
        UsageStatus::Unknown => "Unknown",
    }
}

pub(super) fn failure_reason_label(reason: &FailureReason) -> &'static str {
    match reason {
        FailureReason::SessionExhausted => "SessionExhausted",
        FailureReason::WeeklyExhausted => "WeeklyExhausted",
        FailureReason::AccountUnavailable => "AccountUnavailable",
        FailureReason::AuthInvalid => "AuthInvalid",
        FailureReason::QuotaExhausted => "QuotaExhausted",
        FailureReason::RateLimited => "RateLimited",
        FailureReason::CommandFailed => "CommandFailed",
        FailureReason::ValidationFailed => "ValidationFailed",
        FailureReason::Unknown => "Unknown",
    }
}

pub(super) fn profile_account_state_label(state: &relay_core::ProfileAccountState) -> &'static str {
    match state {
        relay_core::ProfileAccountState::Healthy => "Healthy",
        relay_core::ProfileAccountState::AccountUnavailable => "AccountUnavailable",
    }
}

pub(super) fn profile_account_state_tone(state: &relay_core::ProfileAccountState) -> CellTone {
    match state {
        relay_core::ProfileAccountState::Healthy => CellTone::Good,
        relay_core::ProfileAccountState::AccountUnavailable => CellTone::Bad,
    }
}

pub(super) fn auth_mode_label(mode: &AuthMode) -> &'static str {
    match mode {
        AuthMode::ConfigFilesystem => "ConfigFilesystem",
        AuthMode::EnvReference => "EnvReference",
        AuthMode::KeychainReference => "KeychainReference",
    }
}

pub(super) fn agent_kind_label(kind: &AgentKind) -> &'static str {
    match kind {
        AgentKind::Codex => "Codex",
    }
}

pub(super) fn probe_provider_label(provider: &ProbeProvider) -> &'static str {
    match provider {
        ProbeProvider::CodexOfficial => "CodexOfficial",
    }
}

pub(super) fn switch_outcome_label(outcome: &SwitchOutcome) -> &'static str {
    match outcome {
        SwitchOutcome::NotRun => "NotRun",
        SwitchOutcome::Success => "Success",
        SwitchOutcome::Failed => "Failed",
    }
}

pub(super) fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

pub(super) fn user_facing_usage_note(snapshot: &UsageSnapshot) -> Option<String> {
    snapshot
        .message
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn active_state_fields(state: &ActiveState) -> Vec<(&'static str, String)> {
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
    ]
}

pub(super) fn app_settings_fields(settings: &AppSettings) -> Vec<(&'static str, String)> {
    vec![
        (
            "Auto-switch Enabled",
            yes_no(settings.auto_switch_enabled).into(),
        ),
        ("Cooldown Seconds", settings.cooldown_seconds.to_string()),
        (
            "Refresh Interval Seconds",
            refresh_interval_label(settings.refresh_interval_seconds),
        ),
        (
            "Network Query Concurrency",
            settings.network_query_concurrency.to_string(),
        ),
    ]
}

fn refresh_interval_label(seconds: i64) -> String {
    if seconds == 0 {
        "Off".into()
    } else {
        seconds.to_string()
    }
}

pub(super) fn codex_settings_fields(settings: &CodexSettings) -> Vec<(&'static str, String)> {
    vec![(
        "Usage Source Mode",
        usage_source_mode_label(&settings.usage_source_mode).into(),
    )]
}

pub(super) fn probe_identity_fields(
    identity: &ProfileProbeIdentity,
) -> Vec<(&'static str, String)> {
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
        ("Plan", identity.plan_hint().map(|s| capitalize_first(s)).unwrap_or_else(|| "-".into())),
        ("Created", format_datetime(identity.created_at)),
        ("Updated", format_datetime(identity.updated_at)),
    ]
}

#[derive(Clone, Copy)]
pub(super) enum CellTone {
    Info,
    Good,
    Warn,
    Bad,
    Muted,
}

pub(super) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

pub(super) fn format_nickname_with_plan(nickname: &str, usage: Option<&UsageSnapshot>) -> String {
    match usage.and_then(|u| u.plan_hint.as_deref()) {
        Some(plan) => format!("{} ({})", nickname, capitalize_first(plan)),
        None => nickname.to_string(),
    }
}
