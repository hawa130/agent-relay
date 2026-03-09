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
    let primaryAgent: AgentKind?
    let liveAgentHome: String
    let agentBinary: String?
    let defaultAgentHome: String?
    let defaultAgentHomeExists: Bool
    let managedFiles: [String]
}

struct StatusReport: Decodable, Sendable {
    let relayHome: String
    let liveAgentHome: String
    let profileCount: Int
    let activeState: ActiveState
    let settings: AppSettings
}

struct ActiveState: Decodable, Sendable {
    let activeProfileId: String?
    let lastSwitchAt: Date?
    let lastSwitchResult: SwitchOutcome
    let autoSwitchEnabled: Bool
    let lastError: String?

    private enum CodingKeys: String, CodingKey {
        case activeProfileId
        case lastSwitchAt
        case lastSwitchResult
        case autoSwitchEnabled
        case lastError
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        activeProfileId = try container.decodeIfPresent(String.self, forKey: .activeProfileId)
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
    let usageSourceMode: UsageSourceMode
    let menuOpenRefreshStaleAfterSeconds: Int
    let usageBackgroundRefreshEnabled: Bool
    let usageBackgroundRefreshIntervalSeconds: Int

    private enum CodingKeys: String, CodingKey {
        case autoSwitchEnabled
        case cooldownSeconds
        case usageSourceMode
        case menuOpenRefreshStaleAfterSeconds
        case usageBackgroundRefreshEnabled
        case usageBackgroundRefreshIntervalSeconds
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        autoSwitchEnabled =
            try container.decodeIfPresent(Bool.self, forKey: .autoSwitchEnabled)
            ?? false
        cooldownSeconds =
            try container.decodeIfPresent(Int.self, forKey: .cooldownSeconds)
            ?? 600
        usageSourceMode =
            try container.decodeIfPresent(UsageSourceMode.self, forKey: .usageSourceMode)
            ?? .auto
        menuOpenRefreshStaleAfterSeconds =
            try container.decodeIfPresent(Int.self, forKey: .menuOpenRefreshStaleAfterSeconds)
            ?? 10
        usageBackgroundRefreshEnabled =
            try container.decodeIfPresent(Bool.self, forKey: .usageBackgroundRefreshEnabled)
            ?? true
        usageBackgroundRefreshIntervalSeconds =
            try container.decodeIfPresent(Int.self, forKey: .usageBackgroundRefreshIntervalSeconds)
            ?? 120
    }
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
}

struct UsageSnapshot: Decodable, Sendable {
    let profileId: String?
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
}

struct UsageSettingsDraft: Encodable, Sendable {
    let sourceMode: UsageSourceMode?
    let menuOpenRefreshStaleAfterSeconds: Int?
    let backgroundRefreshEnabled: Bool?
    let backgroundRefreshIntervalSeconds: Int?
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
    let profileId: String?
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
    let profileId: String
    let previousProfileId: String?
    let checkpointId: String
    let rollbackPerformed: Bool
    let switchedAt: Date
    let message: String
}

struct ProfileProbeIdentity: Decodable, Sendable {
    let profileId: String
    let accountId: String
    let email: String?
    let planHint: String?
    let provider: String?
    let principalId: String?
    let displayName: String?

    private enum CodingKeys: String, CodingKey {
        case profileId
        case provider
        case accountId
        case principalId
        case displayName
        case credentials
        case metadata
        case email
        case planHint
    }

    private enum CredentialsKeys: String, CodingKey {
        case accountId
    }

    private enum MetadataKeys: String, CodingKey {
        case email
        case planHint
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        profileId = try container.decode(String.self, forKey: .profileId)
        provider = try container.decodeIfPresent(String.self, forKey: .provider)
        principalId = try container.decodeIfPresent(String.self, forKey: .principalId)
        displayName = try container.decodeIfPresent(String.self, forKey: .displayName)

        var nestedAccountID: String?
        if container.contains(.credentials) {
            let credentials = try container.nestedContainer(keyedBy: CredentialsKeys.self, forKey: .credentials)
            nestedAccountID = try credentials.decodeIfPresent(String.self, forKey: .accountId)
        }
        guard let accountId =
            try container.decodeIfPresent(String.self, forKey: .accountId)
            ?? nestedAccountID
            ?? principalId
        else {
            throw DecodingError.dataCorruptedError(
                forKey: .accountId,
                in: container,
                debugDescription: "Missing probe identity account/principal identifier"
            )
        }
        self.accountId = accountId

        var nestedEmail: String?
        var nestedPlanHint: String?
        if container.contains(.metadata) {
            let metadata = try container.nestedContainer(keyedBy: MetadataKeys.self, forKey: .metadata)
            nestedEmail = try metadata.decodeIfPresent(String.self, forKey: .email)
            nestedPlanHint = try metadata.decodeIfPresent(String.self, forKey: .planHint)
        }
        email =
            try container.decodeIfPresent(String.self, forKey: .email)
            ?? nestedEmail
            ?? displayName
        planHint =
            try container.decodeIfPresent(String.self, forKey: .planHint)
            ?? nestedPlanHint
    }
}

struct AgentLinkResult: Decodable, Sendable {
    let profile: Profile
    let probeIdentity: ProfileProbeIdentity
    let activated: Bool
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

enum UsageSourceMode: String, CaseIterable, Decodable, Encodable, Sendable {
    case auto = "Auto"
    case local = "Local"
    case webEnhanced = "WebEnhanced"

    var cliValue: String {
        switch self {
        case .auto:
            return "auto"
        case .local:
            return "local"
        case .webEnhanced:
            return "web-enhanced"
        }
    }

    var displayName: String {
        switch self {
        case .auto:
            return "Auto"
        case .local:
            return "CLI (RPC/PTy)"
        case .webEnhanced:
            return "OAuth API"
        }
    }
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

struct EditProfilePayload: Encodable, Sendable {
    let id: String
    let nickname: String
    let priority: Int
    let agentHome: String??
    let configPath: String??
    let authMode: AuthMode

    init(profileId: String, draft: ProfileDraft) {
        id = profileId
        nickname = draft.nickname
        priority = draft.priority
        agentHome = draft.clearAgentHome ? .some(nil) : .some(draft.agentHome.isEmpty ? nil : draft.agentHome)
        configPath = draft.clearConfigPath ? .some(nil) : .some(draft.configPath.isEmpty ? nil : draft.configPath)
        authMode = draft.authMode
    }
}

struct ProfileIdPayload: Encodable, Sendable {
    let id: String
}

struct ImportProfilePayload: Encodable, Sendable {
    let agent = "codex"
    let nickname: String?
    let priority: Int
}

struct LoginProfilePayload: Encodable, Sendable {
    let agent = "codex"
    let nickname: String?
    let priority: Int
}

struct SwitchPayload: Encodable, Sendable {
    let target: String
}

struct AutoSwitchPayload: Encodable, Sendable {
    let enabled: Bool
}

struct EventsListPayload: Encodable, Sendable {
    let limit: Int
}

struct LogsTailPayload: Encodable, Sendable {
    let lines: Int
}

struct UsageRefreshPayload: Encodable, Sendable {
    let id: String?
    let enabled: Bool
    let all: Bool
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
