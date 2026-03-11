import Defaults
import Foundation
import SwiftUI

private final class TaskIDBox: @unchecked Sendable {
    private let lock = NSLock()
    private var taskID: String?
    private var cancelRequested = false

    func set(_ value: String) -> Bool {
        lock.lock()
        taskID = value
        let shouldCancel = cancelRequested
        lock.unlock()
        return shouldCancel
    }

    func requestCancel() -> String? {
        lock.lock()
        cancelRequested = true
        let value = taskID
        lock.unlock()
        return value
    }
}

private enum TaskTerminalWaitOutcome {
    case update(RelayTaskUpdate)
    case daemonFailure(detail: String)
}

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
    @Published private(set) var isLoggingIn = false
    @Published private(set) var isFetchingEnabledUsage = false
    @Published private(set) var isRefreshingUsageList = false
    @Published private(set) var fetchingUsageProfileIds: Set<String> = []
    @Published var selectedProfileId: String?

    private enum QueryPendingKey: Hashable {
        case usageAll
        case usageProfile(String)
        case activityEvents
        case activityLogs
        case doctor
    }

    private enum MutationPendingKey: Hashable {
        case switching
        case profileMutation
        case loginTask
        case restartEngine
    }

    private let daemonClient = RelayDaemonClient()
    private let notificationService = RelayNotificationService()
    private var hasStarted = false
    private var daemonNotificationsTask: Task<Void, Never>?
    private var queryPending: Set<QueryPendingKey> = []
    private var mutationPending: Set<MutationPendingKey> = []
    private var queryStates: [QueryStateKey: QueryStateItem] = [:]
    private var taskUpdates: [String: RelayTaskUpdate] = [:]
    private var taskWaiters: [String: CheckedContinuation<TaskTerminalWaitOutcome, Never>] = [:]
    private var currentLoginTaskID: String?
    private var loginCancellationRequested = false
    private var bulkUsageRefreshRequested = false

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

    var networkQueryConcurrency: Int {
        status?.settings.networkQueryConcurrency ?? 10
    }

    func usageSnapshot(for profileId: String) -> UsageSnapshot? {
        usageSnapshots.first { $0.profileId == profileId }
    }

    func isFetchingUsage(profileId: String) -> Bool {
        usageQueryState(for: profileId)?.status == .pending
    }

    func usageRefreshError(profileId: String) -> String? {
        guard let state = usageQueryState(for: profileId), state.status == .error else {
            return nil
        }
        return state.message
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
        do {
            try await ensureSessionStateLoaded()
            triggerRefreshEnabledUsage(notifyOnFailure: notifyOnFailure)
            triggerRefreshActivity(notifyOnFailure: notifyOnFailure)
            triggerRefreshDoctor(notifyOnFailure: notifyOnFailure)
        } catch {
            if notifyOnFailure {
                await notificationService.post(
                    title: "Relay refresh failed",
                    body: error.localizedDescription
                )
            }
        }
    }

    func switchToProfile(_ profileId: String) async {
        guard !mutationPending.contains(.switching) else {
            return
        }

        beginMutation(.switching)
        defer {
            endMutation(.switching)
        }

        do {
            try await ensureSessionStateLoaded()
            let report = try await daemonClient.switchToProfile(profileId)
            selectProfile(profileId)
            await notificationService.post(
                title: "Relay switched profile",
                body: report.message
            )
        } catch {
            await notificationService.post(
                title: "Relay switch failed",
                body: error.localizedDescription
            )
        }
    }

    func setAutoSwitch(enabled: Bool) async {
        var previousStatus: StatusReport?
        var previousCodexSettings: CodexSettings?
        do {
            try await ensureSessionStateLoaded()
            previousStatus = status
            previousCodexSettings = codexSettings
            applyAppSettingsOptimistic(autoSwitchEnabled: enabled)
            _ = try await daemonClient.setAutoSwitch(enabled: enabled)
        } catch {
            rollbackSettingsIfNeeded(
                previousStatus: previousStatus,
                previousCodexSettings: previousCodexSettings
            )
            await notificationService.post(
                title: "Relay settings update failed",
                body: error.localizedDescription
            )
        }
    }

    func setRefreshInterval(seconds: Int) async {
        var previousStatus: StatusReport?
        var previousCodexSettings: CodexSettings?
        do {
            try await ensureSessionStateLoaded()
            previousStatus = status
            previousCodexSettings = codexSettings
            applyAppSettingsOptimistic(refreshIntervalSeconds: seconds)
            _ = try await daemonClient.setRefreshInterval(seconds: seconds)
        } catch {
            rollbackSettingsIfNeeded(
                previousStatus: previousStatus,
                previousCodexSettings: previousCodexSettings
            )
            await notificationService.post(
                title: "Relay settings update failed",
                body: error.localizedDescription
            )
        }
    }

    func setNetworkQueryConcurrency(value: Int) async {
        var previousStatus: StatusReport?
        var previousCodexSettings: CodexSettings?
        do {
            try await ensureSessionStateLoaded()
            previousStatus = status
            previousCodexSettings = codexSettings
            applyAppSettingsOptimistic(networkQueryConcurrency: value)
            _ = try await daemonClient.setNetworkQueryConcurrency(value: value)
        } catch {
            rollbackSettingsIfNeeded(
                previousStatus: previousStatus,
                previousCodexSettings: previousCodexSettings
            )
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
        guard
            !isFetchingUsage(profileId: profileId),
            !queryPending.contains(.usageProfile(profileId))
        else {
            return
        }
        do {
            try await ensureSessionStateLoaded()
            triggerBackgroundQuery([.usageProfile(profileId)], failureTitle: "Relay usage refresh failed") { [daemonClient] in
                _ = try await daemonClient.refreshUsage(profileId: profileId)
            }
        } catch {
            return
        }
    }

    func refreshEnabledUsage() async {
        guard !isFetchingEnabledUsage, !queryPending.contains(.usageAll) else {
            return
        }
        do {
            try await ensureSessionStateLoaded()
            triggerRefreshEnabledUsage(notifyOnFailure: false)
        } catch {
            return
        }
    }

    func refreshAllUsage() async {
        guard !isFetchingEnabledUsage, !queryPending.contains(.usageAll) else {
            return
        }
        do {
            try await ensureSessionStateLoaded()
            triggerRefreshAllUsage(notifyOnFailure: false)
        } catch {
            return
        }
    }

    func refreshForMenuOpen() async {
        await refreshIfStale(maxAge: 15)
    }

    func setProfileEnabled(_ profileId: String, enabled: Bool) async {
        await performProfileMutation { [daemonClient] in
            _ = try await daemonClient.setProfileEnabled(
                profileId: profileId,
                enabled: enabled
            )
        }
    }

    func editProfile(profileId: String, draft: ProfileDraft) async {
        await performProfileMutation { [daemonClient] in
            _ = try await daemonClient.editProfile(profileId: profileId, draft: draft)
        }
    }

    func removeProfile(_ profileId: String) async {
        await performProfileMutation { [daemonClient] in
            _ = try await daemonClient.removeProfile(profileId: profileId)
        }
    }

    func importProfile(agent: AgentKind, nickname: String?, priority: Int) async {
        await performProfileMutation { [self, daemonClient] in
            let profile = try await daemonClient.importProfile(
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
        guard !mutationPending.contains(.profileMutation), !mutationPending.contains(.loginTask) else {
            return .failed(detail: "Another profile change is already in progress.")
        }

        beginMutation(.loginTask)
        defer {
            currentLoginTaskID = nil
            loginCancellationRequested = false
            endMutation(.loginTask)
        }

        do {
            try await ensureSessionStateLoaded()
            loginCancellationRequested = false
            currentLoginTaskID = nil
            let taskIDBox = TaskIDBox()
            let outcome = try await withTaskCancellationHandler {
                let start = try await daemonClient.startLoginProfile(
                    agent: agent,
                    nickname: nickname,
                    priority: priority
                )
                let shouldCancel = taskIDBox.set(start.taskId)
                currentLoginTaskID = start.taskId
                if loginCancellationRequested {
                    _ = try? await daemonClient.cancelTask(taskId: start.taskId)
                }
                if shouldCancel {
                    Task.detached { [daemonClient] in
                        _ = try? await daemonClient.cancelTask(taskId: start.taskId)
                    }
                    try Task.checkCancellation()
                }
                let terminal = await waitForTaskTerminal(taskId: start.taskId)
                switch terminal {
                case let .update(taskUpdate):
                    return handleLoginTaskUpdate(taskUpdate, agent: agent)
                case let .daemonFailure(detail):
                    return .failed(detail: detail)
                }
            } onCancel: {
                guard let taskID = taskIDBox.requestCancel() else {
                    return
                }
                Task.detached { [daemonClient] in
                    _ = try? await daemonClient.cancelTask(taskId: taskID)
                }
            }
            await applyAddAccountOutcome(outcome)
            return outcome
        } catch {
            let outcome = addAccountResult(for: error, agent: agent)
            await applyAddAccountOutcome(outcome)
            return outcome
        }
    }

    func addAccount(agent: AgentKind, priority: Int = 100) async -> AddAccountResult {
        await loginProfile(agent: agent, nickname: nil, priority: priority)
    }

    func cancelLogin() async {
        guard mutationPending.contains(.loginTask), !loginCancellationRequested else {
            return
        }
        loginCancellationRequested = true
        guard let taskID = await waitForCurrentLoginTaskID() else {
            return
        }
        _ = try? await daemonClient.cancelTask(taskId: taskID)
    }

    func exportDiagnostics() async {
        do {
            diagnosticsExport = try await daemonClient.exportDiagnostics()
        } catch {
            await notificationService.post(
                title: "Relay diagnostics export failed",
                body: error.localizedDescription
            )
        }
    }

    func restartEngine() async {
        guard !mutationPending.contains(.restartEngine) else {
            return
        }

        beginMutation(.restartEngine)
        engineConnectionState = .starting
        defer {
            endMutation(.restartEngine)
        }

        do {
            let initial = try await daemonClient.restart()
            apply(initialState: initial)
        } catch {
            engineConnectionState = .degraded
            await notificationService.post(
                title: "Relay engine restart failed",
                body: error.localizedDescription
            )
        }
    }

    private func performProfileMutation(
        _ operation: @escaping @Sendable () async throws -> Void
    ) async {
        guard !mutationPending.contains(.profileMutation) else {
            return
        }

        beginMutation(.profileMutation)
        defer {
            endMutation(.profileMutation)
        }

        do {
            try await ensureSessionStateLoaded()
            try await operation()
        } catch {
            await notificationService.post(
                title: "Relay profile update failed",
                body: error.localizedDescription
            )
        }
    }

    private func updateCodexSettings(_ draft: CodexSettingsDraft) async {
        var previousStatus: StatusReport?
        var previousCodexSettings: CodexSettings?
        do {
            try await ensureSessionStateLoaded()
            previousStatus = status
            previousCodexSettings = codexSettings
            if let sourceMode = draft.sourceMode {
                codexSettings = CodexSettings(usageSourceMode: sourceMode)
            }
            _ = try await daemonClient.setCodexSettings(draft)
        } catch {
            rollbackSettingsIfNeeded(
                previousStatus: previousStatus,
                previousCodexSettings: previousCodexSettings
            )
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
        } catch {
            engineConnectionState = .degraded
        }
    }

    private func ensureSessionStateLoaded() async throws {
        guard status == nil || codexSettings == nil else {
            return
        }

        let initial = try await daemonClient.start()
        apply(initialState: initial)
        startNotificationStreamIfNeeded()
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

    private func handle(_ update: RelaySessionUpdate) {
        switch update {
        case let .usageUpdated(payload):
            for snapshot in payload.snapshots {
                mergeUsageSnapshot(snapshot)
            }
            finalizeStateUpdate()
        case let .queryStateUpdated(payload):
            applyQueryStates(payload.states)
            finalizeStateUpdate()
        case let .activeStateUpdated(payload):
            if var status {
                status.activeState = payload.activeState
                self.status = status
            }
            if let activeProfile = payload.activeProfile {
                applyProfile(activeProfile.profile)
            }
            normalizeSelection()
            synchronizeActiveUsage()
            finalizeStateUpdate()
        case let .settingsUpdated(payload):
            applySettingsResult(payload.settings)
            finalizeStateUpdate()
        case let .profilesUpdated(payload):
            applyProfileItems(payload.profiles)
            finalizeStateUpdate()
        case let .activityEventsUpdated(payload):
            events = payload.events
            endQuery(.activityEvents)
            finalizeStateUpdate()
        case let .activityLogsUpdated(payload):
            logTail = payload.logs
            endQuery(.activityLogs)
            finalizeStateUpdate()
        case let .doctorUpdated(payload):
            doctor = payload.report
            endQuery(.doctor)
            finalizeStateUpdate()
        case let .switchCompleted(payload):
            applySwitchCompleted(payload.report)
            if payload.trigger == .auto {
                Task {
                    await notificationService.post(
                        title: "Relay auto-switched profile",
                        body: payload.report.message
                    )
                }
            }
        case let .switchFailed(payload):
            applySwitchFailed(payload)
            if payload.trigger == .auto {
                Task {
                    await notificationService.post(
                        title: "Relay auto-switch failed",
                        body: payload.message
                    )
                }
            }
        case let .taskUpdated(payload):
            applyTaskUpdate(payload.task)
            finalizeStateUpdate()
        case let .healthUpdated(payload):
            engineConnectionState = payload.state
            if let detail = payload.detail, payload.state == .degraded {
                failPendingTasks(detail: detail)
            }
            finalizeStateUpdate()
        }
    }

    private func triggerRefreshEnabledUsage(notifyOnFailure: Bool) {
        bulkUsageRefreshRequested = true
        triggerBackgroundQuery([.usageAll], failureTitle: notifyOnFailure ? "Relay refresh failed" : nil) { [daemonClient] in
            _ = try await daemonClient.refreshEnabledUsage()
        }
    }

    private func triggerRefreshAllUsage(notifyOnFailure: Bool) {
        bulkUsageRefreshRequested = true
        triggerBackgroundQuery([.usageAll], failureTitle: notifyOnFailure ? "Relay refresh failed" : nil) { [daemonClient] in
            _ = try await daemonClient.refreshAllUsage()
        }
    }

    private func applyTaskUpdate(_ task: RelayTaskUpdate) {
        taskUpdates[task.taskId] = task
        if task.isTerminal, let waiter = taskWaiters.removeValue(forKey: task.taskId) {
            waiter.resume(returning: .update(task))
        }
    }

    private func waitForTaskTerminal(taskId: String) async -> TaskTerminalWaitOutcome {
        if let existing = taskUpdates[taskId], existing.isTerminal {
            return .update(existing)
        }

        return await withCheckedContinuation { continuation in
            taskWaiters[taskId] = continuation
        }
    }

    private func waitForCurrentLoginTaskID(
        timeoutNanoseconds: UInt64 = 2_000_000_000
    ) async -> String? {
        if let currentLoginTaskID {
            return currentLoginTaskID
        }

        let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
        while DispatchTime.now().uptimeNanoseconds < deadline {
            if let currentLoginTaskID {
                return currentLoginTaskID
            }
            if !mutationPending.contains(.loginTask) {
                return nil
            }
            try? await Task.sleep(nanoseconds: 20_000_000)
        }
        return currentLoginTaskID
    }

    private func handleLoginTaskUpdate(
        _ task: RelayTaskUpdate,
        agent: AgentKind
    ) -> AddAccountResult {
        switch task.status {
        case .pending:
            return .failed(detail: "Login task did not complete.")
        case .succeeded:
            guard let result = task.profileLoginResult else {
                return .failed(detail: "\(agent.rawValue) login completed without a result.")
            }
            selectProfile(result.profile.id)
            return .success
        case .cancelled:
            return .notSignedIn(detail: "Browser sign-in was cancelled or did not complete.")
        case .failed:
            let detail =
                task.message
                ?? "\(task.errorCode ?? "RELAY_INTERNAL"): login failed"
            if let errorCode = task.errorCode {
                return .failed(detail: "\(errorCode): \(detail)")
            }
            return .failed(detail: detail)
        }
    }

    private func failPendingTasks(detail: String) {
        let pendingWaiters = taskWaiters
        taskWaiters.removeAll()
        for (_, waiter) in pendingWaiters {
            waiter.resume(returning: .daemonFailure(detail: detail))
        }
    }

    private func triggerRefreshActivity(notifyOnFailure: Bool) {
        triggerBackgroundQuery(
            [.activityEvents, .activityLogs],
            failureTitle: notifyOnFailure ? "Relay activity refresh failed" : nil
        ) { [daemonClient] in
            _ = try await daemonClient.refreshActivity()
        }
    }

    private func triggerRefreshDoctor(notifyOnFailure: Bool) {
        triggerBackgroundQuery(
            [.doctor],
            failureTitle: notifyOnFailure ? "Relay diagnostics refresh failed" : nil
        ) { [daemonClient] in
            _ = try await daemonClient.refreshDoctor()
        }
    }

    private func triggerBackgroundQuery(
        _ keys: Set<QueryPendingKey>,
        failureTitle: String?,
        operation: @escaping @Sendable () async throws -> Void
    ) {
        guard queryPending.isDisjoint(with: keys) else {
            return
        }

        beginQueries(keys)
        Task { [weak self] in
            do {
                try await operation()
            } catch {
                await MainActor.run {
                    self?.endQueries(keys)
                }
                if let failureTitle {
                    await self?.notificationService.post(
                        title: failureTitle,
                        body: error.localizedDescription
                    )
                }
            }
        }
    }

    private func beginQueries(_ keys: Set<QueryPendingKey>) {
        queryPending.formUnion(keys)
        synchronizePendingState()
    }

    private func endQueries(_ keys: Set<QueryPendingKey>) {
        queryPending.subtract(keys)
        synchronizePendingState()
    }

    private func endQuery(_ key: QueryPendingKey) {
        queryPending.remove(key)
        synchronizePendingState()
    }

    private func beginMutation(_ key: MutationPendingKey) {
        mutationPending.insert(key)
        synchronizePendingState()
    }

    private func endMutation(_ key: MutationPendingKey) {
        mutationPending.remove(key)
        synchronizePendingState()
    }

    private func synchronizePendingState() {
        let activeUsageStates = queryStates.values.filter { item in
            item.key.kind == .usageProfile
        }
        isFetchingEnabledUsage = activeUsageStates.contains { $0.status == .pending }
        fetchingUsageProfileIds = Set<String>(
            activeUsageStates.compactMap { item in
                guard item.status == .pending else {
                    return nil
                }
                return item.key.profileId
            }
        )
        if bulkUsageRefreshRequested {
            let bulkRefreshActive = queryPending.contains(.usageAll) || !fetchingUsageProfileIds.isEmpty
            isRefreshingUsageList = bulkRefreshActive
            if !bulkRefreshActive {
                bulkUsageRefreshRequested = false
            }
        } else {
            isRefreshingUsageList = false
        }
        isRefreshing = !queryPending.isEmpty || isFetchingEnabledUsage
        isSwitching = mutationPending.contains(.switching)
        isMutatingProfiles = mutationPending.contains(.profileMutation)
        isLoggingIn = mutationPending.contains(.loginTask)
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

    private func applyAddAccountOutcome(_ outcome: AddAccountResult) async {
        switch outcome {
        case .success, .cancelled, .notSignedIn:
            return
        case let .failed(detail):
            await notificationService.post(
                title: "Relay profile update failed",
                body: detail
            )
        }
    }

    private func apply(initialState: RPCInitialState) {
        status = initialState.status
        codexSettings = initialState.codexSettings
        applyProfileItems(initialState.profiles)
        queryStates = [:]
        doctor = nil
        events = []
        logTail = nil
        engineConnectionState = initialState.engine.connectionState
        bulkUsageRefreshRequested = false
        normalizeSelection()
        synchronizeActiveUsage()
        synchronizePendingState()
        finalizeStateUpdate()
    }

    private func applySettingsResult(_ result: RPCSettingsResult) {
        codexSettings = result.codex
        guard let status else {
            return
        }
        self.status = StatusReport(
            relayHome: status.relayHome,
            liveAgentHome: status.liveAgentHome,
            profileCount: status.profileCount,
            activeState: status.activeState,
            settings: result.app
        )
    }

    private func applyAppSettingsOptimistic(
        autoSwitchEnabled: Bool? = nil,
        refreshIntervalSeconds: Int? = nil,
        networkQueryConcurrency: Int? = nil
    ) {
        guard let status else {
            return
        }

        let currentSettings = status.settings
        let nextSettings = AppSettings(
            autoSwitchEnabled: autoSwitchEnabled ?? currentSettings.autoSwitchEnabled,
            cooldownSeconds: currentSettings.cooldownSeconds,
            refreshIntervalSeconds: refreshIntervalSeconds ?? currentSettings.refreshIntervalSeconds,
            networkQueryConcurrency: networkQueryConcurrency ?? currentSettings.networkQueryConcurrency
        )
        let nextActiveState = ActiveState(
            activeProfileId: status.activeState.activeProfileId,
            lastSwitchAt: status.activeState.lastSwitchAt,
            lastSwitchResult: status.activeState.lastSwitchResult,
            autoSwitchEnabled: autoSwitchEnabled ?? status.activeState.autoSwitchEnabled
        )

        self.status = StatusReport(
            relayHome: status.relayHome,
            liveAgentHome: status.liveAgentHome,
            profileCount: status.profileCount,
            activeState: nextActiveState,
            settings: nextSettings
        )
        finalizeStateUpdate()
    }

    private func rollbackSettingsIfNeeded(
        previousStatus: StatusReport?,
        previousCodexSettings: CodexSettings?
    ) {
        if let previousStatus {
            status = previousStatus
        }
        if let previousCodexSettings {
            codexSettings = previousCodexSettings
        }
        synchronizeActiveUsage()
    }

    private func applyProfileItems(_ items: [ProfileListItem]) {
        profiles = items.map(\.profile)
        usageSnapshots = items.compactMap(\.usageSummary)
        normalizeSelection()
        synchronizeActiveUsage()
        if let status {
            self.status = StatusReport(
                relayHome: status.relayHome,
                liveAgentHome: status.liveAgentHome,
                profileCount: profiles.count,
                activeState: status.activeState,
                settings: status.settings
            )
        }
    }

    private func applyProfile(_ profile: Profile) {
        if let index = profiles.firstIndex(where: { $0.id == profile.id }) {
            profiles[index] = profile
        } else {
            profiles.append(profile)
        }
        normalizeSelection()
        synchronizeActiveUsage()
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

    private func finalizeStateUpdate() {
        lastRefresh = Date()
    }

    private func usageQueryState(for profileId: String) -> QueryStateItem? {
        queryStates[QueryStateKey(kind: .usageProfile, profileId: profileId)]
    }

    private func applyQueryStates(_ states: [QueryStateItem]) {
        queryStates = Dictionary(uniqueKeysWithValues: states.map { ($0.key, $0) })
        clearLocalUsageQueryGates()
        synchronizePendingState()
    }

    private func clearLocalUsageQueryGates() {
        queryPending = Set(
            queryPending.filter { key in
                switch key {
                case .usageAll, .usageProfile:
                    return false
                default:
                    return true
                }
            }
        )
    }

    private func applySwitchCompleted(_ report: SwitchReport) {
        guard let status else {
            return
        }
        self.status = StatusReport(
            relayHome: status.relayHome,
            liveAgentHome: status.liveAgentHome,
            profileCount: status.profileCount,
            activeState: ActiveState(
                activeProfileId: report.profileId,
                lastSwitchAt: report.switchedAt,
                lastSwitchResult: .success,
                autoSwitchEnabled: status.activeState.autoSwitchEnabled
            ),
            settings: status.settings
        )
        normalizeSelection()
        synchronizeActiveUsage()
    }

    private func applySwitchFailed(_ payload: SwitchFailedNotification) {
        guard let status else {
            return
        }
        self.status = StatusReport(
            relayHome: status.relayHome,
            liveAgentHome: status.liveAgentHome,
            profileCount: status.profileCount,
            activeState: ActiveState(
                activeProfileId: status.activeState.activeProfileId,
                lastSwitchAt: status.activeState.lastSwitchAt,
                lastSwitchResult: .failed,
                autoSwitchEnabled: status.activeState.autoSwitchEnabled
            ),
            settings: status.settings
        )
    }
}
