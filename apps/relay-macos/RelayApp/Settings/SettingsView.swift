import Defaults
import LaunchAtLogin
import SwiftUI

public struct SettingsView: View {
    @ObservedObject var model: RelayAppModel
    @Default(.selectedSettingsSection) private var selectedSectionRaw
    @Default(.selectedProfileID) private var selectedProfileID
    @State private var showingAddSheet = false
    @State private var showingImportSheet = false
    @State private var editingProfile: Profile?
    @State private var deletingProfile: Profile?

    public init(model: RelayAppModel) {
        self.model = model
    }

    public var body: some View {
        NavigationSplitView {
            settingsSidebar
        } detail: {
            detailPane
        }
        .navigationSplitViewStyle(.balanced)
        .task {
            await model.refresh()
        }
        .sheet(isPresented: $showingAddSheet) {
            ProfileEditorSheet(
                title: "Add Profile",
                initialDraft: .empty,
                mode: .create
            ) { draft in
                await model.addProfile(draft)
            }
        }
        .sheet(isPresented: $showingImportSheet) {
            ImportCodexSheet { nickname, priority in
                await model.importCodexProfile(nickname: nickname, priority: priority)
            }
        }
        .sheet(item: $editingProfile) { profile in
            ProfileEditorSheet(
                title: "Edit Profile",
                initialDraft: ProfileDraft(profile: profile),
                mode: .edit(profile)
            ) { draft in
                await model.editProfile(profileID: profile.id, draft: draft)
            }
        }
        .alert("Remove Profile?", isPresented: .constant(deletingProfile != nil), presenting: deletingProfile) { profile in
            Button("Remove", role: .destructive) {
                Task {
                    await model.removeProfile(profile.id)
                    deletingProfile = nil
                }
            }
            Button("Cancel", role: .cancel) {
                deletingProfile = nil
            }
        } message: { profile in
            Text("Remove profile \"\(profile.nickname)\" from Relay?")
        }
    }

    private var settingsSidebar: some View {
        List(selection: sectionSelection) {
            ForEach(SettingsSection.allCases) { section in
                Label(section.title, systemImage: section.symbol)
                    .tag(section.rawValue)
            }
        }
        .listStyle(.sidebar)
    }

