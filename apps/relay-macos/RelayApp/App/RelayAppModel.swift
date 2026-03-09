import Foundation
import SwiftUI
import Defaults

@MainActor
public final class RelayAppModel: ObservableObject {
    @Published private(set) var status: StatusReport?
    @Published private(set) var usage: UsageSnapshot?
    @Published private(set) var usageSnapshots: [UsageSnapshot] = []
    @Published private(set) var doctor: DoctorReport?
    @Published private(set) var profiles: [Profile] = []
    @Published private(set) var events: [FailureEvent] = []
    @Published private(set) var logTail: LogTail?
    @Published private(set) var diagnosticsExport: DiagnosticsExport?
    @Published private(set) var lastRefresh: Date?
    @Published private(set) var isRefreshing = false
    @Published private(set) var isSwitching = false
    @Published private(set) var isMutatingProfiles = false
    @Published var selectedProfileId: String?
    @Published var lastErrorMessage: String?
    private let client = RelayCLIClient()
    private let notificationService = RelayNotificationService()
    private var pollTask: Task<Void, Never>?

    public init() {
        selectedProfileId = Defaults[.selectedProfileId]
        Task {
            await notificationService.requestAuthorizationIfNeeded()
            await refresh()
            startPolling()
        }
    }

    deinit {
        pollTask?.cancel()
    }

    public var menuBarTitle: String {
        activeProfile?.nickname ?? "Relay"
    }

    public var menuBarSymbol: String {
        switch status?.activeState.lastSwitchResult {
        case .success:
            return "bolt.circle.fill"
        case .failed:
            return "exclamationmark.triangle.fill"
        case .notRun, .none:
            return "bolt.circle"
        }
    }

    var activeProfileId: String? {
        status?.activeState.activeProfileId
    }

    var activeProfile: Profile? {
        guard let activeProfileId else {
            return nil
        }
        return profiles.first { $0.id == activeProfileId }
    }

    var selectedProfile: Profile? {
        guard let selectedProfileId else {
            return activeProfile ?? profiles.first
        }
        return profiles.first { $0.id == selectedProfileId } ?? activeProfile ?? profiles.first
    }

    var selectedUsage: UsageSnapshot? {
        guard let profileId = selectedProfile?.id else {
            return usage
        }
        return usageSnapshot(for: profileId)
    }

    var autoSwitchEnabled: Bool {
        status?.settings.autoSwitchEnabled ?? false
    }

    func usageSnapshot(for profileId: String) -> UsageSnapshot? {
        usageSnapshots.first { $0.profileId == profileId }
    }

    func selectProfile(_ profileId: String?) {
        selectedProfileId = profileId
        Defaults[.selectedProfileId] = profileId
    }

    func refresh(notifyOnFailure: Bool = false) async {
        guard !isRefreshing else {
            return
        }

        isRefreshing = true
        defer {
            isRefreshing = false
        }

        do {
            async let statusTask = client.fetchStatus()
            async let usageTask = client.fetchUsage()
            async let usageListTask = client.fetchUsageList()
            async let doctorTask = client.fetchDoctor()
            async let profilesTask = client.fetchProfiles()
            async let eventsTask = client.fetchEvents(limit: 10)
            async let logsTask = client.fetchLogs(lines: 25)

            status = try await statusTask
            usage = try await usageTask
            usageSnapshots = try await usageListTask
            doctor = try await doctorTask
            profiles = try await profilesTask
            events = try await eventsTask
            logTail = try await logsTask
            normalizeSelection()
            lastRefresh = Date()
            lastErrorMessage = nil
        } catch {
            lastErrorMessage = error.localizedDescription
            if notifyOnFailure {
                await notificationService.post(
                    title: "Relay refresh failed",
                    body: error.localizedDescription
                )
            }
        }
    }

