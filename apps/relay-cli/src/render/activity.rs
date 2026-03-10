use super::common::*;
use super::*;

pub(crate) fn render_switch_report(report: &SwitchReport) -> String {
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

pub(crate) fn render_failure_events(
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

pub(crate) fn render_log_tail(log: &LogTail) -> String {
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

pub(crate) fn render_diagnostics_export(export: &DiagnosticsExport) -> String {
    render_sections(vec![(
        "Diagnostics",
        vec![
            ("Archive Path", export.archive_path.clone()),
            ("Bundle Dir", export.bundle_dir.clone()),
            ("Created At", format_datetime(export.created_at)),
        ],
    )])
}
