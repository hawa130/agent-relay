import SwiftUI

public struct ProfilesSettingsPaneView: View {
    @ObservedObject var model: RelayAppModel
    @State private var showingLoginSheet = false
    @State private var editingProfile: Profile?
    @State private var deletingProfile: Profile?

    public init(model: RelayAppModel) {
        self.model = model
    }

    public var body: some View {
        HStack(spacing: 0) {
            sidebar
            Divider()
            detail
        }
        .background(Color(nsColor: .windowBackgroundColor))
        .task {
            await model.refresh()
        }
        .onAppear {
            SettingsPaneID.persistedSelection = .profiles
        }
        .sheet(isPresented: $showingLoginSheet) {
            AddAccountSheet(
                isBusy: model.isMutatingProfiles,
                onContinue: { priority in
                    await model.addAccount(agent: .codex, priority: priority)
                }
            )
        }
        .sheet(item: $editingProfile) { profile in
            ProfileEditorSheet(
                title: "Edit Profile",
                initialDraft: ProfileDraft(profile: profile),
                mode: .edit(profile)
            ) { draft in
                await model.editProfile(profileId: profile.id, draft: draft)
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

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 6) {
                Text("Profiles")
                    .font(.system(size: 28, weight: .semibold, design: .rounded))
                Text("Manage connected agent accounts and inspect live usage in one place.")
                    .foregroundStyle(.secondary)
            }

            HStack(spacing: 10) {
                Button("Add Account") {
                    showingLoginSheet = true
                }
                .buttonStyle(.borderedProminent)
                .disabled(model.isMutatingProfiles)

                Button("Import Current Live") {
                    Task {
                        await model.importProfile(agent: .codex, nickname: nil, priority: 100)
                    }
                }
                .disabled(model.isMutatingProfiles)
            }

            List(
                selection: Binding(
                    get: { model.selectedProfileId },
                    set: { value in
                        model.selectProfile(value)
                    }
                )
            ) {
                ForEach(model.profiles) { profile in
                    ProfileListRow(
                        profile: profile,
                        usage: model.usageSnapshot(for: profile.id),
                        isActive: model.activeProfileId == profile.id
                    )
                    .tag(Optional(profile.id))
                    .listRowInsets(EdgeInsets(top: 8, leading: 10, bottom: 8, trailing: 10))
                }
            }
            .listStyle(.sidebar)
        }
        .padding(24)
        .frame(minWidth: 330, idealWidth: 340, maxWidth: 360, maxHeight: .infinity, alignment: .topLeading)
        .background(Color(nsColor: .controlBackgroundColor))
    }

    private var detail: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                if let profile = selectedProfile {
                    profileHero(profile)
                    usageCard(profile)
                    settingsCard(profile)
                    if let error = model.lastErrorMessage {
                        SettingsSurfaceCard("Last Error") {
                            Text(error)
                                .foregroundStyle(.red)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                } else {
                    ContentUnavailableView(
                        "No Profile Selected",
                        systemImage: "person.crop.square",
                        description: Text("Choose a profile on the left to inspect its details and actions.")
                    )
                    .frame(maxWidth: .infinity, minHeight: 520)
                }
            }
            .padding(28)
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func profileHero(_ profile: Profile) -> some View {
        SettingsSurfaceCard(profile.nickname) {
            HStack(alignment: .top, spacing: 24) {
                VStack(alignment: .leading, spacing: 12) {
                    settingsRow("Agent", value: profile.agent.rawValue)
                    settingsRow("Priority", value: "\(profile.priority)")
                    settingsRow("Auth Mode", value: profile.authMode.displayName)
                    settingsRow("Status", value: profile.enabled ? "Enabled" : "Disabled")
                    if model.activeProfileId == profile.id {
                        settingsRow("Current", value: "Active")
                    }
                }

                Spacer(minLength: 24)

                VStack(alignment: .trailing, spacing: 10) {
                    HStack(spacing: 10) {
                        Button("Refresh Usage") {
                            Task {
                                await model.refreshUsage(profileId: profile.id)
                            }
                        }

                        Button("Activate") {
                            Task {
                                await model.switchToProfile(profile.id)
                            }
                        }
                        .buttonStyle(.borderedProminent)
                        .disabled(!profile.enabled || model.isSwitching)
                    }

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
                    .toggleStyle(.switch)
                    .disabled(model.isMutatingProfiles)
                }
            }

            Divider()

            HStack(spacing: 12) {
                Button("Edit") {
                    editingProfile = profile
                }
                .disabled(model.isMutatingProfiles)

                Button("Remove", role: .destructive) {
                    deletingProfile = profile
                }
                .disabled(model.isMutatingProfiles)

                Spacer()

                if let failure = selectedFailureEvent {
                    Label(failure.reason.rawValue.replacingOccurrences(of: "_", with: " "), systemImage: "exclamationmark.triangle.fill")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.orange)
                }
            }
        }
    }

