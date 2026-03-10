import Combine
import Foundation

enum AddAccountResult: Sendable, Equatable {
    case success
    case cancelled
    case notSignedIn(detail: String)
    case failed(detail: String)
}

@MainActor
public final class SettingsPaneModel: ObservableObject {
    private let session: RelayAppModel
    private var cancellables: Set<AnyCancellable> = []
    @Published private(set) var selectedItem: SettingsSidebarSelection

    public init(session: RelayAppModel) {
        self.session = session
        self.selectedItem = .general
        bindSession()
    }

    var autoSwitchEnabled: Bool { session.autoSwitchEnabled }
    var profilesCount: Int { session.status?.profileCount ?? session.profiles.count }
    var agents: [AgentSettingsDescriptor] { AgentSettingsCatalog.supportedAgents }
    var codexSettings: CodexSettings? { session.codexSettings }
    var lastErrorMessage: String? { session.lastErrorMessage }

    func selectItem(_ item: SettingsSidebarSelection) {
        guard selectedItem != item else {
            return
        }

        selectedItem = item
    }

    func setAutoSwitch(enabled: Bool) async {
        await session.setAutoSwitch(enabled: enabled)
    }

    func setUsageSourceMode(_ mode: UsageSourceMode) async {
        await session.setCodexUsageSourceMode(mode)
    }

    public func refreshIfStale() async {
        await session.refreshIfStale(maxAge: 30)
    }

    private func bindSession() {
        session.objectWillChange
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.objectWillChange.send()
            }
            .store(in: &cancellables)
    }
}

@MainActor
public final class ProfilesPaneModel: ObservableObject {
    private let session: RelayAppModel
    private var cancellables: Set<AnyCancellable> = []

    public init(session: RelayAppModel) {
        self.session = session
        bindSession()
    }

    var profiles: [Profile] { session.profiles }
    var selectedProfileId: String? { session.selectedProfileId }
    var activeProfileId: String? { session.activeProfileId }
    var selectedProfile: Profile? { session.selectedProfile }
    var lastErrorMessage: String? { session.lastErrorMessage }
    var isSwitching: Bool { session.isSwitching }
    var isMutatingProfiles: Bool { session.isMutatingProfiles }
    func isRefreshingUsage(profileId: String) -> Bool { session.isRefreshingUsage(profileId: profileId) }

    func usageSnapshot(for profileId: String) -> UsageSnapshot? {
        session.usageSnapshot(for: profileId)
    }

    func recentFailureEvent(for profileId: String) -> FailureEvent? {
        session.events.first { $0.profileId == profileId }
    }

    func selectProfile(_ profileId: String?) {
        session.selectProfile(profileId)
    }

    func addAccount(agent: AgentKind, priority: Int = 100) async -> AddAccountResult {
        await session.addAccount(agent: agent, priority: priority)
    }

    func importProfile(agent: AgentKind, nickname: String?, priority: Int) async {
        await session.importProfile(agent: agent, nickname: nickname, priority: priority)
    }

    func editProfile(profileId: String, draft: ProfileDraft) async {
        await session.editProfile(profileId: profileId, draft: draft)
    }

    func removeProfile(_ profileId: String) async {
        await session.removeProfile(profileId)
    }

    func setProfileEnabled(_ profileId: String, enabled: Bool) async {
        await session.setProfileEnabled(profileId, enabled: enabled)
    }

    func switchToProfile(_ profileId: String) async {
        await session.switchToProfile(profileId)
    }

    func refreshUsage(profileId: String) async {
        await session.refreshUsage(profileId: profileId)
    }

    public func refreshIfStale() async {
        await session.refreshIfStale(maxAge: 30)
    }

    private func bindSession() {
        session.objectWillChange
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.objectWillChange.send()
            }
            .store(in: &cancellables)
    }
}

@MainActor
struct MenuBarPresenter {
    let session: RelayAppModel

    var title: String {
        session.menuBarTitle
    }

    var symbolName: String {
        session.menuBarSymbol
    }

    var currentCardSubtitle: String {
        if session.isRefreshing {
            return "Refreshing…"
        }

        if let lastRefresh = session.lastRefresh {
            let formatter = RelativeDateTimeFormatter()
            formatter.unitsStyle = .short
            return "Updated \(formatter.localizedString(for: lastRefresh, relativeTo: Date()))"
        }

        return "Waiting for refresh"
    }

    func currentCardNotes(usage: UsageSnapshot?) -> [String] {
        usage?.userFacingNote.map { [$0] } ?? []
    }

    func profileStatusText(profile: Profile, usage: UsageSnapshot?, isActive: Bool) -> String {
        if isActive {
            return "Active"
        }

        if !profile.enabled {
            return "Disabled"
        }

        if usage?.stale == true {
            return "Stale"
        }

        return "Ready"
    }

    func profileFooterText(profile: Profile, usage: UsageSnapshot?) -> String? {
        var parts = [profile.agent.rawValue]

        if let usage {
            parts.append(usage.source.displayName)
            if usage.stale {
                parts.append("Stale")
            }
        } else {
            parts.append("No usage yet")
        }

        parts.append("P\(profile.priority)")
        return parts.joined(separator: " • ")
    }

    func profileSymbolName(profile: Profile, usage: UsageSnapshot?, isActive: Bool) -> String {
        if isActive {
            return "checkmark.circle.fill"
        }

        if !profile.enabled {
            return "slash.circle"
        }

        switch (usage?.weekly.status ?? usage?.session.status) ?? .unknown {
        case .warning:
            return "exclamationmark.circle"
        case .exhausted:
            return "xmark.circle"
        default:
            return "circle"
        }
    }
}
