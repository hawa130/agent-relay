use super::common::*;
use super::*;

pub(crate) fn render_usage_list(
    snapshots: &[UsageSnapshot],
    profiles: &[relay_core::ProfileListItem],
) -> String {
    let mut table = new_table();
    table.set_header(vec!["Profile", "State", "Session", "Weekly", "Next Reset"]);

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
            styled_cell(
                list_window_label(&snapshot.session, false),
                status_tone(snapshot.session.status),
            ),
            styled_cell(
                list_window_label(&snapshot.weekly, true),
                status_tone(snapshot.weekly.status),
            ),
            Cell::new(format_optional_datetime(snapshot.next_reset_at)),
        ]));
    }

    table.to_string()
}

pub(crate) fn render_usage_detail(snapshot: &UsageSnapshot) -> String {
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

pub(super) fn usage_fields(snapshot: &UsageSnapshot) -> Vec<(&'static str, String)> {
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

pub(super) fn list_window_label(window: &UsageWindow, coarse_hours: bool) -> String {
    match (window.used_percent, window.reset_at) {
        (Some(percent), Some(reset_at)) => {
            format!(
                "{percent:.0}% · {}",
                compact_reset_label(reset_at, coarse_hours)
            )
        }
        (Some(percent), None) => format!("{percent:.0}%"),
        (None, _) => "-".into(),
    }
}

fn compact_reset_label(reset_at: DateTime<Utc>, coarse_hours: bool) -> String {
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
    if hours > 0 || (coarse_hours && days == 0) {
        parts.push(format!("{hours}h"));
    }
    if !coarse_hours && (minutes > 0 || parts.is_empty()) {
        parts.push(format!("{minutes}m"));
    }
    if parts.is_empty() {
        "1h".into()
    } else {
        parts.join("")
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
