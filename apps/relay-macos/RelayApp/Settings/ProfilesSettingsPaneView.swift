import AppKit
import SwiftUI

enum UsageAlertSeverity: Equatable {
    case warning
}

struct UsageCardNote: Equatable {
    let text: String
    let severity: UsageAlertSeverity?
}

enum UsageCardNoteResolver {
    static func note(usage: UsageSnapshot?, usageRefreshError: String?) -> UsageCardNote? {
        if let note = usage?.userFacingNote {
            return UsageCardNote(
                text: note,
                severity: severity(for: usage)
            )
        }

        guard let error = usageRefreshError?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nilIfEmpty
        else {
            return nil
        }

        return UsageCardNote(text: error, severity: .warning)
    }

    static func severity(for usage: UsageSnapshot?) -> UsageAlertSeverity? {
        if usage?.remoteError != nil {
            return .warning
        }

        if usage?.stale == true || usage?.source != .webEnhanced {
            return .warning
        }

        return nil
    }

    static func color(for note: UsageCardNote) -> Color {
        guard let severity = note.severity else {
            return .secondary
        }

        switch severity {
        case .warning:
            return NativePreferencesTheme.Colors.semanticAccent(.warning)
        }
    }
}

enum UsageToolbarRefreshScope: Equatable {
    case enabled
    case all
}

enum UsageToolbarRefreshScopeResolver {
    static func resolve(modifierFlags: NSEvent.ModifierFlags) -> UsageToolbarRefreshScope {
        modifierFlags.contains(.option) ? .all : .enabled
    }
}

public struct ProfilesSettingsPaneView: View {
    @ObservedObject var model: ProfilesPaneModel
    @State private var deletingProfile: Profile?

    public init(model: ProfilesPaneModel) {
        self.model = model
    }

