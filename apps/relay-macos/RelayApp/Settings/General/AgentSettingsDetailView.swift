import SwiftUI

struct AgentSettingsDetailView: View {
    let descriptor: AgentSettingsDescriptor
    @ObservedObject var model: SettingsPaneModel

    var body: some View {
        Form {
            switch descriptor.agent {
            case .codex:
                Section("Usage") {
                    Picker(
                        "Usage source",
                        selection: Binding(
                            get: { model.codexSettings?.usageSourceMode ?? .auto },
                            set: { mode in
                                Task {
                                    await model.setUsageSourceMode(mode)
                                }
                            })) {
                        ForEach(UsageSourceMode.allCases, id: \.self) { mode in
                            Text(mode.displayName).tag(mode)
                        }
                    }
                    .pickerStyle(.segmented)

                    Text((model.codexSettings?.usageSourceMode ?? .auto).helpText)
                        .font(NativePreferencesTheme.Typography.detail)
                        .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .formStyle(.grouped)
    }
}
