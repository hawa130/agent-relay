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
    @Published var selectedProfileID: String?
    @Published var lastErrorMessage: String?
    private let client = RelayCLIClient()
    private let notificationService = RelayNotificationService()
    private var pollTask: Task<Void, Never>?

    public init() {
        selectedProfileID = Defaults[.selectedProfileID]
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

    var activeProfileID: String? {
        status?.activeState.activeProfileID
    }

    var activeProfile: Profile? {
        guard let activeProfileID else {
            return nil
        }
        return profiles.first { $0.id == activeProfileID }
    }

    var selectedProfile: Profile? {
        guard let selectedProfileID else {
            return activeProfile ?? profiles.first
        }
        return profiles.first { $0.id == selectedProfileID } ?? activeProfile ?? profiles.first
    }

    var selectedUsage: UsageSnapshot? {
        guard let profileID = selectedProfile?.id else {
            return usage
        }
        return usageSnapshot(for: profileID)
    }

    var autoSwitchEnabled: Bool {
        status?.settings.autoSwitchEnabled ?? false
    }

    func usageSnapshot(for profileID: String) -> UsageSnapshot? {
        usageSnapshots.first { $0.profileID == profileID }
    }

    func selectProfile(_ profileID: String?) {
        selectedProfileID = profileID
        Defaults[.selectedProfileID] = profileID
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

    func switchToProfile(_ profileID: String) async {
        guard !isSwitching else {
            return
        }

        isSwitching = true
        defer {
            isSwitching = false
        }

        do {
            let report = try await client.switchToProfile(profileID)
            selectProfile(profileID)
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

    func refreshUsage(profileID: String) async {
        do {
            let snapshot = try await client.refreshUsage(profileID: profileID)
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

    func setProfileEnabled(_ profileID: String, enabled: Bool) async {
        await performProfileMutation { [self] in
            _ = try await self.client.setProfileEnabled(profileID: profileID, enabled: enabled)
        }
    }

    func editProfile(profileID: String, draft: ProfileDraft) async {
        await performProfileMutation { [self] in
            _ = try await self.client.editProfile(profileID: profileID, draft: draft)
        }
    }

    func removeProfile(_ profileID: String) async {
        await performProfileMutation { [self] in
            _ = try await self.client.removeProfile(profileID: profileID)
        }
    }

    func importCodexProfile(nickname: String?, priority: Int) async {
        await performProfileMutation { [self] in
            let profile = try await self.client.importCodexProfile(nickname: nickname, priority: priority)
            await MainActor.run {
                self.selectProfile(profile.id)
            }
        }
    }

    func loginCodexProfile(nickname: String?, priority: Int) async {
        guard !isMutatingProfiles else {
            return
        }

        isMutatingProfiles = true
        defer {
            isMutatingProfiles = false
        }

        do {
            let result = try await client.loginCodexProfile(nickname: nickname, priority: priority)
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

    func addCodexAccount(priority: Int) async {
        await loginCodexProfile(nickname: nil, priority: priority)
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
        if let selectedProfileID, profiles.contains(where: { $0.id == selectedProfileID }) {
            return
        }
        selectProfile(activeProfileID ?? profiles.first?.id)
    }

    private func mergeUsageSnapshot(_ snapshot: UsageSnapshot) {
        if let profileID = snapshot.profileID,
            let index = usageSnapshots.firstIndex(where: { $0.profileID == profileID })
        {
            usageSnapshots[index] = snapshot
        } else {
            usageSnapshots.append(snapshot)
        }

        if snapshot.profileID == activeProfileID {
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
