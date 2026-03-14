import Darwin
import Foundation

actor RelayDaemonClient {
    private static let defaultRequestTimeoutSeconds: TimeInterval = 30
    private static let minimumRequestTimeoutSeconds: TimeInterval = 0.1

    private let relayCLIPathOverride: String?
    private let environment: [String: String]
    private let requestTimeoutSeconds: TimeInterval
    private var process: Process?
    private var stdinPipe: Pipe?
    private var stdoutBuffer = Data()
    private var nextRequestID = 0
    private var pending: [String: CheckedContinuation<Data, Error>] = [:]
    private var pendingTimeouts: [String: Task<Void, Never>] = [:]
    private var initializedState: RPCInitialState?
    private var startTask: Task<RPCInitialState, Error>?
    private var isStopping = false
    private let stream: AsyncStream<RelaySessionUpdate>
    private let continuation: AsyncStream<RelaySessionUpdate>.Continuation

    init(
        relayCLIPathOverride: String? = nil,
        requestTimeoutSeconds: TimeInterval? = nil,
        environment: [String: String] = ProcessInfo.processInfo.environment)
    {
        self.relayCLIPathOverride = relayCLIPathOverride
        self.environment = environment
        self.requestTimeoutSeconds = Self.resolveRequestTimeoutSeconds(
            override: requestTimeoutSeconds,
            environment: environment)
        let streamPair = AsyncStream.makeStream(of: RelaySessionUpdate.self)
        stream = streamPair.stream
        continuation = streamPair.continuation
    }

    nonisolated var notifications: AsyncStream<RelaySessionUpdate> {
        stream
    }

    func start() async throws -> RPCInitialState {
        if let initializedState {
            return initializedState
        }

        if let startTask {
            return try await startTask.value
        }

        let task = Task { try await self.performStart() }
        startTask = task

        do {
            let initial = try await task.value
            startTask = nil
            return initial
        } catch {
            startTask = nil
            throw error
        }
    }

    private func performStart() async throws -> RPCInitialState {
        if let initializedState {
            return initializedState
        }

        let command = try resolvedRelayCLIPath()
        let process = Process()
        let stdout = Pipe()
        let stderr = Pipe()
        let stdin = Pipe()

        process.executableURL = URL(fileURLWithPath: command)
        process.arguments = ["daemon", "--stdio"]
        process.standardOutput = stdout
        process.standardError = stderr
        process.standardInput = stdin
        process.environment = environment
        process.terminationHandler = { [weak self] process in
            Task {
                await self?.handleTermination(process)
            }
        }

        stdout.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            Task {
                await self?.consumeStdout(data)
            }
        }

        stderr.fileHandleForReading.readabilityHandler = { handle in
            _ = handle.availableData
        }

        do {
            try process.run()
        } catch {
            throw RelayClientError.launchFailed(error.localizedDescription)
        }

        self.process = process
        stdinPipe = stdin

        let initial: RPCInitializeResult = try await sendRequest(
            method: "initialize",
            params: RPCInitializeParams(),
            as: RPCInitializeResult.self)
        _ = try await sendRequest(
            method: "session/subscribe",
            params: RPCSubscribeParams(
                topics: [
                    "usage.updated",
                    "query_state.updated",
                    "active_state.updated",
                    "settings.updated",
                    "profiles.updated",
                    "activity.events.updated",
                    "activity.logs.updated",
                    "doctor.updated",
                    "switch.completed",
                    "switch.failed",
                    "task.updated",
                    "health.updated"
                ]),
            as: RPCResponseAck.self)
        initializedState = initial.initialState
        return initial.initialState
    }

    func stop() async {
        guard let process else {
            return
        }

        isStopping = true
        process.terminationHandler = nil
        _ = try? await sendRequest(method: "shutdown", params: EmptyParams(), as: RPCResponseAck.self)
        terminateProcess()
        cleanupProcessState()
        startTask = nil
        isStopping = false
    }

    func restart() async throws -> RPCInitialState {
        await stop()
        return try await start()
    }

    func fetchDoctor() async throws -> DoctorReport {
        try await request(method: "relay/doctor/get", as: DoctorReport.self)
    }

    func fetchStatus() async throws -> StatusReport {
        try await request(method: "relay/status/get", as: StatusReport.self)
    }

    func refreshUsage(profileId: String) async throws -> UsageSnapshot {
        let result = try await request(
            method: "relay/usage/refresh",
            params: RefreshUsageRPCParams(profileId: profileId, includeDisabled: false),
            as: RPCUsageRefreshResult.self)
        guard let snapshot = result.snapshots.first else {
            throw RelayClientError.invalidResponse("missing snapshot in refresh result")
        }
        return snapshot
    }

    func refreshEnabledUsage() async throws -> [UsageSnapshot] {
        let result = try await request(
            method: "relay/usage/refresh",
            params: RefreshUsageRPCParams(profileId: nil, includeDisabled: false),
            as: RPCUsageRefreshResult.self)
        return result.snapshots
    }

    func refreshAllUsage() async throws -> [UsageSnapshot] {
        let result = try await request(
            method: "relay/usage/refresh",
            params: RefreshUsageRPCParams(profileId: nil, includeDisabled: true),
            as: RPCUsageRefreshResult.self)
        return result.snapshots
    }

    func refreshActivity() async throws -> RPCActivityRefreshResult {
        try await request(method: "relay/activity/refresh", as: RPCActivityRefreshResult.self)
    }

    func refreshDoctor() async throws -> DoctorReport {
        try await request(method: "relay/doctor/refresh", as: DoctorReport.self)
    }

    func fetchCodexSettings() async throws -> CodexSettings {
        try await request(method: "relay/settings/get", as: RPCSettingsResult.self).codex
    }

    func setCodexSettings(_ draft: CodexSettingsDraft) async throws -> CodexSettings {
        let result = try await request(
            method: "relay/settings/update",
            params: RPCSettingsUpdatePayload(app: nil, codex: draft),
            as: RPCSettingsResult.self)
        return result.codex
    }

    func switchToProfile(_ profileId: String) async throws -> SwitchReport {
        try await request(
            method: "relay/switch/activate",
            params: RPCProfileID(profileId: profileId),
            as: SwitchReport.self)
    }

    func switchToNextProfile() async throws -> SwitchReport {
        try await request(method: "relay/switch/next", as: SwitchReport.self)
    }

    func editProfile(profileId: String, draft: ProfileDraft) async throws -> Profile {
        try await request(
            method: "relay/profiles/edit",
            params: RPCEditProfileParams(profileId: profileId, draft: draft),
            as: Profile.self)
    }

    func removeProfile(profileId: String) async throws -> Profile {
        try await request(
            method: "relay/profiles/remove",
            params: RPCProfileID(profileId: profileId),
            as: Profile.self)
    }

    func importProfile(agent: AgentKind, nickname: String?, priority: Int) async throws -> Profile {
        try await request(
            method: "relay/profiles/import",
            params: RPCImportProfileParams(agent: agent, nickname: nickname, priority: priority),
            as: Profile.self)
    }

    func startLoginProfile(
        agent: AgentKind,
        nickname: String?,
        priority: Int) async throws -> TaskStartResult
    {
        try await request(
            method: "relay/profiles/login/start",
            params: RPCLoginProfileParams(
                agent: agent,
                nickname: nickname,
                priority: priority,
                mode: .browser),
            as: TaskStartResult.self)
    }

    func cancelTask(taskId: String) async throws -> TaskCancelResult {
        try await request(
            method: "relay/tasks/cancel",
            params: RPCTaskID(taskId: taskId),
            as: TaskCancelResult.self)
    }

    func setProfileEnabled(profileId: String, enabled: Bool) async throws -> Profile {
        try await request(
            method: "relay/profiles/set_enabled",
            params: RPCSetProfileEnabledParams(profileId: profileId, enabled: enabled),
            as: Profile.self)
    }

    func setAutoSwitch(enabled: Bool) async throws -> AppSettings {
        let result = try await request(
            method: "relay/settings/update",
            params: RPCSettingsUpdatePayload(
                app: AppSettingsPatch(
                    autoSwitchEnabled: enabled,
                    cooldownSeconds: nil,
                    refreshIntervalSeconds: nil,
                    networkQueryConcurrency: nil),
                codex: nil),
            as: RPCSettingsResult.self)
        return result.app
    }

    func setRefreshInterval(seconds: Int) async throws -> AppSettings {
        let result = try await request(
            method: "relay/settings/update",
            params: RPCSettingsUpdatePayload(
                app: AppSettingsPatch(
                    autoSwitchEnabled: nil,
                    cooldownSeconds: nil,
                    refreshIntervalSeconds: seconds,
                    networkQueryConcurrency: nil),
                codex: nil),
            as: RPCSettingsResult.self)
        return result.app
    }

    func setNetworkQueryConcurrency(value: Int) async throws -> AppSettings {
        let result = try await request(
            method: "relay/settings/update",
            params: RPCSettingsUpdatePayload(
                app: AppSettingsPatch(
                    autoSwitchEnabled: nil,
                    cooldownSeconds: nil,
                    refreshIntervalSeconds: nil,
                    networkQueryConcurrency: value),
                codex: nil),
            as: RPCSettingsResult.self)
        return result.app
    }

    func exportDiagnostics() async throws -> DiagnosticsExport {
        try await request(method: "relay/activity/diagnostics/export", as: DiagnosticsExport.self)
    }

    private func request<Response: Decodable & Sendable>(
        method: String,
        as type: Response.Type) async throws -> Response
    {
        try await request(method: method, params: EmptyParams(), as: type)
    }

    private func request<Response: Decodable & Sendable>(
        method: String,
        params: some Encodable & Sendable,
        as type: Response.Type) async throws -> Response
    {
        _ = try await start()
        return try await sendRequest(method: method, params: params, as: type)
    }

    private func sendRequest<Response: Decodable & Sendable>(
        method: String,
        params: some Encodable & Sendable,
        as type: Response.Type) async throws -> Response
    {
        guard let stdinPipe else {
            throw RelayClientError.relayNotFound([])
        }

        let requestID = nextID()
        let encoder = JSONEncoder.relayEncoder
        let payload = try encoder.encode(
            RPCRequestEnvelope(id: requestID, method: method, params: params))

        let line = payload + Data([0x0A])
        let responseData = try await withCheckedThrowingContinuation { continuation in
            pending[requestID] = continuation
            scheduleTimeout(for: requestID)
            stdinPipe.fileHandleForWriting.write(line)
        }

        let decoder = JSONDecoder.relayDecoder
        if let errorEnvelope = try? decoder.decode(RPCErrorEnvelope.self, from: responseData) {
            throw RelayClientError.commandFailed(
                code: errorEnvelope.error.data?.relayErrorCode,
                message: errorEnvelope.error.message)
        }

        let envelope = try decoder.decode(RPCResponseEnvelope<Response>.self, from: responseData)
        return envelope.result
    }

    private func nextID() -> String {
        nextRequestID += 1
        return "rpc-\(nextRequestID)"
    }

    private func consumeStdout(_ data: Data) async {
        guard !data.isEmpty else {
            return
        }

        stdoutBuffer.append(data)
        while let newlineIndex = stdoutBuffer.firstIndex(of: 0x0A) {
            let line = stdoutBuffer.prefix(upTo: newlineIndex)
            stdoutBuffer.removeSubrange(...newlineIndex)
            guard !line.isEmpty else {
                continue
            }
            await handleLine(Data(line))
        }
    }

    private func handleLine(_ data: Data) async {
        guard
            let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else {
            return
        }

        if let method = object["method"] as? String, method == "session/update" {
            if let update = decodeNotification(from: data) {
                continuation.yield(update)
            }
            return
        }

        guard let id = object["id"] as? String, let continuation = pending.removeValue(forKey: id) else {
            return
        }
        pendingTimeouts.removeValue(forKey: id)?.cancel()
        continuation.resume(returning: data)
    }

    private func decodeNotification(from data: Data) -> RelaySessionUpdate? {
        guard
            let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let params = object["params"] as? [String: Any],
            let topic = params["topic"] as? String,
            let payload = params["payload"]
        else {
            return nil
        }

        guard let payloadData = try? JSONSerialization.data(withJSONObject: payload) else {
            return nil
        }

        switch topic {
        case "usage.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: UsageUpdatedNotification.self).map(RelaySessionUpdate.usageUpdated)
        case "query_state.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: QueryStateUpdatedNotification.self).map(RelaySessionUpdate.queryStateUpdated)
        case "active_state.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: ActiveStateUpdatedNotification.self).map(RelaySessionUpdate.activeStateUpdated)
        case "settings.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: SettingsUpdatedNotification.self).map(RelaySessionUpdate.settingsUpdated)
        case "profiles.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: ProfilesUpdatedNotification.self).map(RelaySessionUpdate.profilesUpdated)
        case "activity.events.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: ActivityEventsUpdatedNotification.self).map(RelaySessionUpdate.activityEventsUpdated)
        case "activity.logs.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: ActivityLogsUpdatedNotification.self).map(RelaySessionUpdate.activityLogsUpdated)
        case "doctor.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: DoctorUpdatedNotification.self).map(RelaySessionUpdate.doctorUpdated)
        case "switch.completed":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: SwitchCompletedNotification.self).map(RelaySessionUpdate.switchCompleted)
        case "switch.failed":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: SwitchFailedNotification.self).map(RelaySessionUpdate.switchFailed)
        case "task.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: TaskUpdatedNotification.self).map(RelaySessionUpdate.taskUpdated)
        case "health.updated":
            return decodeNotificationPayload(
                topic: topic,
                payloadData: payloadData,
                as: HealthUpdatedNotification.self).map(RelaySessionUpdate.healthUpdated)
        default:
            return nil
        }
    }

    private func decodeNotificationPayload<Payload: Decodable>(
        topic: String,
        payloadData: Data,
        as type: Payload.Type) -> Payload?
    {
        do {
            return try JSONDecoder.relayDecoder.decode(type, from: payloadData)
        } catch {
            let payload = String(data: payloadData, encoding: .utf8) ?? "<non-utf8 payload>"
            fputs(
                "RelayDaemonClient failed to decode \(topic) notification: \(error)\npayload: \(payload)\n",
                stderr)
            return nil
        }
    }

    private func scheduleTimeout(for requestID: String) {
        pendingTimeouts.removeValue(forKey: requestID)?.cancel()
        let timeoutSeconds = requestTimeoutSeconds
        let timeoutDuration = Duration.seconds(timeoutSeconds)
        pendingTimeouts[requestID] = Task { [weak self] in
            do {
                try await Task.sleep(for: timeoutDuration)
            } catch {
                return
            }
            await self?.failPendingRequestTimeout(id: requestID, timeoutSeconds: timeoutSeconds)
        }
    }

    private func failPendingRequestTimeout(id: String, timeoutSeconds: TimeInterval) {
        guard let continuation = pending.removeValue(forKey: id) else {
            return
        }
        pendingTimeouts.removeValue(forKey: id)?.cancel()
        continuation.resume(throwing: RelayClientError.commandFailed(
            code: "RELAY_DAEMON_TIMEOUT",
            message: "daemon request timed out after \(Self.formatTimeout(timeoutSeconds)) seconds"))
    }

    private func cancelPendingTimeouts() {
        for task in pendingTimeouts.values {
            task.cancel()
        }
        pendingTimeouts.removeAll()
    }

    private func handleTermination(_ process: Process) {
        cancelPendingTimeouts()
        if isStopping {
            cleanupProcessState()
            return
        }
        let error = RelayClientError.commandFailed(
            code: "RELAY_DAEMON_EXITED",
            message: "daemon exited with status \(process.terminationStatus)")
        for continuation in pending.values {
            continuation.resume(throwing: error)
        }
        pending.removeAll()
        continuation.yield(.healthUpdated(
            HealthUpdatedNotification(
                state: .degraded,
                detail: "AgentRelay engine exited.")))
        cleanupProcessState()
    }

    private func cleanupProcessState() {
        cancelPendingTimeouts()
        process = nil
        stdinPipe = nil
        stdoutBuffer.removeAll(keepingCapacity: false)
        initializedState = nil
        startTask = nil
    }

    private func terminateProcess() {
        guard let process else {
            return
        }

        if process.isRunning {
            process.terminate()
        }

        let pid = process.processIdentifier
        guard pid > 0 else {
            return
        }

        Task(priority: .userInitiated) {
            try? await Task.sleep(for: .milliseconds(200))
            if process.isRunning {
                kill(pid, SIGKILL)
            }
        }
    }

    private func resolvedRelayCLIPath() throws -> String {
        if let override = relayCLIPathOverride ?? environment["AGRELAY_CLI_PATH"], !override.isEmpty {
            return override
        }

        for candidate in bundledRelayCandidates() where FileManager.default.isExecutableFile(atPath: candidate) {
            return candidate
        }

        if let pathBinary = findRelayOnPATH() {
            return pathBinary
        }

        throw RelayClientError.relayNotFound(bundledRelayCandidates())
    }

    private func findRelayOnPATH() -> String? {
        guard let path = environment["PATH"], !path.isEmpty else {
            return nil
        }

        for directory in path.split(separator: ":") {
            let candidate = String(directory) + "/agrelay"
            if FileManager.default.isExecutableFile(atPath: candidate) {
                return candidate
            }
        }

        return nil
    }

    private func bundledRelayCandidates() -> [String] {
        var candidates: [String] = []

        if let builtInPlugInsPath = Bundle.main.builtInPlugInsPath {
            candidates.append((builtInPlugInsPath as NSString).appendingPathComponent("agrelay"))
        }

        if let sharedSupportPath = Bundle.main.sharedSupportPath {
            candidates.append((sharedSupportPath as NSString).appendingPathComponent("agrelay"))
        }

        if let resourcePath = Bundle.main.path(forResource: "agrelay", ofType: nil, inDirectory: "bin") {
            candidates.append(resourcePath)
        }

        if let resourceURL = Bundle.main.resourceURL {
            candidates.append(
                resourceURL
                    .appending(path: "bin", directoryHint: .isDirectory)
                    .appending(path: "agrelay")
                    .path(percentEncoded: false))
        }

        if let executableURL = Bundle.main.executableURL {
            let executableDir = executableURL.deletingLastPathComponent()
            let contentsDir = executableDir.deletingLastPathComponent()
            candidates.append(
                contentsDir
                    .appending(path: "Helpers", directoryHint: .isDirectory)
                    .appending(path: "agrelay")
                    .path(percentEncoded: false))
            candidates.append(
                contentsDir
                    .appending(path: "Resources", directoryHint: .isDirectory)
                    .appending(path: "bin", directoryHint: .isDirectory)
                    .appending(path: "agrelay")
                    .path(percentEncoded: false))
        }

        if Bundle.main.bundleURL.pathExtension == "app" {
            candidates.append(
                Bundle.main.bundleURL
                    .appending(path: "Contents", directoryHint: .isDirectory)
                    .appending(path: "Helpers", directoryHint: .isDirectory)
                    .appending(path: "agrelay")
                    .path(percentEncoded: false))
            candidates.append(
                Bundle.main.bundleURL
                    .appending(path: "Contents", directoryHint: .isDirectory)
                    .appending(path: "Resources", directoryHint: .isDirectory)
                    .appending(path: "bin", directoryHint: .isDirectory)
                    .appending(path: "agrelay")
                    .path(percentEncoded: false))
        }

        var seen = Set<String>()
        return candidates.filter { seen.insert($0).inserted }
    }

    private static func resolveRequestTimeoutSeconds(
        override: TimeInterval?,
        environment: [String: String]) -> TimeInterval
    {
        if let override, override.isFinite {
            return max(override, minimumRequestTimeoutSeconds)
        }
        if
            let raw = environment["RELAY_DAEMON_REQUEST_TIMEOUT_SECONDS"],
            let parsed = TimeInterval(raw),
            parsed.isFinite
        {
            return max(parsed, minimumRequestTimeoutSeconds)
        }
        return defaultRequestTimeoutSeconds
    }

    private static func formatTimeout(_ seconds: TimeInterval) -> String {
        if seconds.rounded() == seconds {
            return String(Int(seconds))
        }
        return seconds.formatted(.number.precision(.fractionLength(1)))
    }
}

