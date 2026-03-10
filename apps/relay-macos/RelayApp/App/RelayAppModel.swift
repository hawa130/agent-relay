import Defaults
import Foundation
import SwiftUI

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
    @Published private(set) var engineConnectionState: EngineConnectionState = .starting
    @Published private(set) var isRefreshing = false
    @Published private(set) var isSwitching = false
    @Published private(set) var isMutatingProfiles = false
    @Published private(set) var isRefreshingEnabledUsage = false
    @Published private(set) var refreshingUsageProfileIds: Set<String> = []
    @Published var selectedProfileId: String?
    @Published var lastErrorMessage: String?

    private let daemonClient = RelayDaemonClient()
    private let legacyClient = RelayCLIClient()
    private let notificationService = RelayNotificationService()
    private var hasStarted = false
    private var daemonNotificationsTask: Task<Void, Never>?

    public init() {
        selectedProfileId = Defaults[.selectedProfileId]
    }

    public var menuBarTitle: String {
        activeProfile?.nickname ?? "Relay"
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

    var refreshIntervalSeconds: Int {
        status?.settings.refreshIntervalSeconds ?? 60
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
            await startDaemonSession()
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
            async let statusTask = daemonClient.fetchStatus()
            async let codexSettingsTask = daemonClient.fetchCodexSettings()
            async let profileListTask = daemonClient.fetchProfileList()
            async let doctorTask = daemonClient.fetchDoctor()
            async let eventsTask = daemonClient.fetchEvents(limit: 10)
            async let logsTask = daemonClient.fetchLogs(lines: 25)

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
            let report = try await daemonClient.switchToProfile(profileId)
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
            _ = try await daemonClient.setAutoSwitch(enabled: enabled)
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay settings update failed",
                body: error.localizedDescription
            )
        }
    }

    func setRefreshInterval(seconds: Int) async {
        do {
            _ = try await daemonClient.setRefreshInterval(seconds: seconds)
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
            let snapshot = try await daemonClient.refreshUsage(profileId: profileId)
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
            let snapshots = try await daemonClient.refreshEnabledUsage()
            for snapshot in snapshots {
                mergeUsageSnapshot(snapshot)
            }
            finalizeUsageRefresh()
        } catch {
            lastErrorMessage = error.localizedDescription
        }
    }

    func refreshForMenuOpen() async {
        await refreshIfStale(maxAge: 15)
    }

    func setProfileEnabled(_ profileId: String, enabled: Bool) async {
        await performProfileMutation { [self] in
            _ = try await self.daemonClient.setProfileEnabled(profileId: profileId, enabled: enabled)
        }
    }

    func editProfile(profileId: String, draft: ProfileDraft) async {
        await performProfileMutation { [self] in
            _ = try await self.daemonClient.editProfile(profileId: profileId, draft: draft)
        }
    }

    func removeProfile(_ profileId: String) async {
        await performProfileMutation { [self] in
            _ = try await self.daemonClient.removeProfile(profileId: profileId)
        }
    }

    func importProfile(agent: AgentKind, nickname: String?, priority: Int) async {
        await performProfileMutation { [self] in
            let profile = try await self.daemonClient.importProfile(
                agent: agent,
                nickname: nickname,
                priority: priority
            )
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
            let result = try await legacyClient.loginProfile(
                agent: agent,
                nickname: nickname,
                priority: priority
            )
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
            case .success, .cancelled:
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
            diagnosticsExport = try await daemonClient.exportDiagnostics()
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay diagnostics export failed",
                body: error.localizedDescription
            )
        }
    }

    func restartEngine() async {
        engineConnectionState = .starting
        do {
            let initial = try await daemonClient.restart()
            apply(initialState: initial)
            await refresh()
        } catch {
            engineConnectionState = .degraded
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay engine restart failed",
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
            codexSettings = try await daemonClient.setCodexSettings(draft)
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay Codex settings update failed",
                body: error.localizedDescription
            )
        }
    }

    private func startDaemonSession() async {
        do {
            let initial = try await daemonClient.start()
            apply(initialState: initial)
            startNotificationStreamIfNeeded()
            await refresh()
        } catch {
            engineConnectionState = .degraded
            lastErrorMessage = error.localizedDescription
        }
    }

    private func startNotificationStreamIfNeeded() {
        guard daemonNotificationsTask == nil else {
            return
        }

        daemonNotificationsTask = Task { [weak self] in
            guard let self else {
                return
            }
            for await update in daemonClient.notifications {
                await MainActor.run {
                    self.handle(update)
                }
            }
        }
    }

    private func apply(initialState: RPCInitialState) {
        status = initialState.status
        codexSettings = initialState.codexSettings
        profiles = initialState.profiles.map(\.profile)
        usageSnapshots = initialState.profiles.compactMap(\.usageSummary)
        engineConnectionState = initialState.engine.connectionState
        normalizeSelection()
        synchronizeActiveUsage()
        lastRefresh = Date()
        lastErrorMessage = nil
    }

    private func handle(_ update: RelaySessionUpdate) {
        switch update {
        case let .usageUpdated(payload):
            for snapshot in payload.snapshots {
                mergeUsageSnapshot(snapshot)
            }
            finalizeUsageRefresh()
        case let .activeStateUpdated(payload):
            if var status {
                status.activeState = payload.activeState
                self.status = status
            }
            normalizeSelection()
            synchronizeActiveUsage()
        case let .switchCompleted(payload):
            lastErrorMessage = nil
            if payload.trigger == .auto {
                Task {
                    await notificationService.post(
                        title: "Relay auto-switched profile",
                        body: payload.report.message
                    )
                }
            }
        case let .switchFailed(payload):
            lastErrorMessage = "\(payload.errorCode): \(payload.message)"
            if payload.trigger == .auto {
                Task {
                    await notificationService.post(
                        title: "Relay auto-switch failed",
                        body: payload.message
                    )
                }
            }
        case let .healthUpdated(payload):
            engineConnectionState = payload.state
            if let detail = payload.detail, payload.state == .degraded {
                lastErrorMessage = detail
            }
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
        lastRefresh = Date()
        lastErrorMessage = nil
    }
}
