import Foundation
import SwiftUI
import Defaults

@MainActor
public final class RelayAppModel: ObservableObject {
    @Published private(set) var status: StatusReport?
    @Published private(set) var codexSettings: CodexSettings?
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
    @Published private(set) var isAutoSwitching = false
    @Published private(set) var isMutatingProfiles = false
    @Published private(set) var isRefreshingEnabledUsage = false
    @Published private(set) var refreshingUsageProfileIds: Set<String> = []
    @Published var selectedProfileId: String?
    @Published var lastErrorMessage: String?
    private let client = RelayCLIClient()
    private let notificationService = RelayNotificationService()
    private var lastAutoSwitchConflictSignature: String?
    private var hasStarted = false

    public init() {
        selectedProfileId = Defaults[.selectedProfileId]
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

    func isRefreshingUsage(profileId: String) -> Bool {
        refreshingUsageProfileIds.contains(profileId)
    }

    func selectProfile(_ profileId: String?) {
        selectedProfileId = profileId
        Defaults[.selectedProfileId] = profileId
    }

    public func start() {
        guard !hasStarted else {
            return
        }

        hasStarted = true
        Task {
            await refresh()
        }
    }

    func refreshIfStale(maxAge seconds: TimeInterval) async {
        guard let lastRefresh else {
            await refresh()
            return
        }

        if Date().timeIntervalSince(lastRefresh) >= seconds {
            await refresh()
        }
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
            async let codexSettingsTask = client.fetchCodexSettings()
            async let profileListTask = client.fetchProfileList()
            async let doctorTask = client.fetchDoctor()
            async let eventsTask = client.fetchEvents(limit: 10)
            async let logsTask = client.fetchLogs(lines: 25)

            status = try await statusTask
            codexSettings = try await codexSettingsTask
            let profileItems = try await profileListTask
            profiles = profileItems.map(\.profile)
            usageSnapshots = profileItems.compactMap(\.usageSummary)
            doctor = try await doctorTask
            events = try await eventsTask
            logTail = try await logsTask
            normalizeSelection()
            synchronizeActiveUsage()
            resetAutoSwitchConflictSuppressionIfNeeded()
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

    func setCodexUsageSourceMode(_ mode: UsageSourceMode) async {
        await updateCodexSettings(CodexSettingsDraft(sourceMode: mode))
    }

    func refreshUsage(profileId: String) async {
        guard refreshingUsageProfileIds.insert(profileId).inserted else {
            return
        }
        defer {
            refreshingUsageProfileIds.remove(profileId)
        }

        do {
            let snapshot = try await client.refreshUsage(profileId: profileId)
            mergeUsageSnapshot(snapshot)
            finalizeUsageRefresh()
        } catch {
            lastErrorMessage = error.localizedDescription
        }
    }

    func refreshEnabledUsage() async {
        guard !isRefreshingEnabledUsage else {
            return
        }

        isRefreshingEnabledUsage = true
        defer {
            isRefreshingEnabledUsage = false
        }

        do {
            let snapshots = try await client.refreshEnabledUsage()
            for snapshot in snapshots {
                mergeUsageSnapshot(snapshot)
            }
            finalizeUsageRefresh()
            await attemptAutoSwitchIfNeeded()
        } catch {
            lastErrorMessage = error.localizedDescription
        }
    }

    func refreshForMenuOpen() async {
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

    func loginProfile(agent: AgentKind, nickname: String?, priority: Int) async -> AddAccountResult {
        guard !isMutatingProfiles else {
            return .failed(detail: "Another profile change is already in progress.")
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
            return .success
        } catch is CancellationError {
            lastErrorMessage = nil
            return .cancelled
        } catch {
            let outcome = addAccountResult(for: error, agent: agent)
            switch outcome {
            case .success:
                lastErrorMessage = nil
            case .cancelled:
                lastErrorMessage = nil
            case let .notSignedIn(detail):
                lastErrorMessage = "\(agent.rawValue): Not signed in. \(detail)"
            case let .failed(detail):
                lastErrorMessage = detail
                await notificationService.post(
                    title: "Relay profile update failed",
                    body: detail
                )
            }
            return outcome
        }
    }

    func addAccount(agent: AgentKind, priority: Int = 100) async -> AddAccountResult {
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

    private func updateCodexSettings(_ draft: CodexSettingsDraft) async {
        do {
            codexSettings = try await client.setCodexSettings(draft)
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay Codex settings update failed",
                body: error.localizedDescription
            )
        }
    }

    private func addAccountResult(for error: Error, agent: AgentKind) -> AddAccountResult {
        let description = error.localizedDescription.trimmingCharacters(in: .whitespacesAndNewlines)
        let normalized = description.lowercased()

        if normalized.contains("timed out waiting for browser sign-in")
            || normalized.contains("did not complete successfully")
            || normalized.contains("without creating auth.json")
            || normalized.contains("login cancelled")
            || normalized.contains("login canceled")
            || normalized.contains("sign-in cancelled")
            || normalized.contains("sign-in canceled")
        {
            return .notSignedIn(detail: "Browser sign-in was cancelled or did not complete.")
        }

        return .failed(detail: description.isEmpty ? "\(agent.rawValue) login failed." : description)
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

    private func synchronizeActiveUsage() {
        guard let activeProfileId else {
            usage = nil
            return
        }
        usage = usageSnapshot(for: activeProfileId)
    }

    private func finalizeUsageRefresh() {
        synchronizeActiveUsage()
        resetAutoSwitchConflictSuppressionIfNeeded()
        lastRefresh = Date()
        lastErrorMessage = nil
    }

    private func attemptAutoSwitchIfNeeded() async {
        guard autoSwitchEnabled else {
            lastAutoSwitchConflictSignature = nil
            return
        }
        guard !isSwitching, !isAutoSwitching else {
            return
        }
        guard let activeProfileId else {
            lastAutoSwitchConflictSignature = nil
            return
        }
        guard let activeSnapshot = usageSnapshot(for: activeProfileId), activeSnapshot.canAutoSwitch else {
            lastAutoSwitchConflictSignature = nil
            return
        }

        let conflictSignature = autoSwitchConflictSignature(activeProfileId: activeProfileId)
        if lastAutoSwitchConflictSignature == conflictSignature {
            return
        }

        isAutoSwitching = true
        defer {
            isAutoSwitching = false
        }

        do {
            let report = try await client.switchToNextProfile()
            lastAutoSwitchConflictSignature = nil
            selectProfile(report.profileId)
            await refresh()
            await notificationService.post(
                title: "Relay auto-switched profile",
                body: report.message
            )
        } catch {
            lastErrorMessage = error.localizedDescription
            if isAutoSwitchExhaustedConflict(error) {
                lastAutoSwitchConflictSignature = conflictSignature
                await notificationService.post(
                    title: "Relay auto-switch paused",
                    body: "All enabled profiles are exhausted or unavailable for auto-switch. Staying on current profile."
                )
                return
            }

            lastAutoSwitchConflictSignature = nil
            await notificationService.post(
                title: "Relay auto-switch failed",
                body: error.localizedDescription
            )
        }
    }

    private func resetAutoSwitchConflictSuppressionIfNeeded() {
        guard autoSwitchEnabled, let activeProfileId else {
            lastAutoSwitchConflictSignature = nil
            return
        }
        guard let activeSnapshot = usageSnapshot(for: activeProfileId), activeSnapshot.canAutoSwitch else {
            lastAutoSwitchConflictSignature = nil
            return
        }

        let currentSignature = autoSwitchConflictSignature(activeProfileId: activeProfileId)
        if lastAutoSwitchConflictSignature != currentSignature {
            lastAutoSwitchConflictSignature = nil
        }
    }

    private func autoSwitchConflictSignature(activeProfileId: String) -> String {
        let enabledState = profiles
            .filter(\.enabled)
            .map { profile -> String in
                let snapshot = usageSnapshot(for: profile.id)
                let confidence = snapshot?.confidence.rawValue ?? "missing"
                let stale = snapshot.map { String($0.stale) } ?? "missing"
                let session = snapshot?.session.status.rawValue ?? "missing"
                let weekly = snapshot?.weekly.status.rawValue ?? "missing"
                return "\(profile.id):\(confidence):\(stale):\(session):\(weekly)"
            }
            .joined(separator: "|")
        return "\(activeProfileId)|\(enabledState)"
    }

    private func isAutoSwitchExhaustedConflict(_ error: Error) -> Bool {
        guard case let RelayCLIClientError.commandFailed(code, _) = error else {
            return false
        }
        return code == "RELAY_CONFLICT"
    }
}
