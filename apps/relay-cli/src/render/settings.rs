use super::common::*;
use super::*;

pub(crate) fn render_settings(settings: &AppSettings) -> String {
    render_sections(vec![("Settings", app_settings_fields(settings))])
}

pub(crate) fn render_codex_settings(settings: &CodexSettings) -> String {
    render_sections(vec![("Codex Settings", codex_settings_fields(settings))])
}

pub(crate) fn render_autoswitch_settings(settings: &AppSettings) -> String {
    render_sections(vec![("Autoswitch", autoswitch_fields(settings))])
}
