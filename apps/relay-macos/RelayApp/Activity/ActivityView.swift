import SwiftUI

struct ActivityView: View {
    @ObservedObject var model: ActivityPaneModel

    var body: some View {
        NativePaneScrollView {
            VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionSpacing) {
                paneHeader(
                    title: "Activity",
                    subtitle: "Inspect recent events, logs, and diagnostics exports."
                )

                SettingsSurfaceCard("Controls") {
                    HStack {
                        Button("Refresh") {
                            Task {
                                await model.refresh()
                            }
                        }

                        Button("Export Diagnostics") {
                            Task {
                                await model.exportDiagnostics()
                            }
                        }
                    }
                }

                HStack(alignment: .top, spacing: 12) {
                    eventsPanel
                    logsPanel
                }

                diagnosticsPanel
            }
        }
    }

    private var eventsPanel: some View {
        SettingsSurfaceCard("Recent Events") {
            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    if model.events.isEmpty {
                        Text("No failure events recorded.")
                            .foregroundStyle(.secondary)
                    } else {
                        ForEach(model.events) { event in
                            VStack(alignment: .leading, spacing: 4) {
                                Text(event.message)
                                Text("\(event.reason.rawValue) at \(event.createdAt.formatted())")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var logsPanel: some View {
        SettingsSurfaceCard("Recent Logs") {
            ScrollView {
                VStack(alignment: .leading, spacing: 8) {
                    if let lines = model.logTail?.lines, !lines.isEmpty {
                        ForEach(Array(lines.enumerated()), id: \.offset) { _, line in
                            Text(line)
                                .font(.system(.caption, design: .monospaced))
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    } else {
                        Text("No log lines available.")
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var diagnosticsPanel: some View {
        SettingsSurfaceCard("Diagnostics") {
            VStack(alignment: .leading, spacing: 8) {
                Text(model.diagnosticsExport?.archivePath ?? "No diagnostics export generated yet.")
                    .font(NativePreferencesTheme.Typography.detail)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)

                if let error = model.lastErrorMessage {
                    Text(error)
                        .font(NativePreferencesTheme.Typography.detail)
                        .foregroundStyle(.red)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
