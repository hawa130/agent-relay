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
    let liveAgentHome: String
    let profileCount: Int
    let activeState: ActiveState
    let settings: AppSettings

    private enum CodingKeys: String, CodingKey {
        case relayHome
        case liveAgentHome
        case liveCodexHome
        case profileCount
        case activeState
        case settings
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        relayHome = try container.decode(String.self, forKey: .relayHome)
        liveAgentHome =
            try container.decodeIfPresent(String.self, forKey: .liveAgentHome)
            ?? container.decode(String.self, forKey: .liveCodexHome)
        profileCount = try container.decode(Int.self, forKey: .profileCount)
        activeState = try container.decode(ActiveState.self, forKey: .activeState)
        settings = try container.decode(AppSettings.self, forKey: .settings)
    }
}

struct ActiveState: Decodable, Sendable {
    let activeProfileID: String?
    let lastSwitchAt: Date?
    let lastSwitchResult: SwitchOutcome
    let autoSwitchEnabled: Bool
    let lastError: String?

    private enum CodingKeys: String, CodingKey {
        case activeProfileID
        case activeProfileId
        case lastSwitchAt
        case lastSwitchResult
        case autoSwitchEnabled
        case lastError
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        activeProfileID =
            try container.decodeIfPresent(String.self, forKey: .activeProfileID)
            ?? container.decodeIfPresent(String.self, forKey: .activeProfileId)
        lastSwitchAt = try container.decodeIfPresent(Date.self, forKey: .lastSwitchAt)
        lastSwitchResult =
            try container.decodeIfPresent(SwitchOutcome.self, forKey: .lastSwitchResult)
            ?? .notRun
        autoSwitchEnabled =
            try container.decodeIfPresent(Bool.self, forKey: .autoSwitchEnabled)
            ?? false
        lastError = try container.decodeIfPresent(String.self, forKey: .lastError)
    }
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
    let agentHome: String?
    let configPath: String?
    let authMode: AuthMode
    let createdAt: Date
    let updatedAt: Date

    private enum CodingKeys: String, CodingKey {
        case id
        case nickname
        case agent
        case priority
        case enabled
        case agentHome
        case codexHome
        case configPath
        case authMode
        case createdAt
        case updatedAt
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        nickname = try container.decode(String.self, forKey: .nickname)
        agent = try container.decode(AgentKind.self, forKey: .agent)
        priority = try container.decode(Int.self, forKey: .priority)
        enabled = try container.decode(Bool.self, forKey: .enabled)
        agentHome =
            try container.decodeIfPresent(String.self, forKey: .agentHome)
            ?? container.decodeIfPresent(String.self, forKey: .codexHome)
        configPath = try container.decodeIfPresent(String.self, forKey: .configPath)
        authMode = try container.decode(AuthMode.self, forKey: .authMode)
        createdAt = try container.decode(Date.self, forKey: .createdAt)
        updatedAt = try container.decode(Date.self, forKey: .updatedAt)
    }
}

struct UsageSnapshot: Decodable, Sendable {
    let profileID: String?
    let profileName: String?
    let source: UsageSource
    let confidence: UsageConfidence
    let stale: Bool
    let lastRefreshedAt: Date
    let nextResetAt: Date?
    let session: UsageWindow
    let weekly: UsageWindow
    let autoSwitchReason: FailureReason?
    let canAutoSwitch: Bool
    let message: String?

    private enum CodingKeys: String, CodingKey {
        case profileID
        case profileId
        case profileName
        case source
        case confidence
        case stale
        case lastRefreshedAt
        case nextResetAt
        case session
        case weekly
        case autoSwitchReason
        case canAutoSwitch
        case message
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        profileID =
            try container.decodeIfPresent(String.self, forKey: .profileID)
            ?? container.decodeIfPresent(String.self, forKey: .profileId)
        profileName = try container.decodeIfPresent(String.self, forKey: .profileName)
        source = try container.decode(UsageSource.self, forKey: .source)
        confidence = try container.decode(UsageConfidence.self, forKey: .confidence)
        stale = try container.decode(Bool.self, forKey: .stale)
        lastRefreshedAt = try container.decode(Date.self, forKey: .lastRefreshedAt)
        nextResetAt = try container.decodeIfPresent(Date.self, forKey: .nextResetAt)
        session = try container.decode(UsageWindow.self, forKey: .session)
        weekly = try container.decode(UsageWindow.self, forKey: .weekly)
        autoSwitchReason = try container.decodeIfPresent(FailureReason.self, forKey: .autoSwitchReason)
        canAutoSwitch = try container.decode(Bool.self, forKey: .canAutoSwitch)
        message = try container.decodeIfPresent(String.self, forKey: .message)
    }
}

