import Darwin
import Foundation

actor RelayDaemonClient {
    private let relayCLIPathOverride: String?
    private let environment: [String: String]
    private var process: Process?
    private var stdinPipe: Pipe?
    private var stdoutBuffer = Data()
    private var nextRequestID = 0
    private var pending: [String: CheckedContinuation<Data, Error>] = [:]
    private var initializedState: RPCInitialState?
    private var startTask: Task<RPCInitialState, Error>?
    private var isStopping = false
    private let stream: AsyncStream<RelaySessionUpdate>
    private let continuation: AsyncStream<RelaySessionUpdate>.Continuation

    init(
        relayCLIPathOverride: String? = nil,
        environment: [String: String] = ProcessInfo.processInfo.environment
    ) {
        self.relayCLIPathOverride = relayCLIPathOverride
        self.environment = environment
        let streamPair = AsyncStream.makeStream(of: RelaySessionUpdate.self)
        self.stream = streamPair.stream
        self.continuation = streamPair.continuation
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
            throw RelayCLIClientError.launchFailed(error.localizedDescription)
        }

        self.process = process
        self.stdinPipe = stdin

        let initial: RPCInitializeResult = try await sendRequest(
            method: "initialize",
            params: RPCInitializeParams(),
            as: RPCInitializeResult.self
        )
        _ = try await sendRequest(
            method: "session/subscribe",
            params: RPCSubscribeParams(
                topics: [
                    "usage.updated",
                    "active_state.updated",
                    "switch.completed",
                    "switch.failed",
                    "health.updated",
                ]
            ),
            as: RPCResponseAck.self
        )
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

    func fetchProfileList() async throws -> [ProfileListItem] {
        try await request(method: "relay/profiles/list", as: [ProfileListItem].self)
    }

    func fetchCurrentUsage() async throws -> UsageSnapshot {
        let result = try await request(
            method: "relay/usage/get",
            params: UsageGetParams(profileId: nil),
            as: UsageResult.self
        )
        return result.snapshot
    }

    func fetchUsage(profileId: String) async throws -> UsageSnapshot {
        let result = try await request(
            method: "relay/usage/get",
            params: UsageGetParams(profileId: profileId),
            as: UsageResult.self
        )
        return result.snapshot
    }

    func refreshUsage(profileId: String) async throws -> UsageSnapshot {
        let result = try await request(
            method: "relay/usage/refresh",
            params: RefreshUsageRPCParams(profileId: profileId, includeDisabled: false),
            as: RPCUsageRefreshResult.self
        )
        guard let snapshot = result.snapshots.first else {
            throw RelayCLIClientError.invalidResponse("missing snapshot in refresh result")
        }
        return snapshot
    }

    func refreshEnabledUsage() async throws -> [UsageSnapshot] {
        let result = try await request(
            method: "relay/usage/refresh",
            params: RefreshUsageRPCParams(profileId: nil, includeDisabled: false),
            as: RPCUsageRefreshResult.self
        )
        return result.snapshots
    }

    func fetchCodexSettings() async throws -> CodexSettings {
        try await request(method: "relay/settings/get", as: RPCSettingsResult.self).codex
    }

    func setCodexSettings(_ draft: CodexSettingsDraft) async throws -> CodexSettings {
        let result = try await request(
            method: "relay/settings/update",
            params: RPCSettingsUpdatePayload(app: nil, codex: draft),
            as: RPCSettingsResult.self
        )
        return result.codex
    }

    func switchToProfile(_ profileId: String) async throws -> SwitchReport {
        try await request(
            method: "relay/switch/activate",
            params: RPCProfileID(profileId: profileId),
            as: SwitchReport.self
        )
    }

    func switchToNextProfile() async throws -> SwitchReport {
        try await request(method: "relay/switch/next", as: SwitchReport.self)
    }

    func editProfile(profileId: String, draft: ProfileDraft) async throws -> Profile {
        try await request(
            method: "relay/profiles/edit",
            params: RPCEditProfileParams(profileId: profileId, draft: draft),
            as: Profile.self
        )
    }

    func removeProfile(profileId: String) async throws -> Profile {
        try await request(
            method: "relay/profiles/remove",
            params: RPCProfileID(profileId: profileId),
            as: Profile.self
        )
    }

    func importProfile(agent: AgentKind, nickname: String?, priority: Int) async throws -> Profile {
        try await request(
            method: "relay/profiles/import",
            params: RPCImportProfileParams(agent: agent, nickname: nickname, priority: priority),
            as: Profile.self
        )
    }

    func setProfileEnabled(profileId: String, enabled: Bool) async throws -> Profile {
        try await request(
            method: "relay/profiles/set_enabled",
            params: RPCSetProfileEnabledParams(profileId: profileId, enabled: enabled),
            as: Profile.self
        )
    }

    func setAutoSwitch(enabled: Bool) async throws -> AppSettings {
        let result = try await request(
            method: "relay/settings/update",
            params: RPCSettingsUpdatePayload(
                app: AppSettingsPatch(
                    autoSwitchEnabled: enabled,
                    cooldownSeconds: nil,
                    refreshIntervalSeconds: nil
                ),
                codex: nil
            ),
            as: RPCSettingsResult.self
        )
        return result.app
    }

    func setRefreshInterval(seconds: Int) async throws -> AppSettings {
        let result = try await request(
            method: "relay/settings/update",
            params: RPCSettingsUpdatePayload(
                app: AppSettingsPatch(
                    autoSwitchEnabled: nil,
                    cooldownSeconds: nil,
                    refreshIntervalSeconds: seconds
                ),
                codex: nil
            ),
            as: RPCSettingsResult.self
        )
        return result.app
    }

    func fetchEvents(limit: Int) async throws -> [FailureEvent] {
        let result = try await request(
            method: "relay/activity/events/list",
            params: EventsListPayload(limit: limit),
            as: RPCEventsResult.self
        )
        return result.events
    }

    func fetchLogs(lines: Int) async throws -> LogTail {
        let result = try await request(
            method: "relay/activity/logs/tail",
            params: LogsTailPayload(lines: lines),
            as: RPCLogsResult.self
        )
        return result.logs
    }

    func exportDiagnostics() async throws -> DiagnosticsExport {
        try await request(method: "relay/activity/diagnostics/export", as: DiagnosticsExport.self)
    }

    private func request<Response: Decodable & Sendable>(
        method: String,
        as type: Response.Type
    ) async throws -> Response {
        try await request(method: method, params: EmptyParams(), as: type)
    }

    private func request<Params: Encodable & Sendable, Response: Decodable & Sendable>(
        method: String,
        params: Params,
        as type: Response.Type
    ) async throws -> Response {
        _ = try await start()
        return try await sendRequest(method: method, params: params, as: type)
    }

    private func sendRequest<Params: Encodable & Sendable, Response: Decodable & Sendable>(
        method: String,
        params: Params,
        as type: Response.Type
    ) async throws -> Response {
        guard let stdinPipe else {
            throw RelayCLIClientError.relayNotFound([])
        }

        let requestID = nextID()
        let encoder = JSONEncoder.relayEncoder
        let payload = try encoder.encode(
            RPCRequestEnvelope(id: requestID, method: method, params: params)
        )

        let line = payload + Data([0x0A])
        let responseData = try await withCheckedThrowingContinuation { continuation in
            pending[requestID] = continuation
            stdinPipe.fileHandleForWriting.write(line)
        }

        let decoder = JSONDecoder.relayDecoder
        if let errorEnvelope = try? decoder.decode(RPCErrorEnvelope.self, from: responseData) {
            throw RelayCLIClientError.commandFailed(
                code: errorEnvelope.error.data?.relayErrorCode,
                message: errorEnvelope.error.message
            )
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

        let decoder = JSONDecoder.relayDecoder
        switch topic {
        case "usage.updated":
            return try? .usageUpdated(decoder.decode(UsageUpdatedNotification.self, from: payloadData))
        case "active_state.updated":
            return try? .activeStateUpdated(decoder.decode(ActiveStateUpdatedNotification.self, from: payloadData))
        case "switch.completed":
            return try? .switchCompleted(decoder.decode(SwitchCompletedNotification.self, from: payloadData))
        case "switch.failed":
            return try? .switchFailed(decoder.decode(SwitchFailedNotification.self, from: payloadData))
        case "health.updated":
            return try? .healthUpdated(decoder.decode(HealthUpdatedNotification.self, from: payloadData))
        default:
            return nil
        }
    }

    private func handleTermination(_ process: Process) {
        if isStopping {
            cleanupProcessState()
            return
        }
        let error = RelayCLIClientError.commandFailed(
            code: "RELAY_DAEMON_EXITED",
            message: "relay daemon exited with status \(process.terminationStatus)"
        )
        for continuation in pending.values {
            continuation.resume(throwing: error)
        }
        pending.removeAll()
        continuation.yield(.healthUpdated(
            HealthUpdatedNotification(
                state: .degraded,
                detail: "Relay engine exited."
            )
        ))
        cleanupProcessState()
    }

    private func cleanupProcessState() {
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

        DispatchQueue.global(qos: .userInitiated).asyncAfter(deadline: .now() + 0.2) {
            if process.isRunning {
                kill(pid, SIGKILL)
            }
        }
    }

    private func resolvedRelayCLIPath() throws -> String {
        if let override = relayCLIPathOverride ?? environment["RELAY_CLI_PATH"], !override.isEmpty {
            return override
        }

        for candidate in bundledRelayCandidates() where FileManager.default.isExecutableFile(atPath: candidate) {
            return candidate
        }

        if let pathBinary = findRelayOnPATH() {
            return pathBinary
        }

        throw RelayCLIClientError.relayNotFound(bundledRelayCandidates())
    }

    private func findRelayOnPATH() -> String? {
        guard let path = environment["PATH"], !path.isEmpty else {
            return nil
        }

        for directory in path.split(separator: ":") {
            let candidate = String(directory) + "/relay"
            if FileManager.default.isExecutableFile(atPath: candidate) {
                return candidate
            }
        }

        return nil
    }

    private func bundledRelayCandidates() -> [String] {
        var candidates: [String] = []

        if let resourcePath = Bundle.main.path(forResource: "relay", ofType: nil, inDirectory: "bin") {
            candidates.append(resourcePath)
        }

        if let resourceURL = Bundle.main.resourceURL {
            candidates.append(
                resourceURL
                    .appending(path: "bin", directoryHint: .isDirectory)
                    .appending(path: "relay")
                    .path(percentEncoded: false)
            )
        }

        if let executableURL = Bundle.main.executableURL {
            let executableDir = executableURL.deletingLastPathComponent()
            let contentsDir = executableDir.deletingLastPathComponent()
            candidates.append(
                contentsDir
                    .appending(path: "Resources", directoryHint: .isDirectory)
                    .appending(path: "bin", directoryHint: .isDirectory)
                    .appending(path: "relay")
                    .path(percentEncoded: false)
            )
        }

        if Bundle.main.bundleURL.pathExtension == "app" {
            candidates.append(
                Bundle.main.bundleURL
                    .appending(path: "Contents", directoryHint: .isDirectory)
                    .appending(path: "Resources", directoryHint: .isDirectory)
                    .appending(path: "bin", directoryHint: .isDirectory)
                    .appending(path: "relay")
                    .path(percentEncoded: false)
            )
        }

        var seen = Set<String>()
        return candidates.filter { seen.insert($0).inserted }
    }
}

private struct EmptyParams: Encodable, Sendable {}

private struct RPCResponseAck: Decodable, Sendable {
    let accepted: Bool?
}

private struct RPCProfileID: Encodable, Sendable {
    let profileId: String
}

private struct UsageGetParams: Encodable, Sendable {
    let profileId: String?
}

private struct RefreshUsageRPCParams: Encodable, Sendable {
    let profileId: String?
    let includeDisabled: Bool
}

private struct RPCSetProfileEnabledParams: Encodable, Sendable {
    let profileId: String
    let enabled: Bool
}

private struct RPCImportProfileParams: Encodable, Sendable {
    let request: RPCImportProfileRequest

    init(agent: AgentKind, nickname: String?, priority: Int) {
        request = RPCImportProfileRequest(agent: agent, nickname: nickname, priority: priority)
    }
}

private struct RPCImportProfileRequest: Encodable, Sendable {
    let agent: AgentKind
    let nickname: String?
    let priority: Int
}

private struct RPCEditProfileParams: Encodable, Sendable {
    let profileId: String
    let request: RPCEditProfileRequest

    init(profileId: String, draft: ProfileDraft) {
        self.profileId = profileId
        self.request = RPCEditProfileRequest(draft: draft)
    }
}

private struct RPCEditProfileRequest: Encodable, Sendable {
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
