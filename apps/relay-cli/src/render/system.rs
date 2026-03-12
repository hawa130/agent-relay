use super::common::*;
use super::*;

pub(crate) fn render_doctor_report(report: &DoctorReport) -> String {
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
                ("AgentRelay Home", report.relay_home.clone()),
                ("AgentRelay DB", report.relay_db_path.clone()),
                ("AgentRelay Log", report.relay_log_path.clone()),
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

pub(crate) fn render_status_report(report: &SystemStatusReport) -> String {
    render_sections(vec![
        (
            "AgentRelay",
            vec![
                ("AgentRelay Home", report.relay_home.clone()),
                ("Live Agent Home", report.live_agent_home.clone()),
                ("Profile Count", report.profile_count.to_string()),
            ],
        ),
        ("Active State", active_state_fields(&report.active_state)),
        ("Settings", app_settings_fields(&report.settings)),
    ])
}