struct UsageWindow: Decodable, Sendable {
    let usedPercent: Double?
    let windowMinutes: Int?
    let resetAt: Date?
    let status: UsageStatus
    let exact: Bool
}

struct FailureEvent: Decodable, Identifiable, Sendable {
    let id: String
    let profileID: String?
    let reason: FailureReason
    let message: String
    let cooldownUntil: Date?
    let createdAt: Date

    private enum CodingKeys: String, CodingKey {
        case id
        case profileID
        case profileId
        case reason
        case message
        case cooldownUntil
        case createdAt
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        profileID =
            try container.decodeIfPresent(String.self, forKey: .profileID)
            ?? container.decodeIfPresent(String.self, forKey: .profileId)
        reason = try container.decode(FailureReason.self, forKey: .reason)
        message = try container.decode(String.self, forKey: .message)
        cooldownUntil = try container.decodeIfPresent(Date.self, forKey: .cooldownUntil)
        createdAt = try container.decode(Date.self, forKey: .createdAt)
    }
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

    private enum CodingKeys: String, CodingKey {
        case profileID
        case profileId
        case previousProfileID
        case previousProfileId
        case checkpointID
        case checkpointId
        case rollbackPerformed
        case switchedAt
        case message
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        profileID =
            try container.decodeIfPresent(String.self, forKey: .profileID)
            ?? container.decode(String.self, forKey: .profileId)
        previousProfileID =
            try container.decodeIfPresent(String.self, forKey: .previousProfileID)
            ?? container.decodeIfPresent(String.self, forKey: .previousProfileId)
        checkpointID =
            try container.decodeIfPresent(String.self, forKey: .checkpointID)
            ?? container.decode(String.self, forKey: .checkpointId)
        rollbackPerformed = try container.decode(Bool.self, forKey: .rollbackPerformed)
        switchedAt = try container.decode(Date.self, forKey: .switchedAt)
        message = try container.decode(String.self, forKey: .message)
    }
}

enum AgentKind: String, Decodable, Sendable {
    case codex = "Codex"
}

enum AuthMode: String, Codable, Sendable, CaseIterable {
    case configFilesystem = "ConfigFilesystem"
    case envReference = "EnvReference"
    case keychainReference = "KeychainReference"
}

enum FailureReason: String, Decodable, Sendable {
    case sessionExhausted = "SessionExhausted"
    case weeklyExhausted = "WeeklyExhausted"
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

enum UsageSource: String, Decodable, Sendable {
    case local = "Local"
    case fallback = "Fallback"
    case webEnhanced = "WebEnhanced"
}

enum UsageConfidence: String, Decodable, Sendable {
    case high = "High"
    case medium = "Medium"
    case low = "Low"
}

enum UsageStatus: String, Decodable, Sendable {
    case healthy = "Healthy"
    case warning = "Warning"
    case exhausted = "Exhausted"
    case unknown = "Unknown"
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
    var agentHome: String
    var configPath: String
    var authMode: AuthMode
    var clearAgentHome: Bool
    var clearConfigPath: Bool

    static let empty = ProfileDraft(
        nickname: "",
        priority: 100,
        agentHome: "",
        configPath: "",
        authMode: .configFilesystem,
        clearAgentHome: false,
        clearConfigPath: false
    )

    init(
        nickname: String,
        priority: Int,
        agentHome: String,
        configPath: String,
        authMode: AuthMode,
        clearAgentHome: Bool,
        clearConfigPath: Bool
    ) {
        self.nickname = nickname
        self.priority = priority
        self.agentHome = agentHome
        self.configPath = configPath
        self.authMode = authMode
        self.clearAgentHome = clearAgentHome
        self.clearConfigPath = clearConfigPath
    }

    init(profile: Profile) {
        self.nickname = profile.nickname
        self.priority = profile.priority
        self.agentHome = profile.agentHome ?? ""
        self.configPath = profile.configPath ?? ""
        self.authMode = profile.authMode
        self.clearAgentHome = false
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

extension JSONEncoder {
    static var relayEncoder: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        return encoder
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
