import Combine
import Foundation

@MainActor
public final class SettingsSessionModel: ObservableObject {
    private let session: RelayAppModel
    private var cancellables: Set<AnyCancellable> = []

    public init(session: RelayAppModel) {
        self.session = session
        bindSession()
    }

    var status: StatusReport? { session.status }
    var doctor: DoctorReport? { session.doctor }
    var profilesCount: Int { session.status?.profileCount ?? session.profiles.count }
    var lastErrorMessage: String? { session.lastErrorMessage }
    var autoSwitchEnabled: Bool { session.autoSwitchEnabled }

    func setAutoSwitch(enabled: Bool) async {
        await session.setAutoSwitch(enabled: enabled)
    }

    func setUsageSourceMode(_ mode: UsageSourceMode) async {
        await session.setUsageSourceMode(mode)
    }

    func setMenuOpenRefreshStaleAfterSeconds(_ seconds: Int) async {
        await session.setMenuOpenRefreshStaleAfterSeconds(seconds)
    }

    func setBackgroundRefreshEnabled(_ enabled: Bool) async {
        await session.setBackgroundRefreshEnabled(enabled)
    }

    func setBackgroundRefreshIntervalSeconds(_ seconds: Int) async {
        await session.setBackgroundRefreshIntervalSeconds(seconds)
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

    func usageSnapshot(for profileId: String) -> UsageSnapshot? {
        session.usageSnapshot(for: profileId)
    }

    func recentFailureEvent(for profileId: String) -> FailureEvent? {
        session.events.first { $0.profileId == profileId }
    }

    func selectProfile(_ profileId: String?) {
        session.selectProfile(profileId)
    }

    func addAccount(agent: AgentKind, priority: Int) async {
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

    func refreshIfStale() async {
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
public final class ActivityPaneModel: ObservableObject {
    private let session: RelayAppModel
    private var cancellables: Set<AnyCancellable> = []

    public init(session: RelayAppModel) {
        self.session = session
        bindSession()
    }

    var events: [FailureEvent] { session.events }
    var logTail: LogTail? { session.logTail }
    var diagnosticsExport: DiagnosticsExport? { session.diagnosticsExport }
    var lastErrorMessage: String? { session.lastErrorMessage }

    func refresh() async {
        await session.refresh()
    }

    func refreshIfStale() async {
        await session.refreshIfStale(maxAge: 30)
    }

    func exportDiagnostics() async {
        await session.exportDiagnostics()
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
            parts.append(usage.source.rawValue)
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