    public var body: some View {
        NavigationSplitView {
            sidebar
        } content: {
            contentColumn
        } detail: {
            detail
        }
        .navigationSplitViewStyle(.balanced)
        .sheet(
            isPresented: Binding(
                get: { model.isPresentingAddSheet },
                set: { isPresented in
                    if !isPresented {
                        model.dismissAddSheet()
                    }
                }
            )
        ) {
            AddProfileSheet(
                agents: model.agents,
                profileCountForAgent: { agent in
                    model.profileCount(for: agent)
                },
                isBusy: model.isMutatingProfiles,
                onImportProfile: { agent in
                    await model.importProfile(agent: agent, nickname: nil, priority: 100)
                },
                onStartLogin: { agent in
                    await model.addAccount(agent: agent)
                },
                onCancelLogin: {
                    await model.cancelAddAccount()
                }
            )
        }
        .sheet(
            item: Binding(
                get: { model.editingProfile },
                set: { profile in
                    if profile == nil {
                        model.dismissEditSheet()
                    }
                }
            )
        ) { profile in
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
        List(ProfilesSidebarFilter.allCases, selection: selectedFilterBinding) { item in
            ProfilesSidebarItemLabel(item: item)
                .badge(profileCount(for: item))
                .tag(item)
        }
        .listStyle(.sidebar)
        .navigationSplitViewColumnWidth(140)
    }

    private var contentColumn: some View {
        List(selection: selectedProfileBinding) {
            ForEach(model.filteredProfiles) { profile in
                ProfileListRow(
                    profile: profile,
                    usage: model.usageSnapshot(for: profile.id),
                    isActive: model.activeProfileId == profile.id,
                    isFetchingUsage: model.isFetchingUsage(profileId: profile.id),
                    usageRefreshError: model.usageRefreshError(profileId: profile.id)
                )
                .tag(Optional(profile.id))
            }

            if model.filteredProfiles.isEmpty {
                ContentUnavailableView(
                    "No Profiles",
                    systemImage: "person.crop.square",
                    description: Text(emptyStateDescription)
                )
                .disabled(true)
            }
        }
        .listStyle(.inset)
        .navigationSplitViewColumnWidth(min: 260, ideal: 340, max: 400)
        .toolbar {
            ToolbarItemGroup(placement: .navigation) {
                VStack(alignment: .leading, spacing: 1) {
                    Text(model.selectedFilter.title)
                        .font(.headline)

                    Text(profileCountSummary)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }

            ToolbarItemGroup(placement: .secondaryAction) {
                Spacer(minLength: 0)
            }

            ToolbarItemGroup(placement: .confirmationAction) {
                if model.isFetchingEnabledUsage {
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 28, height: 28)
                        .help("Refreshing usage")
                } else {
                    Button {
                        let scope = UsageToolbarRefreshScopeResolver.resolve(
                            modifierFlags: NSApp.currentEvent?.modifierFlags ?? []
                        )
                        Task {
                            switch scope {
                            case .enabled:
                                await model.refreshEnabledUsage()
                            case .all:
                                await model.refreshAllUsage()
                            }
                        }
                    } label: {
                        Label("Refresh Usage", systemImage: "arrow.clockwise")
                    }
                    .labelStyle(.iconOnly)
                    .buttonStyle(.bordered)
                    .help("Refresh Usage For Enabled Profiles. Option-click to refresh all profiles.")
                }

                Button {
                    model.presentAddSheet()
                } label: {
                    Label("Add Profile", systemImage: "plus")
                }
                .accessibilityLabel("Add Profile")
                .help("Add Profile")
            }
        }
    }

    private var selectedProfileBinding: Binding<String?> {
        Binding(
            get: {
                guard let selectedProfileId = model.selectedProfileId,
                      model.filteredProfiles.contains(where: { $0.id == selectedProfileId })
                else {
                    return nil
                }
                return selectedProfileId
            },
            set: { profileId in
                model.selectProfile(profileId)
            }
        )
    }

    private var selectedFilterBinding: Binding<ProfilesSidebarFilter?> {
        Binding(
            get: { model.selectedFilter },
            set: { filter in
                if let filter {
                    model.selectFilter(filter)
                }
            }
        )
    }

    private var detail: some View {
        Group {
            if let profile = selectedProfile {
                NativePaneScrollView {
                    VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionSpacing) {
                        profileHero(profile)
                        usageCard(profile)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            } else {
                ContentUnavailableView(
                    "No Profile Selected",
                    systemImage: "person.crop.square",
                    description: Text("Choose a profile on the left to inspect its details and actions.")
                )
                .frame(maxWidth: .infinity, minHeight: 420)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                .background(NativePreferencesTheme.Colors.paneBackground)
            }
        }
        .navigationSplitViewColumnWidth(min: 380, ideal: 460)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Toggle(isOn: selectedProfileActiveBinding) {
                    Label(
                        selectedProfileIsActive ? "Profile is active" : "Activate Profile",
                        systemImage: selectedProfileIsActive ? "checkmark.circle.fill" : "checkmark.circle"
                    )
                }
                .labelStyle(.iconOnly)
                .toggleStyle(.button)
                .disabled(isActiveToggleDisabled)
                .help(selectedProfileIsActive ? "Profile is active" : "Activate Profile")
                .accessibilityLabel(selectedProfileIsActive ? "Profile is active" : "Activate Profile")
            }

            ToolbarItemGroup(placement: .confirmationAction) {
                Button {
                    model.presentEditForSelectedProfile()
                } label: {
                    Label("Edit Profile", systemImage: "square.and.pencil")
                }
                .accessibilityLabel("Edit Profile")
                .help("Edit Profile")
                .disabled(selectedProfile == nil || model.isMutatingProfiles)

                Button(role: .destructive) {
                    deletingProfile = selectedProfile
                } label: {
                    Label("Delete Profile", systemImage: "trash")
                }
                .accessibilityLabel("Delete Profile")
                .help("Delete Profile")
                .disabled(selectedProfile == nil || model.isMutatingProfiles)
            }
        }
    }

    private var profileCountSummary: String {
        let count = model.selectedFilterProfileCount
        return count == 1 ? "1 profile" : "\(count) profiles"
    }

    private var emptyStateDescription: String {
        model.selectedFilterEmptyStateDescription
    }

    private func profileHero(_ profile: Profile) -> some View {
        SettingsSurfaceCard(nil) {
            VStack(alignment: .leading, spacing: 12) {
                HStack(alignment: .top, spacing: 12) {
                    ProfileHeroAgentIcon(agent: profile.agent)

                    VStack(alignment: .leading, spacing: 0) {
                        ProfileAgentLabel(
                            title: profile.agent.rawValue,
                            showsActiveBadge: model.activeProfileId == profile.id
                        )

                        Text(profile.nickname)
                            .font(.system(size: 19, weight: .semibold, design: .rounded))

                        HStack(spacing: 6) {
                            ProfileStatusBadge(
                                title: profile.enabled ? "Enabled" : "Disabled",
                                dotColor: profile.enabled ? NativePreferencesTheme.Colors.semanticAccent(.success) : NativePreferencesTheme.Colors.disabledIndicator
                            )
                            ProfileInfoBadge(title: "Priority", value: "\(profile.priority)")
                        }
                        .padding(.top, 4)
                    }

                    Spacer(minLength: 0)

                    VStack(alignment: .trailing, spacing: 8) {
                        Toggle("Enabled", isOn: selectedProfileEnabledBinding)
                            .toggleStyle(.switch)
                            .labelsHidden()
                            .disabled(model.isMutatingProfiles)
                    }

                }
                if let failure = selectedFailureEvent {
                    Label(failure.reason.rawValue.replacingOccurrences(of: "_", with: " "), systemImage: "exclamationmark.triangle.fill")
                        .font(NativePreferencesTheme.Typography.detail.weight(.semibold))
                        .foregroundStyle(NativePreferencesTheme.Colors.semanticAccent(.warning))
                }
            }
        }
    }

