import Foundation

struct RelayCLIClient {
    private let relayCLIPathOverride: String?
    private let environment: [String: String]

    init(
        relayCLIPathOverride: String? = nil,
        environment: [String: String] = ProcessInfo.processInfo.environment
    ) {
        self.relayCLIPathOverride = relayCLIPathOverride
        self.environment = environment
    }

    func fetchDoctor() async throws -> DoctorReport {
        try await run(["doctor"], as: DoctorReport.self)
    }

    func fetchStatus() async throws -> StatusReport {
        try await run(["status"], as: StatusReport.self)
    }

    func fetchUsage() async throws -> UsageSnapshot {
        try await run(["usage"], as: UsageSnapshot.self)
    }

    func fetchUsage(profileId: String) async throws -> UsageSnapshot {
        try await run(
            ["usage", "profile", "--input-json", "-"],
            input: ProfileIdPayload(id: profileId),
            as: UsageSnapshot.self
        )
    }

    func fetchUsageList() async throws -> [UsageSnapshot] {
        try await run(["usage", "list"], as: [UsageSnapshot].self)
    }

    func refreshUsage(profileId: String) async throws -> UsageSnapshot {
        try await run(
            ["usage", "refresh", "--input-json", "-"],
            input: UsageRefreshPayload(id: profileId, enabled: false, all: false),
            as: UsageSnapshot.self
        )
    }

    func refreshEnabledUsage() async throws -> [UsageSnapshot] {
        try await run(
            ["usage", "refresh", "--input-json", "-"],
            input: UsageRefreshPayload(id: nil, enabled: true, all: false),
            as: [UsageSnapshot].self
        )
    }

    func refreshAllUsage() async throws -> [UsageSnapshot] {
        try await run(
            ["usage", "refresh", "--input-json", "-"],
            input: UsageRefreshPayload(id: nil, enabled: false, all: true),
            as: [UsageSnapshot].self
        )
    }

    func setUsageSettings(_ draft: UsageSettingsDraft) async throws -> AppSettings {
        try await run(
            ["usage", "config", "set", "--input-json", "-"],
            input: draft,
            as: AppSettings.self
        )
    }

    func fetchProfiles() async throws -> [Profile] {
        try await run(["profiles", "list"], as: [Profile].self)
    }

    func editProfile(profileId: String, draft: ProfileDraft) async throws -> Profile {
        let payload = EditProfilePayload(profileId: profileId, draft: draft)
        return try await run(
            ["profiles", "edit", "--input-json", "-"],
            input: payload,
            as: Profile.self
        )
    }

    func removeProfile(profileId: String) async throws -> Profile {
        try await run(
            ["profiles", "remove", "--input-json", "-"],
            input: ProfileIdPayload(id: profileId),
            as: Profile.self
        )
    }

    func importCodexProfile(nickname: String?, priority: Int) async throws -> Profile {
        try await run(
            ["profiles", "import", "--input-json", "-"],
            input: ImportProfilePayload(nickname: nickname, priority: priority),
            as: Profile.self
        )
    }

    func loginCodexProfile(nickname: String?, priority: Int) async throws -> CodexLinkResult {
        try await run(
            ["profiles", "login", "--input-json", "-"],
            input: LoginProfilePayload(nickname: nickname, priority: priority),
            as: CodexLinkResult.self
        )
    }

    func switchToProfile(_ profileId: String) async throws -> SwitchReport {
        try await run(
            ["switch", "--input-json", "-"],
            input: SwitchPayload(target: profileId),
            as: SwitchReport.self
        )
    }

    func setAutoSwitch(enabled: Bool) async throws -> AppSettings {
        try await run(
            ["auto-switch", "set", "--input-json", "-"],
            input: AutoSwitchPayload(enabled: enabled),
            as: AppSettings.self
        )
    }

    func setProfileEnabled(profileId: String, enabled: Bool) async throws -> Profile {
        try await run(
            ["profiles", enabled ? "enable" : "disable", "--input-json", "-"],
            input: ProfileIdPayload(id: profileId),
            as: Profile.self
        )
    }

    func fetchEvents(limit: Int) async throws -> [FailureEvent] {
        try await run(
            ["events", "list", "--input-json", "-"],
            input: EventsListPayload(limit: limit),
            as: [FailureEvent].self
        )
    }

    func fetchLogs(lines: Int) async throws -> LogTail {
        try await run(
            ["logs", "tail", "--input-json", "-"],
            input: LogsTailPayload(lines: lines),
            as: LogTail.self
        )
    }

    func exportDiagnostics() async throws -> DiagnosticsExport {
        try await run(["diagnostics", "export"], as: DiagnosticsExport.self)
    }

    private func run<Response: Decodable & Sendable>(
        _ arguments: [String],
        as type: Response.Type
    ) async throws -> Response {
        try await run(arguments, inputData: nil, as: type)
    }

