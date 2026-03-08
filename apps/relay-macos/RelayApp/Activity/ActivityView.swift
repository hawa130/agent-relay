import SwiftUI

struct ActivityView: View {
    @ObservedObject var model: RelayAppModel

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            toolbar
            HStack(alignment: .top, spacing: 16) {
                eventsPanel
                logsPanel
            }
            diagnosticsPanel
        }
        .padding(20)
        .task {
            await model.refresh()
        }
    }

    private var toolbar: some View {
        HStack {
            Text("Activity")
                .font(.title2)
            Spacer()
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

    private var eventsPanel: some View {
        GroupBox("Recent Events") {
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
        GroupBox("Recent Logs") {
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
        GroupBox("Diagnostics") {
            VStack(alignment: .leading, spacing: 8) {
                Text(model.diagnosticsExport?.archivePath ?? "No diagnostics export generated yet.")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                if let error = model.lastErrorMessage {
                    Text(error)
                        .font(.caption)
                        .foregroundStyle(.red)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
