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

    func fetchProfiles() async throws -> [Profile] {
        try await run(["profiles", "list"], as: [Profile].self)
    }

    func addProfile(_ draft: ProfileDraft) async throws -> Profile {
        let payload = AddProfilePayload(draft: draft)
        return try await run(
            ["profiles", "add", "--input-json", "-"],
            input: payload,
            as: Profile.self
        )
    }

    func editProfile(profileID: String, draft: ProfileDraft) async throws -> Profile {
        let payload = EditProfilePayload(profileID: profileID, draft: draft)
        return try await run(
            ["profiles", "edit", "--input-json", "-"],
            input: payload,
            as: Profile.self
        )
    }

    func removeProfile(profileID: String) async throws -> Profile {
        try await run(
            ["profiles", "remove", "--input-json", "-"],
            input: ProfileIDPayload(id: profileID),
            as: Profile.self
        )
    }

    func importCodexProfile(nickname: String?, priority: Int) async throws -> Profile {
        try await run(
            ["profiles", "import-codex", "--input-json", "-"],
            input: ImportCodexPayload(nickname: nickname, priority: priority),
            as: Profile.self
        )
    }

    func switchToProfile(_ profileID: String) async throws -> SwitchReport {
        try await run(
            ["switch", "--input-json", "-"],
            input: SwitchPayload(target: profileID),
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

    func setProfileEnabled(profileID: String, enabled: Bool) async throws -> Profile {
        try await run(
            ["profiles", enabled ? "enable" : "disable", "--input-json", "-"],
            input: ProfileIDPayload(id: profileID),
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
        let task = Task.detached(priority: .userInitiated) {
            let process = Process()
            let stdout = Pipe()
            let stderr = Pipe()

            process.executableURL = URL(fileURLWithPath: command)
            process.arguments = ["--json"] + arguments
            process.standardOutput = stdout
            process.standardError = stderr
            if inputData != nil {
                process.standardInput = Pipe()
            }
            process.environment = environment

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

            let output = stdout.fileHandleForReading.readDataToEndOfFile()
            let errorOutput = stderr.fileHandleForReading.readDataToEndOfFile()
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

        return try await task.value
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

private struct AddProfilePayload: Encodable {
    let nickname: String
    let priority: Int
    let agentHome: String?
    let configPath: String?
    let authMode: AuthMode

    init(draft: ProfileDraft) {
        nickname = draft.nickname
        priority = draft.priority
        agentHome = draft.agentHome.isEmpty ? nil : draft.agentHome
        configPath = draft.configPath.isEmpty ? nil : draft.configPath
        authMode = draft.authMode
    }
}

private struct EditProfilePayload: Encodable {
    let id: String
    let nickname: String?
    let priority: Int?
    let agentHome: String??
    let configPath: String??
    let authMode: AuthMode?

    init(profileID: String, draft: ProfileDraft) {
        id = profileID
        nickname = draft.nickname
        priority = draft.priority
        authMode = draft.authMode
        if draft.clearAgentHome {
            agentHome = .some(nil)
        } else {
            agentHome = .some(draft.agentHome.isEmpty ? nil : draft.agentHome)
        }
        if draft.clearConfigPath {
            configPath = .some(nil)
        } else {
            configPath = .some(draft.configPath.isEmpty ? nil : draft.configPath)
        }
    }

    enum CodingKeys: String, CodingKey {
        case id
        case nickname
        case priority
        case agentHome
        case configPath
        case authMode
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(id, forKey: .id)
        try container.encodeIfPresent(nickname, forKey: .nickname)
        try container.encodeIfPresent(priority, forKey: .priority)
        switch agentHome {
        case .some(.some(let value)):
            try container.encode(value, forKey: .agentHome)
        case .some(.none):
            try container.encodeNil(forKey: .agentHome)
        case .none:
            break
        }
        switch configPath {
        case .some(.some(let value)):
            try container.encode(value, forKey: .configPath)
        case .some(.none):
            try container.encodeNil(forKey: .configPath)
        case .none:
            break
        }
        try container.encodeIfPresent(authMode, forKey: .authMode)
    }
}

private struct ProfileIDPayload: Encodable {
    let id: String
}

private struct ImportCodexPayload: Encodable {
    let nickname: String?
    let priority: Int
}

private struct SwitchPayload: Encodable {
    let target: String
}

private struct AutoSwitchPayload: Encodable {
    let enabled: Bool
}

private struct EventsListPayload: Encodable {
    let limit: Int
}

private struct LogsTailPayload: Encodable {
    let lines: Int
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