private struct EmptyParams: Encodable {}

private struct RPCResponseAck: Decodable {
    let accepted: Bool?
}

private struct RPCProfileID: Encodable {
    let profileId: String
}

private struct UsageGetParams: Encodable {
    let profileId: String?
}

private struct RefreshUsageRPCParams: Encodable {
    let profileId: String?
    let includeDisabled: Bool
}

private struct RPCSetProfileEnabledParams: Encodable {
    let profileId: String
    let enabled: Bool
}

private struct RPCImportProfileParams: Encodable {
    let request: RPCImportProfileRequest

    init(agent: AgentKind, nickname: String?, priority: Int) {
        request = RPCImportProfileRequest(agent: agent, nickname: nickname, priority: priority)
    }
}

private struct RPCImportProfileRequest: Encodable {
    let agent: AgentKind
    let nickname: String?
    let priority: Int
}

private struct RPCLoginProfileParams: Encodable {
    let request: RPCLoginProfileRequest

    init(agent: AgentKind, nickname: String?, priority: Int, mode: RPCLoginMode) {
        request = RPCLoginProfileRequest(
            agent: agent,
            nickname: nickname,
            priority: priority,
            mode: mode)
    }
}

private struct RPCLoginProfileRequest: Encodable {
    let agent: AgentKind
    let nickname: String?
    let priority: Int
    let mode: RPCLoginMode
}

private enum RPCLoginMode: String, Encodable {
    case browser = "Browser"
    case deviceAuth = "DeviceAuth"
}

private struct RPCEditProfileParams: Encodable {
    let profileId: String
    let request: RPCEditProfileRequest

    init(profileId: String, draft: ProfileDraft) {
        self.profileId = profileId
        request = RPCEditProfileRequest(draft: draft)
    }
}

private struct RPCEditProfileRequest: Encodable {
    let nickname: String?
    let priority: Int?
    let configPath: String??
    let agentHome: String??
    let authMode: AuthMode?

    init(draft: ProfileDraft) {
        nickname = draft.nickname
        priority = draft.priority
        configPath = draft.clearConfigPath ? .some(nil) : .some(draft.configPath.isEmpty ? nil : draft.configPath)
        agentHome = draft.clearAgentHome ? .some(nil) : .some(draft.agentHome.isEmpty ? nil : draft.agentHome)
        authMode = draft.authMode
    }
}
