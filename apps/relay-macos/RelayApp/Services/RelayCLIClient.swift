import Foundation

struct RelayCLIClient {
    func fetchDoctor() async throws -> DoctorReport {
        try await run(["doctor"], as: DoctorReport.self)
    }

    func fetchStatus() async throws -> StatusReport {
        try await run(["status"], as: StatusReport.self)
    }

    func fetchProfiles() async throws -> [Profile] {
        try await run(["profiles", "list"], as: [Profile].self)
    }

    func addProfile(_ draft: ProfileDraft) async throws -> Profile {
        var arguments = [
            "profiles", "add",
            "--nickname", draft.nickname,
            "--priority", String(draft.priority),
            "--auth-mode", draft.authMode.cliArgument,
        ]

        if !draft.codexHome.isEmpty {
            arguments += ["--codex-home", draft.codexHome]
        }
        if !draft.configPath.isEmpty {
            arguments += ["--config-path", draft.configPath]
        }

        return try await run(arguments, as: Profile.self)
    }

    func editProfile(profileID: String, draft: ProfileDraft) async throws -> Profile {
        var arguments = [
            "profiles", "edit", profileID,
            "--nickname", draft.nickname,
            "--priority", String(draft.priority),
            "--auth-mode", draft.authMode.cliArgument,
        ]

        if draft.clearCodexHome {
            arguments.append("--clear-codex-home")
        } else if !draft.codexHome.isEmpty {
            arguments += ["--codex-home", draft.codexHome]
        }

        if draft.clearConfigPath {
            arguments.append("--clear-config-path")
        } else if !draft.configPath.isEmpty {
            arguments += ["--config-path", draft.configPath]
        }

        return try await run(arguments, as: Profile.self)
    }

    func removeProfile(profileID: String) async throws -> Profile {
        try await run(["profiles", "remove", profileID], as: Profile.self)
    }

    func importCodexProfile(nickname: String?, priority: Int) async throws -> Profile {
        var arguments = [
            "profiles", "import-codex",
            "--priority", String(priority),
        ]

        if let nickname, !nickname.isEmpty {
            arguments += ["--nickname", nickname]
        }

        return try await run(arguments, as: Profile.self)
    }

    func switchToProfile(_ profileID: String) async throws -> SwitchReport {
        try await run(["switch", profileID], as: SwitchReport.self)
    }

    func setAutoSwitch(enabled: Bool) async throws -> AppSettings {
        try await run(
            ["auto-switch", enabled ? "enable" : "disable"],
            as: AppSettings.self
        )
    }

    func setProfileEnabled(profileID: String, enabled: Bool) async throws -> Profile {
        try await run(
            ["profiles", enabled ? "enable" : "disable", profileID],
            as: Profile.self
        )
    }

    func fetchEvents(limit: Int) async throws -> [FailureEvent] {
        try await run(
            ["events", "list", "--limit", String(limit)],
            as: [FailureEvent].self
        )
    }

    func fetchLogs(lines: Int) async throws -> LogTail {
        try await run(
            ["logs", "tail", "--lines", String(lines)],
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
        let command = try resolvedRelayCLIPath()
        let task = Task.detached(priority: .userInitiated) {
            let process = Process()
            let stdout = Pipe()
            let stderr = Pipe()

            process.executableURL = URL(fileURLWithPath: command)
            process.arguments = ["--json"] + arguments
            process.standardOutput = stdout
            process.standardError = stderr
            process.environment = ProcessInfo.processInfo.environment

            do {
                try process.run()
            } catch {
                throw RelayCLIClientError.launchFailed(error.localizedDescription)
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
        if let override = ProcessInfo.processInfo.environment["RELAY_CLI_PATH"], !override.isEmpty {
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
        guard let path = ProcessInfo.processInfo.environment["PATH"], !path.isEmpty else {
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