    private var detailPane: some View {
        VStack(alignment: .leading, spacing: 18) {
            titleBlock(title: selectedSection.title, subtitle: subtitle(for: selectedSection))

            switch selectedSection {
            case .general:
                generalForm
            case .profiles:
                profilesView
            case .activity:
                activityView
            }
        }
        .padding(28)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private var generalForm: some View {
        Form {
            Section("Relay") {
                LabeledContent("CLI", value: ProcessInfo.processInfo.environment["RELAY_CLI_PATH"] ?? "Bundled relay")
                LabeledContent("Relay Home", value: model.status?.relayHome ?? "-")
                LabeledContent("Live Agent Home", value: model.status?.liveAgentHome ?? "-")
                LabeledContent("Platform", value: model.doctor?.platform ?? "-")
            }

            Section("Behavior") {
                Toggle(
                    "Enable automatic failover",
                    isOn: Binding(
                        get: { model.autoSwitchEnabled },
                        set: { enabled in
                            Task {
                                await model.setAutoSwitch(enabled: enabled)
                            }
                        }
                    )
                )

                LaunchAtLogin.Toggle("Launch at login")
            }

            if let error = model.lastErrorMessage {
                Section("Last Error") {
                    Text(error)
                        .foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
    }

    private var profilesView: some View {
        HStack(alignment: .top, spacing: 20) {
            profilesSidebar
            profileDetail
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private var profilesSidebar: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 10) {
                Button("Import Live Codex") {
                    showingImportSheet = true
                }
                .disabled(model.isMutatingProfiles)

                Button("Add Profile") {
                    showingAddSheet = true
                }
                .buttonStyle(.borderedProminent)
                .disabled(model.isMutatingProfiles)
            }

            List(selection: $selectedProfileID) {
                ForEach(model.profiles) { profile in
                    HStack(alignment: .top, spacing: 8) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(profile.nickname)
                            Text(profile.id)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }

                        Spacer(minLength: 8)

                        if model.activeProfileID == profile.id {
                            Text("Current")
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(.tint)
                        }
                    }
                    .tag(Optional(profile.id))
                }
            }
            .listStyle(.sidebar)
            .frame(minWidth: 260, maxWidth: 300, minHeight: 420)
        }
    }

    private var profileDetail: some View {
        Group {
            if let profile = selectedProfile {
                Form {
                    if model.activeProfileID == profile.id {
                        Section {
                            Label("This is the active profile in use right now.", systemImage: "checkmark.circle.fill")
                                .foregroundStyle(.tint)
                        }
                    }

                    Section("Summary") {
                        LabeledContent("Nickname", value: profile.nickname)
                        LabeledContent("Agent", value: profile.agent.rawValue)
                        LabeledContent("Priority", value: "\(profile.priority)")
                        LabeledContent("Auth Mode", value: profile.authMode.displayName)
                        LabeledContent("Status", value: profile.enabled ? "Enabled" : "Disabled")
                        if model.activeProfileID == profile.id {
                            LabeledContent("Current", value: "Active")
                        }
                    }

                    Section("Paths") {
                        LabeledContent("Agent Home", value: profile.agentHome ?? "-")
                        LabeledContent("Config Path", value: profile.configPath ?? "-")
                    }

                    Section("Actions") {
                        Toggle(
                            "Enabled",
                            isOn: Binding(
                                get: { profile.enabled },
                                set: { enabled in
                                    Task {
                                        await model.setProfileEnabled(profile.id, enabled: enabled)
                                    }
                                }
                            )
                        )
                        .disabled(model.isMutatingProfiles)

                        HStack {
                            Button("Switch") {
                                Task {
                                    await model.switchToProfile(profile.id)
                                }
                            }
                            .disabled(!profile.enabled || model.isSwitching)

                            Button("Edit") {
                                editingProfile = profile
                            }
                            .disabled(model.isMutatingProfiles)

                            Button("Remove", role: .destructive) {
                                deletingProfile = profile
                            }
                            .disabled(model.isMutatingProfiles)
                        }
                    }

                    if let error = model.lastErrorMessage {
                        Section("Last Error") {
                            Text(error)
                                .foregroundStyle(.red)
                        }
                    }
                }
                .formStyle(.grouped)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            } else {
                ContentUnavailableView(
                    "No Profile Selected",
                    systemImage: "sidebar.left",
                    description: Text("Choose a profile from the sidebar to inspect or edit it.")
                )
                .frame(maxWidth: .infinity, minHeight: 420)
            }
        }
        .frame(maxWidth: .infinity, alignment: .topLeading)
    }

    private var activityView: some View {
        Form {
            Section("Controls") {
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

            Section("Recent Events") {
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
                    }
                }
            }

            Section("Recent Logs") {
                if let lines = model.logTail?.lines, !lines.isEmpty {
                    ForEach(Array(lines.enumerated()), id: \.offset) { _, line in
                        Text(line)
                            .font(.system(.caption, design: .monospaced))
                    }
                } else {
                    Text("No log lines available.")
                        .foregroundStyle(.secondary)
                }
            }

            Section("Diagnostics") {
                Text(model.diagnosticsExport?.archivePath ?? "No diagnostics export generated yet.")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                if let error = model.lastErrorMessage {
                    Text(error)
                        .foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
    }

    private var selectedSection: SettingsSection {
        SettingsSection(rawValue: selectedSectionRaw) ?? .general
    }

    private var selectedProfile: Profile? {
        if let selectedProfileID {
            return model.profiles.first { $0.id == selectedProfileID }
        }
        return model.profiles.first
    }

    private var sectionSelection: Binding<String?> {
        Binding(
            get: { selectedSectionRaw },
            set: { value in
                selectedSectionRaw = value ?? SettingsSection.general.rawValue
            }
        )
    }

    private func titleBlock(title: String, subtitle: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.system(size: 28, weight: .semibold, design: .rounded))
            Text(subtitle)
                .foregroundStyle(.secondary)
        }
    }

    private func subtitle(for section: SettingsSection) -> String {
        switch section {
        case .general:
            return "Native preferences for Relay behavior and environment."
        case .profiles:
            return "Manage profiles through Relay CLI without bypassing the execution layer."
        case .activity:
            return "Inspect recent events, logs, and diagnostics exports."
        }
    }
}

private enum ProfileEditorMode {
    case create
    case edit(Profile)
}

private struct ProfileEditorSheet: View {
    let title: String
    let mode: ProfileEditorMode
    let onSave: @MainActor (ProfileDraft) async -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var draft: ProfileDraft