    func switchToProfile(_ profileId: String) async {
        guard !isSwitching else {
            return
        }

        isSwitching = true
        defer {
            isSwitching = false
        }

        do {
            let report = try await client.switchToProfile(profileId)
            selectProfile(profileId)
            await refresh()
            await notificationService.post(
                title: "Relay switched profile",
                body: report.message
            )
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay switch failed",
                body: error.localizedDescription
            )
        }
    }

    func setAutoSwitch(enabled: Bool) async {
        do {
            _ = try await client.setAutoSwitch(enabled: enabled)
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay settings update failed",
                body: error.localizedDescription
            )
        }
    }

    func setUsageSourceMode(_ mode: UsageSourceMode) async {
        await updateUsageSettings(
            UsageSettingsDraft(
                sourceMode: mode,
                menuOpenRefreshStaleAfterSeconds: nil,
                backgroundRefreshEnabled: nil,
                backgroundRefreshIntervalSeconds: nil
            )
        )
    }

    func setMenuOpenRefreshStaleAfterSeconds(_ seconds: Int) async {
        await updateUsageSettings(
            UsageSettingsDraft(
                sourceMode: nil,
                menuOpenRefreshStaleAfterSeconds: seconds,
                backgroundRefreshEnabled: nil,
                backgroundRefreshIntervalSeconds: nil
            )
        )
    }

    func setBackgroundRefreshEnabled(_ enabled: Bool) async {
        await updateUsageSettings(
            UsageSettingsDraft(
                sourceMode: nil,
                menuOpenRefreshStaleAfterSeconds: nil,
                backgroundRefreshEnabled: enabled,
                backgroundRefreshIntervalSeconds: nil
            )
        )
    }

    func setBackgroundRefreshIntervalSeconds(_ seconds: Int) async {
        await updateUsageSettings(
            UsageSettingsDraft(
                sourceMode: nil,
                menuOpenRefreshStaleAfterSeconds: nil,
                backgroundRefreshEnabled: nil,
                backgroundRefreshIntervalSeconds: seconds
            )
        )
    }

    func refreshUsage(profileId: String) async {
        do {
            let snapshot = try await client.refreshUsage(profileId: profileId)
            mergeUsageSnapshot(snapshot)
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
        }
    }

    func refreshEnabledUsage() async {
        do {
            let snapshots = try await client.refreshEnabledUsage()
            for snapshot in snapshots {
                mergeUsageSnapshot(snapshot)
            }
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
        }
    }

    func refreshForMenuOpen() async {
        await refresh()
        guard shouldRefreshUsageOnMenuOpen else {
            return
        }
        await refreshEnabledUsage()
    }

    func setProfileEnabled(_ profileId: String, enabled: Bool) async {
        await performProfileMutation { [self] in
            _ = try await self.client.setProfileEnabled(profileId: profileId, enabled: enabled)
        }
    }

    func editProfile(profileId: String, draft: ProfileDraft) async {
        await performProfileMutation { [self] in
            _ = try await self.client.editProfile(profileId: profileId, draft: draft)
        }
    }

    func removeProfile(_ profileId: String) async {
        await performProfileMutation { [self] in
            _ = try await self.client.removeProfile(profileId: profileId)
        }
    }

    func importProfile(agent: AgentKind, nickname: String?, priority: Int) async {
        await performProfileMutation { [self] in
            let profile = try await self.client.importProfile(agent: agent, nickname: nickname, priority: priority)
            await MainActor.run {
                self.selectProfile(profile.id)
            }
        }
    }

    func loginProfile(agent: AgentKind, nickname: String?, priority: Int) async {
        guard !isMutatingProfiles else {
            return
        }

        isMutatingProfiles = true
        defer {
            isMutatingProfiles = false
        }

        do {
            let result = try await client.loginProfile(agent: agent, nickname: nickname, priority: priority)
            selectProfile(result.profile.id)
            await refresh()
            lastErrorMessage = nil
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay profile update failed",
                body: error.localizedDescription
            )
        }
    }

    func addAccount(agent: AgentKind, priority: Int) async {
        await loginProfile(agent: agent, nickname: nil, priority: priority)
    }

    func exportDiagnostics() async {
        do {
            diagnosticsExport = try await client.exportDiagnostics()
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay diagnostics export failed",
                body: error.localizedDescription
            )
        }
    }

    private func performProfileMutation(
        _ operation: @escaping @Sendable () async throws -> Void
    ) async {
        guard !isMutatingProfiles else {
            return
        }

        isMutatingProfiles = true
        defer {
            isMutatingProfiles = false
        }

        do {
            try await operation()
            await refresh()
            lastErrorMessage = nil
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay profile update failed",
                body: error.localizedDescription
            )
        }
    }

    private func startPolling() {
        guard pollTask == nil else {
            return
        }

        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else {
                    break
                }
                let interval = max(
                    self.status?.settings.usageBackgroundRefreshIntervalSeconds ?? 120,
                    15
                )
                try? await Task.sleep(for: .seconds(interval))
                guard self.status?.settings.usageBackgroundRefreshEnabled ?? true else {
                    continue
                }
                await self.refreshEnabledUsage()
            }
        }
    }

    private func updateUsageSettings(_ draft: UsageSettingsDraft) async {
        do {
            _ = try await client.setUsageSettings(draft)
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay usage settings update failed",
                body: error.localizedDescription
            )
        }
    }

    private func normalizeSelection() {
        if let selectedProfileId, profiles.contains(where: { $0.id == selectedProfileId }) {
            return
        }
        selectProfile(activeProfileId ?? profiles.first?.id)
    }

    private func mergeUsageSnapshot(_ snapshot: UsageSnapshot) {
        if let profileId = snapshot.profileId,
            let index = usageSnapshots.firstIndex(where: { $0.profileId == profileId })
        {
            usageSnapshots[index] = snapshot
        } else {
            usageSnapshots.append(snapshot)
        }

        if snapshot.profileId == activeProfileId {
            usage = snapshot
        }
    }

    private var shouldRefreshUsageOnMenuOpen: Bool {
        guard let settings = status?.settings else {
            return true
        }
        let threshold = TimeInterval(max(settings.menuOpenRefreshStaleAfterSeconds, 0))
        let now = Date()
        return profiles
            .filter(\.enabled)
            .contains { profile in
                guard let snapshot = usageSnapshot(for: profile.id) else {
                    return true
                }
                if snapshot.message == "usage not fetched yet" {
                    return true
                }
                return now.timeIntervalSince(snapshot.lastRefreshedAt) >= threshold
            }
    }
}
