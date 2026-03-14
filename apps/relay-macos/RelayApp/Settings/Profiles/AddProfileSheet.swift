import SwiftUI

struct AddProfileSheet: View {
    let agents: [AgentSettingsDescriptor]
    let profileCountForAgent: (AgentKind) -> Int
    let isBusy: Bool
    let onImportProfile: @MainActor (AgentKind) async -> Void
    let onStartLogin: @MainActor (AgentKind) async -> AddAccountResult
    let onCancelLogin: @MainActor () async -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var mode: AddProfileSheetMode = .catalog
    @State private var loginAgent: AgentKind?
    @State private var flowState: AddProfileSheetFlowState = .requesting
    @State private var loginTask: Task<Void, Never>?

    var body: some View {
        Group {
            switch mode {
            case .catalog:
                AddProfileCatalogView(
                    agents: agents,
                    isBusy: isBusy,
                    profileCountText: profileCountText(for:),
                    onImportProfile: { agent in
                        await onImportProfile(agent)
                        dismiss()
                    },
                    onAddAccount: startLogin(for:),
                    onCancel: {
                        dismiss()
                    })
            case .login:
                AddProfileLoginView(
                    loginTitle: loginTitle,
                    flowState: flowState,
                    isBusy: isBusy,
                    onSecondaryAction: {
                        if flowState.isRequesting {
                            Task { @MainActor in
                                await onCancelLogin()
                            }
                        } else {
                            showCatalog()
                        }
                    },
                    onPrimaryAction: loginPrimaryAction)
            }
        }
        .frame(width: 440)
        .interactiveDismissDisabled(mode == .login && flowState.isRequesting)
        .onDisappear {
            loginTask?.cancel()
            loginTask = nil
        }
    }

    private func profileCountText(for descriptor: AgentSettingsDescriptor) -> String {
        let count = profileCountForAgent(descriptor.agent)
        return AddProfileCatalogProfileText.text(for: descriptor, profileCount: count)
    }

    private var loginTitle: String {
        loginAgent?.displayName ?? "Add Account"
    }

    private var loginPrimaryAction: (() -> Void)? {
        guard let agent = loginAgent, flowState.primaryActionTitle != nil else {
            return nil
        }

        return {
            startLogin(for: agent)
        }
    }

    private func startLogin(for agent: AgentKind) {
        loginAgent = agent
        mode = .login
        flowState = .requesting
        loginTask?.cancel()
        loginTask = Task { @MainActor in
            let result = await onStartLogin(agent)
            guard !Task.isCancelled else {
                return
            }
            if let nextState = AddProfileSheetFlowState.from(result: result) {
                flowState = nextState
            } else {
                dismiss()
            }
            loginTask = nil
        }
    }

    private func showCatalog() {
        loginTask?.cancel()
        loginTask = nil
        loginAgent = nil
        mode = .catalog
        flowState = .requesting
    }
}