    private func usageCard(_ profile: Profile) -> some View {
        let isFetchingUsage = model.isFetchingUsage(profileId: profile.id)
        let usageRefreshError = model.usageRefreshError(profileId: profile.id)
        let note = UsageCardNoteResolver.note(
            usage: model.usageSnapshot(for: profile.id),
            usageRefreshError: usageRefreshError
        )
        return SettingsSurfaceCard(
            "Usage",
            headerAccessory: AnyView(
                Button {
                    Task {
                        await model.refreshUsage(profileId: profile.id)
                    }
                } label: {
                    Group {
                        if isFetchingUsage {
                            ProgressView()
                                .controlSize(.small)
                        } else {
                            Image(systemName: "arrow.clockwise")
                        }
                    }
                    .frame(width: 14, height: 14)
                }
                .buttonStyle(.bordered)
                .disabled(isFetchingUsage)
                .help("Refresh Usage")
            )
        ) {
            if let usage = model.usageSnapshot(for: profile.id) {
                VStack(alignment: .leading, spacing: 10) {
                    UsageMetricRow(title: "Session", window: usage.session, stale: usage.stale)
                    UsageMetricRow(title: "Weekly", window: usage.weekly, stale: usage.stale)

                    if let note {
                        Text(note.text)
                            .font(NativePreferencesTheme.Typography.detail)
                            .foregroundStyle(UsageCardNoteResolver.color(for: note))
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }

                    HStack {
                        Spacer(minLength: 0)

                        VStack(alignment: .trailing, spacing: 2) {
                            Text("Source: \(usage.source.displayName)")
                            Text("Updated: \(usage.lastRefreshedAt.formatted(date: .abbreviated, time: .standard))")
                        }
                        .font(.system(size: 10))
                        .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                    }
                }
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    if isFetchingUsage {
                        HStack(spacing: 8) {
                            ProgressView()
                                .controlSize(.small)
                            Text("Refreshing usage…")
                                .foregroundStyle(.secondary)
                        }
                    } else if let note {
                        Text(note.text)
                            .font(NativePreferencesTheme.Typography.detail)
                            .foregroundStyle(UsageCardNoteResolver.color(for: note))
                    } else {
                        Text("Usage data unavailable.")
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    private var selectedProfile: Profile? {
        model.selectedProfile
    }

    private var selectedProfileIsActive: Bool {
        guard let profile = selectedProfile else {
            return false
        }

        return model.activeProfileId == profile.id
    }

    private var isActiveToggleDisabled: Bool {
        guard let profile = selectedProfile else {
            return true
        }

        return model.isMutatingProfiles || model.isSwitching || !profile.enabled || model.activeProfileId == profile.id
    }

    private var selectedProfileActiveBinding: Binding<Bool> {
        Binding(
            get: { selectedProfileIsActive },
            set: { isActive in
                guard isActive, let profile = selectedProfile else {
                    return
                }

                Task {
                    await model.switchToProfile(profile.id)
                }
            }
        )
    }

    private var selectedProfileEnabledBinding: Binding<Bool> {
        Binding(
            get: { selectedProfile?.enabled ?? false },
            set: { enabled in
                guard let profile = selectedProfile else {
                    return
                }

                Task {
                    await model.setProfileEnabled(profile.id, enabled: enabled)
                }
            }
        )
    }

    private var selectedFailureEvent: FailureEvent? {
        guard let profileId = selectedProfile?.id else {
            return nil
        }
        return model.recentFailureEvent(for: profileId)
    }

    private func profileCount(for item: ProfilesSidebarFilter) -> Int {
        model.profileCount(for: item)
    }
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}

private struct ProfileHeroAgentIcon: View {
    let agent: AgentKind

    var body: some View {
        Group {
            if AgentSettingsCatalog.descriptor(for: agent) != nil {
                ZStack {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(NativePreferencesTheme.Colors.subtleFill)
                        .frame(width: 40, height: 40)

                    AgentIcon(agent: agent, size: 20)
                }
            } else {
                ZStack {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(NativePreferencesTheme.Colors.subtleFill)
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
    let isFetchingUsage: Bool
    let usageRefreshError: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ProfileListAgentLabel(agent: profile.agent)

            HStack(alignment: .top, spacing: 8) {
                if let usage {
                    MultiRingProgressView(
                        items: usage.ringProgressItems,
                        size: .mini
                    ) { _ in
                        EmptyView()
                    }
                    .frame(width: 26, height: 26)
                    .padding(.vertical, 6)
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text(profile.nickname)
                        .font(.system(size: 13, weight: .semibold, design: .rounded))
                        .foregroundStyle(profile.enabled ? .primary : .secondary)

                    if let usage {
                        ProfileListUsageLine(
                            title: "Session",
                            value: usage.session.menuBarDisplayValue,
                            resetDate: usage.session.resetAt
                        )

                        ProfileListUsageLine(
                            title: "Weekly",
                            value: usage.weekly.menuBarDisplayValue,
                            resetDate: usage.weekly.resetAt
                        )

                        HStack {
                            Spacer(minLength: 0)

                            HStack(spacing: 4) {
                                if let indicator = statusIndicator {
                                    ProfileListRowStatusIndicator(indicator: indicator)
                                }

                                updatedText(for: usage)
                                    .font(.system(size: 10))
                                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                            }
                        }
                    } else {
                        HStack(spacing: 6) {
                            Text("Waiting for refresh")
                                .font(NativePreferencesTheme.Typography.detail)
                                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)

                            Spacer(minLength: 0)

                            if let indicator = statusIndicator {
                                ProfileListRowStatusIndicator(indicator: indicator)
                            }
                        }
                    }
                }
            }
        }
        .padding(4)
        .frame(maxWidth: .infinity, alignment: .leading)
        .overlay(alignment: .topTrailing) {
            if isActive {
                ProfileStateBadge(title: "Current", kind: .info)
                    .padding(.top, 4)
                    .padding(.trailing, 4)
            }
        }
    }

    var statusIndicator: ProfileListRowStatusIndicator.Kind? {
        ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: isFetchingUsage,
            usage: usage,
            usageRefreshError: usageRefreshError,
            isStale: usage?.stale == true
        )
    }

    private func updatedText(for usage: UsageSnapshot) -> some View {
        AdaptiveRelativeDateText(
            prefix: "Updated ",
            date: usage.lastRefreshedAt,
            style: .named
        )
    }

}

struct ProfileListRowStatusIndicator: View {
    enum Kind: Equatable {
        case loading
        case warning(message: String)
        case stale

        init?(isFetchingUsage: Bool, usage: UsageSnapshot?, usageRefreshError: String?, isStale: Bool) {
            if isFetchingUsage {
                self = .loading
            } else if let note = usage?.userFacingNote,
                      let severity = UsageCardNoteResolver.severity(for: usage) {
                switch severity {
                case .warning:
                    self = .warning(message: note)
                }
            } else if let usageRefreshError, !usageRefreshError.isEmpty {
                self = .warning(message: usageRefreshError)
            } else if isStale {
                self = .stale
            } else {
                return nil
            }
        }
    }

    let indicator: Kind

    var body: some View {
        Group {
            switch indicator {
            case .loading:
                ProgressView()
                    .controlSize(.mini)
            case let .warning(message):
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(NativePreferencesTheme.Colors.semanticAccent(.warning))
                    .help(message)
            case .stale:
                Image(systemName: "exclamationmark.triangle")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(.secondary)
                    .help("Usage data may be stale")
            }
        }
        .frame(width: 12, height: 12)
    }
}

private struct ProfileListUsageLine: View {
    let title: String
    let value: String
    let resetDate: Date?

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text("\(title) \(value)")
                .font(.system(size: 10))
                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)

            Spacer(minLength: 2)

            if let resetDate {
                ResetRelativeDateText(date: resetDate)
                .font(.system(size: 10))
                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct ProfileListAgentLabel: View {
    let agent: AgentKind

    var body: some View {
        HStack(spacing: 5) {
            if AgentSettingsCatalog.descriptor(for: agent) != nil {
                AgentIcon(agent: agent, size: 12, tint: .secondary)
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
        VStack(alignment: .leading, spacing: 4) {
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
                        .fill(NativePreferencesTheme.Colors.progressTrack)
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
        NativePreferencesTheme.Colors.usageTint(window.status)
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
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(NativePreferencesTheme.Badge.fill(kind), in: Capsule())
            .fixedSize()
    }
}

private struct ProfileAgentLabel: View {
    let title: String
    let showsActiveBadge: Bool

    var body: some View {
        Text(title)
            .font(NativePreferencesTheme.Typography.detail.weight(.semibold))
            .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
            .textCase(.uppercase)
            .overlay(alignment: .topTrailing) {
                if showsActiveBadge {
                    ProfileStateBadge(title: "Active", kind: .info)
                        .offset(x: 52, y: -1)
                        .allowsHitTesting(false)
                }
            }
    }
}

private struct ProfileStatusBadge: View {
    let title: String
    let dotColor: Color

    var body: some View {
        HStack(spacing: 5) {
            Circle()
                .fill(dotColor)
                .frame(width: 5, height: 5)

            Text(title)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(NativePreferencesTheme.Badge.fill(.neutral), in: Capsule())
        .fixedSize()
    }
}

private struct ProfileInfoBadge: View {
    let title: String
    let value: String

    var body: some View {
        HStack(spacing: 0) {
            Text(title)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))
                .padding(.leading, 6)
                .padding(.trailing, 5)
                .padding(.vertical, 2)

            Rectangle()
                .fill(NativePreferencesTheme.Badge.text(.neutral).opacity(0.22))
                .frame(width: 1)
                .padding(.vertical, 2)

            Text(value)
                .font(.system(size: 10, weight: .regular))
                .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))
                .padding(.leading, 5)
                .padding(.trailing, 6)
                .padding(.vertical, 2)
        }
        .background(NativePreferencesTheme.Badge.fill(.neutral), in: Capsule())
        .fixedSize()
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
        VStack(alignment: .leading, spacing: 14) {
            Text(title)
                .font(.title3.weight(.semibold))
                .padding(.horizontal, 16)
                .padding(.top, 16)

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
            .padding(.horizontal, 16)
            .padding(.bottom, 14)
        }
        .frame(width: 500)
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

private struct AddProfileSheet: View {
    let agents: [AgentSettingsDescriptor]
    let profileCountForAgent: (AgentKind) -> Int
    let isBusy: Bool
    let onImportProfile: @MainActor (AgentKind) async -> Void
    let onStartLogin: @MainActor (AgentKind) async -> AddAccountResult
    let onCancelLogin: @MainActor () async -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var mode: Mode = .catalog
    @State private var loginAgent: AgentKind?
    @State private var flowState: FlowState = .requesting
    @State private var loginTask: Task<Void, Never>?

    var body: some View {
        Group {
            switch mode {
            case .catalog:
                catalogView
            case .login:
                loginView
            }
        }
        .frame(width: 440)
        .interactiveDismissDisabled(mode == .login && flowState.isRequesting)
        .onDisappear {
            loginTask?.cancel()
            loginTask = nil
        }
    }

    private var catalogView: some View {
        VStack(alignment: .leading, spacing: 0) {
            Form {
                ForEach(agents) { descriptor in
                    agentRow(descriptor: descriptor)
                }
            }
            .formStyle(.grouped)
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            HStack {
                Spacer()

                Button("Cancel") {
                    dismiss()
                }
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 14)
        }
    }

    private func agentRow(descriptor: AgentSettingsDescriptor) -> some View {
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

                Text(profileCountText(for: descriptor))
                    .font(.system(size: 11, weight: .regular))
                    .foregroundStyle(.secondary)

                HStack(spacing: 6) {
                    Spacer(minLength: 0)

                    Button("Import Current Config") {
                        Task { @MainActor in
                            await onImportProfile(descriptor.agent)
                            dismiss()
                        }
                    }
                    .disabled(isBusy)

                    Button("Add Account...") {
                        startLogin(for: descriptor.agent)
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(isBusy)
                }
                .padding(.top, 4)
            }
        }
    }

    private func profileCountText(for descriptor: AgentSettingsDescriptor) -> String {
        let count = profileCountForAgent(descriptor.agent)
        let profileText = count == 1 ? "1 profile" : "\(count) profiles"
        return "\(descriptor.vendorTitle) • \(profileText)"
    }

    private var loginView: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(loginTitle)
                .font(.title3.weight(.semibold))
                .padding(.horizontal, 16)
                .padding(.top, 16)

            statusCard
                .padding(.horizontal, 16)

            Text(flowState.bodyText)
                .font(.body)
                .foregroundStyle(.secondary)
                .padding(.horizontal, 16)
                .frame(maxWidth: .infinity, alignment: .leading)

            HStack {
                Spacer()

                Button(flowState.secondaryActionTitle) {
                    if flowState.isRequesting {
                        Task { @MainActor in
                            await onCancelLogin()
                        }
                    } else {
                        showCatalog()
                    }
                }

                if let primaryActionTitle = flowState.primaryActionTitle, let agent = loginAgent {
                    Button(primaryActionTitle) {
                        startLogin(for: agent)
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(isBusy || flowState.isRequesting)
                }
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 14)
        }
    }

    private var statusCard: some View {
        HStack(alignment: .top, spacing: 10) {
            Group {
                if flowState.isRequesting {
                    ProgressView()
                        .controlSize(.small)
                        .tint(flowState.accentColor)
                        .frame(width: 22, height: 22)
                } else {
                    Image(systemName: flowState.symbolName)
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundStyle(flowState.accentColor)
                        .frame(width: 22, height: 22)
                }
            }

            VStack(alignment: .leading, spacing: 4) {
                Text(flowState.statusTitle)
                    .font(.headline)

                Text(flowState.statusSubtitle(agentName: loginAgent?.displayName ?? "Agent"))
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                if let detail = flowState.statusDetail {
                    Text(detail)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            }

            Spacer(minLength: 0)
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(flowState.backgroundColor)
        )
    }

    private var loginTitle: String {
        loginAgent?.displayName ?? "Add Account"
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
            switch result {
            case .success, .cancelled:
                dismiss()
            case let .notSignedIn(detail):
                flowState = .notSignedIn(detail: detail)
            case let .failed(detail):
                flowState = .failed(detail: detail)
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

    private enum Mode {
        case catalog
        case login
    }

    private enum FlowState: Equatable {
        case requesting
        case notSignedIn(detail: String)
        case failed(detail: String)

        var isRequesting: Bool {
            if case .requesting = self {
                return true
            }
            return false
        }

        var primaryActionTitle: String? {
            switch self {
            case .requesting:
                return nil
            case .notSignedIn, .failed:
                return "Try Again"
            }
        }

        var secondaryActionTitle: String {
            switch self {
            case .requesting:
                return "Cancel"
            case .notSignedIn, .failed:
                return "Back"
            }
        }

        var bodyText: String {
            switch self {
            case .requesting:
                return "Complete the browser login, or cancel."
            case .notSignedIn:
                return "Login did not complete."
            case let .failed(detail):
                return detail
            }
        }

        var statusTitle: String {
            switch self {
            case .requesting:
                return "Add Account..."
            case .notSignedIn, .failed:
                return "Add Account"
            }
        }

        func statusSubtitle(agentName _: String) -> String {
            switch self {
            case .requesting:
                return "Requesting login..."
            case .notSignedIn:
                return "Not signed in"
            case .failed:
                return "Login failed"
            }
        }

        var statusDetail: String? {
            switch self {
            case .requesting:
                return nil
            case let .notSignedIn(detail), let .failed(detail):
                return detail
            }
        }

        var symbolName: String {
            switch self {
            case .requesting:
                return "key.fill"
            case .notSignedIn:
                return "person.crop.circle.badge.xmark"
            case .failed:
                return "exclamationmark.triangle.fill"
            }
        }

        var accentColor: Color {
            switch self {
            case .requesting:
                return .secondary
            case .notSignedIn:
                return NativePreferencesTheme.Colors.semanticAccent(.warning)
            case .failed:
                return NativePreferencesTheme.Colors.semanticAccent(.danger)
            }
        }

        var backgroundColor: Color {
            switch self {
            case .requesting:
                return NativePreferencesTheme.Colors.pendingFill
            case .notSignedIn:
                return NativePreferencesTheme.Colors.semanticFill(.warning)
            case .failed:
                return NativePreferencesTheme.Colors.semanticFill(.danger)
            }
        }
    }
}

private struct ProfilesSidebarItemLabel: View {
    let item: ProfilesSidebarFilter

    var body: some View {
        switch item {
        case .all:
            Label(item.title, systemImage: "square.grid.2x2")
        case .codex:
            Label {
                Text(item.title)
            } icon: {
                AgentIcon(agent: .codex, size: 14)
                    .frame(width: 16, height: 16)
            }
        }
    }
}
