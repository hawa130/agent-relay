import Foundation

struct RelayEnvelope<T: Decodable & Sendable>: Decodable, Sendable {
    let success: Bool
    let errorCode: String?
    let message: String
    let data: T?
}

struct DoctorReport: Decodable, Sendable {
    let platform: String
    let relayHome: String
    let relayDbPath: String
    let relayLogPath: String
    let liveCodexHome: String
    let codexBinary: String?
    let codexHome: String?
    let codexHomeExists: Bool
    let managedFiles: [String]
}

struct StatusReport: Decodable, Sendable {
    let relayHome: String
    let liveCodexHome: String
    let profileCount: Int
    let activeState: ActiveState
    let settings: AppSettings
}

struct ActiveState: Decodable, Sendable {
    let activeProfileID: String?
    let lastSwitchAt: Date?
    let lastSwitchResult: SwitchOutcome
    let autoSwitchEnabled: Bool
    let lastError: String?
}

struct AppSettings: Decodable, Sendable {
    let autoSwitchEnabled: Bool
    let cooldownSeconds: Int
}

struct Profile: Decodable, Identifiable, Sendable {
    let id: String
    let nickname: String
    let agent: AgentKind
    let priority: Int
    let enabled: Bool
    let codexHome: String?
    let configPath: String?
    let authMode: AuthMode
    let createdAt: Date
    let updatedAt: Date
}

struct FailureEvent: Decodable, Identifiable, Sendable {
    let id: String
    let profileID: String?
    let reason: FailureReason
    let message: String
    let cooldownUntil: Date?
    let createdAt: Date
}

struct LogTail: Decodable, Sendable {
    let path: String
    let lines: [String]
}

struct DiagnosticsExport: Decodable, Sendable {
    let archivePath: String
    let bundleDir: String
    let createdAt: Date
}

struct SwitchReport: Decodable, Sendable {
    let profileID: String
    let previousProfileID: String?
    let checkpointID: String
    let rollbackPerformed: Bool
    let switchedAt: Date
    let message: String
}

enum AgentKind: String, Decodable, Sendable {
    case codex = "Codex"
}

enum AuthMode: String, Decodable, Sendable, CaseIterable {
    case configFilesystem = "ConfigFilesystem"
    case envReference = "EnvReference"
    case keychainReference = "KeychainReference"
}

enum FailureReason: String, Decodable, Sendable {
    case authInvalid = "AuthInvalid"
    case quotaExhausted = "QuotaExhausted"
    case rateLimited = "RateLimited"
    case commandFailed = "CommandFailed"
    case validationFailed = "ValidationFailed"
    case unknown = "Unknown"
}

enum SwitchOutcome: String, Decodable, Sendable {
    case notRun = "NotRun"
    case success = "Success"
    case failed = "Failed"
}

extension AuthMode {
    var cliArgument: String {
        switch self {
        case .configFilesystem:
            return "config-filesystem"
        case .envReference:
            return "env-reference"
        case .keychainReference:
            return "keychain-reference"
        }
    }

    var displayName: String {
        switch self {
        case .configFilesystem:
            return "Config Filesystem"
        case .envReference:
            return "Environment Reference"
        case .keychainReference:
            return "Keychain Reference"
        }
    }
}

struct ProfileDraft: Sendable {
    var nickname: String
    var priority: Int
    var codexHome: String
    var configPath: String
    var authMode: AuthMode
    var clearCodexHome: Bool
    var clearConfigPath: Bool

    static let empty = ProfileDraft(
        nickname: "",
        priority: 100,
        codexHome: "",
        configPath: "",
        authMode: .configFilesystem,
        clearCodexHome: false,
        clearConfigPath: false
    )

    init(
        nickname: String,
        priority: Int,
        codexHome: String,
        configPath: String,
        authMode: AuthMode,
        clearCodexHome: Bool,
        clearConfigPath: Bool
    ) {
        self.nickname = nickname
        self.priority = priority
        self.codexHome = codexHome
        self.configPath = configPath
        self.authMode = authMode
        self.clearCodexHome = clearCodexHome
        self.clearConfigPath = clearConfigPath
    }

    init(profile: Profile) {
        self.nickname = profile.nickname
        self.priority = profile.priority
        self.codexHome = profile.codexHome ?? ""
        self.configPath = profile.configPath ?? ""
        self.authMode = profile.authMode
        self.clearCodexHome = false
        self.clearConfigPath = false
    }
}

extension JSONDecoder {
    static var relayDecoder: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        decoder.dateDecodingStrategy = .custom { decoder in
            let container = try decoder.singleValueContainer()
            let value = try container.decode(String.self)

            if let date = RelayDateParser.parse(value) {
                return date
            }

            throw DecodingError.dataCorruptedError(
                in: container,
                debugDescription: "Invalid RFC3339 date: \(value)"
            )
        }
        return decoder
    }
}

enum RelayDateParser {
    static func parse(_ value: String) -> Date? {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = formatter.date(from: value) {
            return date
        }

        let fallback = ISO8601DateFormatter()
        fallback.formatOptions = [.withInternetDateTime]
        return fallback.date(from: value)
    }
}
