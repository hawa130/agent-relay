import SwiftUI

struct AddProfileAgentRow: View {
    let descriptor: AgentSettingsDescriptor
    let profileCountText: String
    let isBusy: Bool
    let onImportProfile: @MainActor () async -> Void
    let onAddAccount: () -> Void

    var body: some View {
        HStack(alignment: .top, spacing: 6) {
            ZStack {
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(NativePreferencesTheme.Colors.subtleFill)
                    .frame(width: 28, height: 28)

                AgentIcon(agent: descriptor.agent, size: 16, tint: .secondary)
            }
            .frame(width: 28, height: 28)

            VStack(alignment: .leading, spacing: 0) {
                Text(descriptor.title)
                    .font(.system(size: 14, weight: .regular))

                Text(profileCountText)
                    .font(.system(size: 11, weight: .regular))
                    .foregroundStyle(.secondary)

                HStack(spacing: 6) {
                    Spacer(minLength: 0)

                    Button("Import Current Config") {
                        Task { @MainActor in
                            await onImportProfile()
                        }
                    }
                    .disabled(isBusy)

                    Button("Add Account...") {
                        onAddAccount()
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(isBusy)
                }
                .padding(.top, 4)
            }
        }
    }
}