    init(
        title: String,
        initialDraft: ProfileDraft,
        mode: ProfileEditorMode,
        onSave: @escaping @MainActor (ProfileDraft) async -> Void
    ) {
        self.title = title
        self.mode = mode
        self.onSave = onSave
        _draft = State(initialValue: initialDraft)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text(title)
                .font(.title3.weight(.semibold))

            Form {
                Section("Identity") {
                    TextField("Nickname", text: $draft.nickname)
                    Stepper("Priority: \(draft.priority)", value: $draft.priority, in: 0...10_000)
                    Picker("Auth Mode", selection: $draft.authMode) {
                        ForEach(AuthMode.allCases, id: \.self) { mode in
                            Text(mode.displayName).tag(mode)
                        }
                    }
                }

                Section("Paths") {
                    TextField("Agent Home", text: $draft.agentHome)
                    if case .edit = mode {
                        Toggle("Clear Agent Home", isOn: $draft.clearAgentHome)
                    }

                    TextField("Config Path", text: $draft.configPath)
                    if case .edit = mode {
                        Toggle("Clear Config Path", isOn: $draft.clearConfigPath)
                    }
                }
            }
            .formStyle(.grouped)

            HStack {
                Spacer()

                Button("Cancel") {
                    dismiss()
                }

                Button("Save") {
                    Task {
                        await onSave(normalizedDraft)
                        dismiss()
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(draft.nickname.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(24)
        .frame(width: 560)
    }

    private var normalizedDraft: ProfileDraft {
        var copy = draft
        copy.nickname = copy.nickname.trimmingCharacters(in: .whitespacesAndNewlines)
        copy.agentHome = copy.agentHome.trimmingCharacters(in: .whitespacesAndNewlines)
        copy.configPath = copy.configPath.trimmingCharacters(in: .whitespacesAndNewlines)

        switch mode {
        case .create:
            copy.clearAgentHome = false
            copy.clearConfigPath = false
        case let .edit(profile):
            if copy.agentHome.isEmpty, profile.agentHome != nil {
                copy.clearAgentHome = true
            }
            if copy.configPath.isEmpty, profile.configPath != nil {
                copy.clearConfigPath = true
            }
        }

        return copy
    }
}

private struct ImportCodexSheet: View {
    let onImport: @MainActor (String?, Int) async -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var nickname = ""
    @State private var priority = 100

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Import Live Codex")
                .font(.title3.weight(.semibold))

            Form {
                Section("Import") {
                    TextField("Nickname (optional)", text: $nickname)
                    Stepper("Priority: \(priority)", value: $priority, in: 0...10_000)
                }
            }
            .formStyle(.grouped)

            HStack {
                Spacer()

                Button("Cancel") {
                    dismiss()
                }

                Button("Import") {
                    Task {
                        let trimmed = nickname.trimmingCharacters(in: .whitespacesAndNewlines)
                        await onImport(trimmed.isEmpty ? nil : trimmed, priority)
                        dismiss()
                    }
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(24)
        .frame(width: 460)
    }
}
