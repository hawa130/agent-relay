import SwiftUI

struct AddProfileCatalogView: View {
    let agents: [AgentSettingsDescriptor]
    let isBusy: Bool
    let profileCountText: (AgentSettingsDescriptor) -> String
    let onImportProfile: @MainActor (AgentKind) async -> Void
    let onAddAccount: (AgentKind) -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Form {
                ForEach(agents) { descriptor in
                    AddProfileAgentRow(
                        descriptor: descriptor,
                        profileCountText: profileCountText(descriptor),
                        isBusy: isBusy,
                        onImportProfile: {
                            await onImportProfile(descriptor.agent)
                        },
                        onAddAccount: {
                            onAddAccount(descriptor.agent)
                        })
                }
            }
            .formStyle(.grouped)
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            HStack {
                Spacer()

                Button("Cancel") {
                    onCancel()
                }
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 14)
        }
    }
}
