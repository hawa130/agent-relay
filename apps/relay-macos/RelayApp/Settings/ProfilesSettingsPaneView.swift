import SwiftUI

public struct ProfilesSettingsPaneView: View {
    @ObservedObject var model: ProfilesPaneModel
    @State private var showingLoginSheet = false
    @State private var editingProfile: Profile?
    @State private var deletingProfile: Profile?

    public init(model: ProfilesPaneModel) {
        self.model = model
    }

    public var body: some View {
        HStack(spacing: 0) {
            sidebar
            Divider()
            detail
        }
        .background(NativePreferencesTheme.Colors.paneBackground)
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
        VStack(alignment: .leading, spacing: 14) {
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

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 6) {
                    ForEach(model.profiles) { profile in
                        Button {
                            model.selectProfile(profile.id)
                        } label: {
                            ProfileListRow(
                                profile: profile,
                                usage: model.usageSnapshot(for: profile.id),
                                isActive: model.activeProfileId == profile.id,
                                isSelected: model.selectedProfileId == profile.id
                            )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 16)
        .frame(
            minWidth: NativePreferencesTheme.Metrics.sidebarWidth,
            idealWidth: NativePreferencesTheme.Metrics.sidebarWidth,
            maxWidth: NativePreferencesTheme.Metrics.sidebarWidth + 20,
            maxHeight: .infinity,
            alignment: .topLeading
        )
        .background(NativePreferencesTheme.Colors.paneBackground)
    }

    private var detail: some View {
        NativePaneScrollView {
            VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionSpacing) {
                if let profile = selectedProfile {
                    profileHero(profile)
                    usageCard(profile)
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
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func profileHero(_ profile: Profile) -> some View {
        SettingsSurfaceCard(nil) {
            VStack(alignment: .leading, spacing: 14) {
                HStack(alignment: .top, spacing: 14) {
                    ProfileHeroAgentIcon(agent: profile.agent)

                    VStack(alignment: .leading, spacing: 0) {
                        Text(profile.agent.rawValue)
                            .font(NativePreferencesTheme.Typography.detail.weight(.semibold))
                            .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                            .textCase(.uppercase)

                        Text(profile.nickname)
                            .font(.system(size: 19, weight: .semibold, design: .rounded))

                        HStack(spacing: 6) {
                            ProfileStateBadge(
                                title: profile.enabled ? "Enabled" : "Disabled",
                                kind: profile.enabled ? .success : .neutral
                            )

                            if model.activeProfileId == profile.id {
                                ProfileStateBadge(title: "Active", kind: .info)
                            }
                        }
                        .padding(.top, 4)
                    }

                    Spacer(minLength: 20)

                    VStack(alignment: .trailing, spacing: 8) {
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
                        .labelsHidden()
                        .disabled(model.isMutatingProfiles)
                    }
                }

                Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 8) {
                    GridRow {
                        NativeDetailRow(title: "Priority", value: "\(profile.priority)")
                        NativeDetailRow(title: "Auth Mode", value: profile.authMode.displayName)
                    }
                }

                HStack(alignment: .center, spacing: 12) {
                    Button(model.activeProfileId == profile.id ? "Activated" : "Activate") {
                        Task {
                            await model.switchToProfile(profile.id)
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(model.activeProfileId == profile.id || !profile.enabled || model.isSwitching)

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
                            .font(NativePreferencesTheme.Typography.detail.weight(.semibold))
                            .foregroundStyle(.orange)
                    }
                }
            }
        }
    }

    private func usageCard(_ profile: Profile) -> some View {
        SettingsSurfaceCard(
            "Usage",
            headerAccessory: AnyView(
                Button {
                    Task {
                        await model.refreshUsage(profileId: profile.id)
                    }
                } label: {
                    Label("Refresh Usage", systemImage: "arrow.clockwise")
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.bordered)
                .disabled(model.isRefreshingUsage(profileId: profile.id))
                .help("Refresh Usage")
            )
        ) {
            if let usage = model.usageSnapshot(for: profile.id) {
                VStack(alignment: .leading, spacing: 12) {
                    UsageMetricRow(title: "Session", window: usage.session, stale: usage.stale)
                    UsageMetricRow(title: "Weekly", window: usage.weekly, stale: usage.stale)

                    VStack(alignment: .leading, spacing: 8) {
                        NativeDetailRow(title: "Source", value: usage.source.displayName)
                        NativeDetailRow(title: "Updated", value: usage.lastRefreshedAt.formatted())
                    }

                    if let note = usage.userFacingNote {
                        Text(note)
                            .font(NativePreferencesTheme.Typography.detail)
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

    private var selectedProfile: Profile? {
        model.selectedProfile
    }

    private var selectedFailureEvent: FailureEvent? {
        guard let profileId = selectedProfile?.id else {
            return nil
        }
        return model.recentFailureEvent(for: profileId)
    }
}

private struct ProfileHeroAgentIcon: View {
    let agent: AgentKind

    var body: some View {
        Group {
            if let descriptor = AgentSettingsCatalog.descriptor(for: agent) {
                ZStack {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(Color.secondary.opacity(0.12))
                        .frame(width: 40, height: 40)

                    AgentBrandIcon(descriptor: descriptor, size: 20)
                }
            } else {
                ZStack {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(Color.secondary.opacity(0.12))
                        .frame(width: 40, height: 40)

                    Image(systemName: "terminal")
                        .font(.system(size: 18, weight: .medium))
                        .foregroundStyle(.secondary)
                }
            }
        }
        .frame(width: 40, height: 40)
    }
}

private struct ProfileListRow: View {
    let profile: Profile
    let usage: UsageSnapshot?
    let isActive: Bool
    let isSelected: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            ProfileListAgentLabel(agent: profile.agent)

            Text(profile.nickname)
                .font(.system(size: 13, weight: .semibold, design: .rounded))

            Text(subtitle)
                .font(NativePreferencesTheme.Typography.detail)
                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)

            if let usage {
                UsageBadgeRow(usage: usage)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 9)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(rowBackground, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 9, style: .continuous)
                .strokeBorder(rowBorder, lineWidth: isSelected ? 1 : 0.5)
        )
        .overlay(alignment: .topTrailing) {
            if isActive {
                ProfileStateBadge(title: "Current", kind: .info)
                    .padding(.top, 9)
                    .padding(.trailing, 10)
            }
        }
        .contentShape(RoundedRectangle(cornerRadius: 9, style: .continuous))
    }

    private var subtitle: String {
        if let usage {
            var parts: [String] = []

            let relativeFormatter = RelativeDateTimeFormatter()
            relativeFormatter.unitsStyle = .short
            parts.append("Updated \(relativeFormatter.localizedString(for: usage.lastRefreshedAt, relativeTo: Date()))")

            if let resetAt = usage.nextResetAt {
                parts.append("Resets \(relativeFormatter.localizedString(for: resetAt, relativeTo: Date()))")
            }

            return parts.joined(separator: " • ")
        }
        return "Waiting for refresh"
    }

    private var rowBackground: Color {
        if isSelected {
            return Color.accentColor.opacity(0.12)
        }
        return NativePreferencesTheme.Colors.groupedBackground.opacity(0.55)
    }

    private var rowBorder: Color {
        if isSelected {
            return Color.accentColor.opacity(0.28)
        }
        return NativePreferencesTheme.Colors.sectionBorder.opacity(0.55)
    }
}

private struct ProfileListAgentLabel: View {
    let agent: AgentKind

    var body: some View {
        HStack(spacing: 5) {
            if let descriptor = AgentSettingsCatalog.descriptor(for: agent) {
                AgentBrandIcon(descriptor: descriptor, size: 12, tint: .secondary)
                    .frame(width: 12, height: 12)
            } else {
                Image(systemName: "terminal")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .frame(width: 12, height: 12)
            }

            Text(agent.rawValue.uppercased())
                .font(NativePreferencesTheme.Typography.detail.weight(.semibold))
                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
        }
    }
}

private struct UsageMetricRow: View {
    let title: String
    let window: UsageWindow
    let stale: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack {
                Text(title)
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Text(window.usedPercent.map { String(format: "%.0f%% used", $0) } ?? window.status.rawValue.capitalized)
                    .font(NativePreferencesTheme.Typography.detail)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
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
            .frame(height: NativePreferencesTheme.Metrics.usageBarHeight)

            HStack {
                Text(window.resetAt.map { "Resets \($0.formatted(date: .abbreviated, time: .shortened))" } ?? "No reset window")
                    .font(NativePreferencesTheme.Typography.detail)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                Spacer()
                Text(window.exact ? "Exact" : "Estimate")
                    .font(NativePreferencesTheme.Typography.detail)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
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

private struct ProfileStateBadge: View {
    let title: String
    let kind: NativePreferencesTheme.Badge.Kind

    var body: some View {
        Text(title)
            .font(.system(size: 10, weight: .semibold))
            .foregroundStyle(NativePreferencesTheme.Badge.text(kind))
            .padding(.horizontal, 7)
            .padding(.vertical, 3)
            .background(NativePreferencesTheme.Badge.fill(kind), in: Capsule())
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
                .padding(.horizontal, 18)
                .padding(.top, 18)

            Form {
                Section {
                    TextField("Nickname", text: $draft.nickname)
                    VStack(alignment: .leading, spacing: 0) {
                        NativeStepperRow(
                            title: "Priority",
                            valueText: "\(draft.priority)",
                            value: $draft.priority,
                            range: 0...10_000
                        )

                        Text("Lower numbers are preferred first during switching.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                } header: {
                    Text("Identity")
                }
            }
            .formStyle(.grouped)
            .frame(maxWidth: .infinity)

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
            .padding(.horizontal, 18)
            .padding(.bottom, 16)
        }
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
                .padding(.horizontal, 18)
                .padding(.top, 18)

            Form {
                Section {
                    VStack(alignment: .leading, spacing: 6) {
                        NativeStepperRow(
                            title: "Priority",
                            valueText: "\(priority)",
                            value: $priority,
                            range: 0...10_000
                        )

                        Text("Lower numbers are preferred first during switching.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                } header: {
                    Text("Profile")
                }

                Section("Flow") {
                    Text("Continue to open the Codex sign-in flow in your browser, then import the signed-in account automatically.")
                        .foregroundStyle(.secondary)
                    Text("The default nickname will be the account email. You can rename it later.")
                        .foregroundStyle(.secondary)
                }
            }
            .formStyle(.grouped)
            .frame(maxWidth: .infinity)

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
            .padding(.horizontal, 18)
            .padding(.bottom, 16)
        }
        .frame(width: 560)
    }
}