    private func run<Input: Encodable, Response: Decodable & Sendable>(
        _ arguments: [String],
        input: Input,
        as type: Response.Type
    ) async throws -> Response {
        let encoder = JSONEncoder.relayEncoder
        let data = try encoder.encode(input)
        return try await run(arguments, inputData: data, as: type)
    }

    private func run<Response: Decodable & Sendable>(
        _ arguments: [String],
        inputData: Data?,
        as type: Response.Type
    ) async throws -> Response {
        let command = try resolvedRelayCLIPath()
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                do {
                    let response = try runRelayProcess(
                        command: command,
                        arguments: arguments,
                        inputData: inputData,
                        environment: environment,
                        as: type
                    )
                    continuation.resume(returning: response)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    private func resolvedRelayCLIPath() throws -> String {
        if let override = relayCLIPathOverride ?? environment["RELAY_CLI_PATH"], !override.isEmpty {
            return override
        }

        for candidate in bundledRelayCandidates() {
            if FileManager.default.isExecutableFile(atPath: candidate) {
                return candidate
            }
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

private func runRelayProcess<Response: Decodable & Sendable>(
    command: String,
    arguments: [String],
    inputData: Data?,
    environment: [String: String],
    as type: Response.Type
) throws -> Response {
    let process = Process()
    let stdout = Pipe()
    let stderr = Pipe()
    let stdoutBuffer = LockedDataBuffer()
    let stderrBuffer = LockedDataBuffer()
    let stdoutDone = DispatchSemaphore(value: 0)
    let stderrDone = DispatchSemaphore(value: 0)

    process.executableURL = URL(fileURLWithPath: command)
    process.arguments = ["--json"] + arguments
    process.standardOutput = stdout
    process.standardError = stderr
    if inputData != nil {
        process.standardInput = Pipe()
    }
    process.environment = environment

    stdout.fileHandleForReading.readabilityHandler = { handle in
        let data = handle.availableData
        if data.isEmpty {
            handle.readabilityHandler = nil
            stdoutDone.signal()
            return
        }
        stdoutBuffer.append(data)
    }

    stderr.fileHandleForReading.readabilityHandler = { handle in
        let data = handle.availableData
        if data.isEmpty {
            handle.readabilityHandler = nil
            stderrDone.signal()
            return
        }
        stderrBuffer.append(data)
    }

    do {
        try process.run()
    } catch {
        throw RelayCLIClientError.launchFailed(error.localizedDescription)
    }

    if let inputData, let stdin = process.standardInput as? Pipe {
        stdin.fileHandleForWriting.write(inputData)
        try? stdin.fileHandleForWriting.close()
    }

    process.waitUntilExit()
    stdoutDone.wait()
    stderrDone.wait()

    let output = stdoutBuffer.snapshot()
    let errorOutput = stderrBuffer.snapshot()
    let decoder = JSONDecoder.relayDecoder

    if output.isEmpty {
        let stderrText = String(decoding: errorOutput, as: UTF8.self)
        throw RelayCLIClientError.emptyOutput(stderrText)
    }

    let envelope: RelayEnvelope<Response>
    do {
        envelope = try decoder.decode(RelayEnvelope<Response>.self, from: output)
    } catch {
        throw RelayCLIClientError.decodeFailed(error.localizedDescription)
    }

    guard envelope.success else {
        throw RelayCLIClientError.commandFailed(
            code: envelope.errorCode,
            message: envelope.message
        )
    }

    guard let data = envelope.data else {
        throw RelayCLIClientError.emptyOutput("Relay returned no data payload.")
    }

    return data
}

private final class LockedDataBuffer: @unchecked Sendable {
    private let lock = NSLock()
    private var data = Data()

    func append(_ chunk: Data) {
        lock.lock()
        data.append(chunk)
        lock.unlock()
    }

    func snapshot() -> Data {
        lock.lock()
        defer { lock.unlock() }
        return data
    }
}

enum RelayCLIClientError: LocalizedError {
    case relayNotFound([String])
    case launchFailed(String)
    case emptyOutput(String)
    case decodeFailed(String)
    case commandFailed(code: String?, message: String)

    var errorDescription: String? {
        switch self {
        case let .relayNotFound(candidates):
            if candidates.isEmpty {
                return "Relay CLI not found. Rebuild the app bundle or set RELAY_CLI_PATH to a relay executable."
            }
            return "Relay CLI not found. Checked: \(candidates.joined(separator: ", "))"
        case let .launchFailed(message):
            return "Failed to launch relay CLI: \(message)"
        case let .emptyOutput(message):
            return message.isEmpty ? "Relay CLI returned no output." : message
        case let .decodeFailed(message):
            return "Failed to decode relay JSON: \(message)"
        case let .commandFailed(code, message):
            if let code {
                return "\(code): \(message)"
            }
            return message
        }
    }
}