    private func usageCard(_ profile: Profile) -> some View {
        SettingsSurfaceCard("Usage") {
            if let usage = model.usageSnapshot(for: profile.id) {
                VStack(alignment: .leading, spacing: 14) {
                    UsageMetricRow(title: "Session", window: usage.session, stale: usage.stale)
                    UsageMetricRow(title: "Weekly", window: usage.weekly, stale: usage.stale)
                    settingsRow("Source", value: usage.source.rawValue)
                    settingsRow("Updated", value: usage.lastRefreshedAt.formatted())
                    if let resetAt = usage.nextResetAt {
                        settingsRow("Next Reset", value: resetAt.formatted())
                    }
                    if let note = usage.userFacingNote {
                        Text(note)
                            .foregroundStyle(usage.stale ? .orange : .secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
            } else {
                Text("Usage data unavailable.")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    private func settingsCard(_ profile: Profile) -> some View {
        SettingsSurfaceCard("Paths") {
            settingsRow("Agent Home", value: profile.agentHome ?? "-")
            settingsRow("Config Path", value: profile.configPath ?? "-")
        }
    }

    private var selectedProfile: Profile? {
        model.selectedProfile
    }

    private var selectedFailureEvent: FailureEvent? {
        guard let profileId = selectedProfile?.id else {
            return nil
        }
        return model.events.first { $0.profileId == profileId }
    }

    private func settingsRow(_ title: String, value: String) -> some View {
        LabeledContent(title, value: value)
    }
}

private struct ProfileListRow: View {
    let profile: Profile
    let usage: UsageSnapshot?
    let isActive: Bool

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Circle()
                .fill(statusColor)
                .frame(width: 8, height: 8)
                .padding(.top, 8)

            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text(profile.nickname)
                        .font(.system(.body, design: .rounded).weight(.semibold))
                    if isActive {
                        Text("Current")
                            .font(.caption2.weight(.semibold))
                            .foregroundStyle(.tint)
                    }
                }

                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)

                if let usage {
                    UsageBadgeRow(usage: usage)
                }
            }

            Spacer(minLength: 10)

            Image(systemName: profile.enabled ? "checkmark.square.fill" : "square")
                .foregroundStyle(profile.enabled ? Color.accentColor : Color.secondary)
                .padding(.top, 2)
        }
        .padding(10)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(Color(nsColor: .controlColor))
        )
    }

    private var subtitle: String {
        if let usage {
            return "\(profile.agent.rawValue) • \(usage.source.rawValue) • \(usage.stale ? "stale" : "fresh")"
        }
        return "\(profile.agent.rawValue) • usage unavailable"
    }

    private var statusColor: Color {
        if !profile.enabled {
            return .gray
        }

        if isActive {
            return .green
        }

        if usage?.stale == true {
            return .orange
        }

        return .secondary
    }
}

private struct UsageMetricRow: View {
    let title: String
    let window: UsageWindow
    let stale: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                Spacer()
                Text(window.usedPercent.map { String(format: "%.0f%% used", $0) } ?? window.status.rawValue.capitalized)
                    .foregroundStyle(.secondary)
            }

            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 999, style: .continuous)
                        .fill(Color.secondary.opacity(0.14))
                    RoundedRectangle(cornerRadius: 999, style: .continuous)
                        .fill(tint.opacity(stale ? 0.45 : 0.8))
                        .frame(width: barWidth(for: geometry.size.width))
                }
            }
            .frame(height: 10)

            HStack {
                Text(window.resetAt.map { "Resets \($0.formatted(date: .abbreviated, time: .shortened))" } ?? "No reset window")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Text(window.exact ? "Exact" : "Estimate")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var tint: Color {
        switch window.status {
        case .healthy:
            return .teal
        case .warning:
            return .orange
        case .exhausted:
            return .red
        case .unknown:
            return .gray
        }
    }

    private func barWidth(for totalWidth: CGFloat) -> CGFloat {
        let percent = min(max(window.usedPercent ?? 0, 0), 100) / 100
        return max(8, totalWidth * percent)
    }
}

private enum ProfileEditorMode {
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
                    Toggle("Clear Agent Home", isOn: $draft.clearAgentHome)
                    TextField("Config Path", text: $draft.configPath)
                    Toggle("Clear Config Path", isOn: $draft.clearConfigPath)
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

private struct AddAccountSheet: View {
    let isBusy: Bool
    let onContinue: @MainActor (Int) async -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var priority = 100

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Add Account")
                .font(.title3.weight(.semibold))

            Form {
                Section("Profile") {
                    Stepper("Priority: \(priority)", value: $priority, in: 0...10_000)
                }

                Section("Flow") {
                    Text("Relay will start `codex login`, let Codex open the browser sign-in flow, then import the signed-in account automatically.")
                        .foregroundStyle(.secondary)
                    Text("The default nickname will be the account email. You can rename it later.")
                        .foregroundStyle(.secondary)
                }
            }
            .formStyle(.grouped)

            HStack {
                Spacer()

                Button("Cancel") {
                    dismiss()
                }

                Button("Continue") {
                    Task {
                        await onContinue(priority)
                        dismiss()
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(isBusy)
            }
        }
        .padding(24)
        .frame(width: 560)
    }
}
