use super::common::*;
use super::usage::{list_window_label, usage_fields};
use super::*;

pub(crate) fn render_profiles_list(items: &[relay_core::ProfileListItem]) -> String {
    let mut table = new_table();
    table.set_header(vec![
        "Current",
        "Nickname",
        "Profile ID",
        "Agent",
        "Priority",
        "Status",
        "Account State",
        "Session",
        "Weekly",
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
            styled_cell(profile_status_label(profile), profile_status_tone(profile)),
            styled_cell(
                profile_account_state_label(&profile.account_state),
                profile_account_state_tone(&profile.account_state),
            ),
            styled_cell(
                usage
                    .map(|value| list_window_label(&value.session, false))
                    .unwrap_or_else(|| "-".into()),
                usage
                    .map(|value| status_tone(value.session.status.clone()))
                    .unwrap_or(CellTone::Muted),
            ),
            styled_cell(
                usage
                    .map(|value| list_window_label(&value.weekly, true))
                    .unwrap_or_else(|| "-".into()),
                usage
                    .map(|value| status_tone(value.weekly.status.clone()))
                    .unwrap_or(CellTone::Muted),
            ),
        ]));
    }

    table.to_string()
}

pub(crate) fn render_profile_detail(profile: &Profile) -> String {
    render_sections(vec![(
        "Profile",
        vec![
            ("Nickname", profile.nickname.clone()),
            ("Profile ID", profile.id.clone()),
            ("Agent", agent_kind_label(&profile.agent).into()),
            ("Priority", profile.priority.to_string()),
            ("Status", profile_status_label(profile).into()),
            (
                "Account State",
                profile_account_state_label(&profile.account_state).into(),
            ),
            (
                "Account HTTP Status",
                profile
                    .account_error_http_status
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".into()),
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

pub(crate) fn render_profile_summary(detail: &ProfileDetail) -> String {
    let mut sections = vec![(
        "Profile",
        vec![
            ("Nickname", detail.profile.nickname.clone()),
            ("Profile ID", detail.profile.id.clone()),
            ("Agent", agent_kind_label(&detail.profile.agent).into()),
            ("Active", yes_no(detail.is_active).into()),
            ("Status", profile_status_label(&detail.profile).into()),
            (
                "Account State",
                profile_account_state_label(&detail.profile.account_state).into(),
            ),
            (
                "Account HTTP Status",
                detail
                    .profile
                    .account_error_http_status
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".into()),
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

    if !detail.current_failure_events.is_empty() {
        sections.push((
            "Current Status",
            detail
                .current_failure_events
                .iter()
                .map(|event| {
                    (
                        failure_reason_label(&event.reason),
                        format!("{} ({})", event.message, format_datetime(event.created_at)),
                    )
                })
                .collect(),
        ));
    }

    render_sections(sections)
}

pub(crate) fn render_agent_link_result(result: &AgentLinkResult) -> String {
    let mut sections = vec![
        (
            "Profile",
            vec![
                ("Nickname", result.profile.nickname.clone()),
                ("Profile ID", result.profile.id.clone()),
                ("Agent", agent_kind_label(&result.profile.agent).into()),
                ("Status", profile_status_label(&result.profile).into()),
                (
                    "Account State",
                    profile_account_state_label(&result.profile.account_state).into(),
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

pub(crate) fn render_probe_identity(identity: &ProfileProbeIdentity) -> String {
    render_sections(vec![("Probe Identity", probe_identity_fields(identity))])
}

pub(crate) fn render_profile_recovery_report(report: &ProfileRecoveryReport) -> String {
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

fn profile_status_label(profile: &Profile) -> &'static str {
    if profile.enabled {
        "enabled"
    } else {
        "disabled"
    }
}

fn profile_status_tone(profile: &Profile) -> CellTone {
    if profile.enabled {
        CellTone::Good
    } else {
        CellTone::Muted
    }
}
