import Foundation

struct DoctorReport: Decodable {
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

struct StatusReport: Decodable {
    let relayHome: String
    let liveAgentHome: String
    let profileCount: Int
    var activeState: ActiveState
    let settings: AppSettings
}

struct ActiveState: Decodable {
    let activeProfileId: String?
    let lastSwitchAt: Date?
    let lastSwitchResult: SwitchOutcome
    let autoSwitchEnabled: Bool

    private enum CodingKeys: String, CodingKey {
        case activeProfileId
        case lastSwitchAt
        case lastSwitchResult
        case autoSwitchEnabled
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
    }

    init(
        activeProfileId: String?,
        lastSwitchAt: Date?,
        lastSwitchResult: SwitchOutcome,
        autoSwitchEnabled: Bool)
    {
        self.activeProfileId = activeProfileId
        self.lastSwitchAt = lastSwitchAt
        self.lastSwitchResult = lastSwitchResult
        self.autoSwitchEnabled = autoSwitchEnabled
    }
}

struct AppSettings: Decodable {
    let autoSwitchEnabled: Bool
    let cooldownSeconds: Int
    let refreshIntervalSeconds: Int
    let networkQueryConcurrency: Int
    let proxyMode: String

    private enum CodingKeys: String, CodingKey {
        case autoSwitchEnabled
        case cooldownSeconds
        case refreshIntervalSeconds
        case networkQueryConcurrency
        case proxyMode
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        autoSwitchEnabled =
            try container.decodeIfPresent(Bool.self, forKey: .autoSwitchEnabled)
                ?? false
        cooldownSeconds =
            try container.decodeIfPresent(Int.self, forKey: .cooldownSeconds)
                ?? 600
        refreshIntervalSeconds =
            try container.decodeIfPresent(Int.self, forKey: .refreshIntervalSeconds)
                ?? 60
        networkQueryConcurrency =
            try container.decodeIfPresent(Int.self, forKey: .networkQueryConcurrency)
                ?? 10
        proxyMode =
            try container.decodeIfPresent(String.self, forKey: .proxyMode)
                ?? "system"
    }

    init(
        autoSwitchEnabled: Bool,
        cooldownSeconds: Int,
        refreshIntervalSeconds: Int,
        networkQueryConcurrency: Int,
        proxyMode: String = "system")
    {
        self.autoSwitchEnabled = autoSwitchEnabled
        self.cooldownSeconds = cooldownSeconds
        self.refreshIntervalSeconds = refreshIntervalSeconds
        self.networkQueryConcurrency = networkQueryConcurrency
        self.proxyMode = proxyMode
    }
}

struct CodexSettings: Decodable {
    let usageSourceMode: UsageSourceMode

    private enum CodingKeys: String, CodingKey {
        case usageSourceMode
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        usageSourceMode =
            try container.decodeIfPresent(UsageSourceMode.self, forKey: .usageSourceMode)
                ?? .auto
    }

    init(usageSourceMode: UsageSourceMode) {
        self.usageSourceMode = usageSourceMode
    }
}

struct Profile: Decodable, Identifiable {
    let id: String
    let nickname: String
    let agent: AgentKind
    let priority: Int
    let enabled: Bool
    let accountState: ProfileAccountState
    let accountErrorHTTPStatus: Int?
    let accountStateUpdatedAt: Date?
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
        case accountState
        case accountErrorHTTPStatus = "accountErrorHttpStatus"
        case accountStateUpdatedAt
        case agentHome
        case configPath
        case authMode
        case createdAt
        case updatedAt
    }

    init(
        id: String,
        nickname: String,
        agent: AgentKind,
        priority: Int,
        enabled: Bool,
        accountState: ProfileAccountState = .healthy,
        accountErrorHTTPStatus: Int? = nil,
        accountStateUpdatedAt: Date? = nil,
        agentHome: String?,
        configPath: String?,
        authMode: AuthMode,
        createdAt: Date,
        updatedAt: Date)
    {
        self.id = id
        self.nickname = nickname
        self.agent = agent
        self.priority = priority
        self.enabled = enabled
        self.accountState = accountState
        self.accountErrorHTTPStatus = accountErrorHTTPStatus
        self.accountStateUpdatedAt = accountStateUpdatedAt
        self.agentHome = agentHome
        self.configPath = configPath
        self.authMode = authMode
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        nickname = try container.decode(String.self, forKey: .nickname)
        agent = try container.decode(AgentKind.self, forKey: .agent)
        priority = try container.decode(Int.self, forKey: .priority)
        enabled = try container.decode(Bool.self, forKey: .enabled)
        accountState = try container.decode(ProfileAccountState.self, forKey: .accountState)
        accountErrorHTTPStatus = try container.decodeIfPresent(Int.self, forKey: .accountErrorHTTPStatus)
        accountStateUpdatedAt = try container.decodeIfPresent(Date.self, forKey: .accountStateUpdatedAt)
        agentHome = try container.decodeIfPresent(String.self, forKey: .agentHome)
        configPath = try container.decodeIfPresent(String.self, forKey: .configPath)
        authMode = try container.decode(AuthMode.self, forKey: .authMode)
        createdAt = try container.decode(Date.self, forKey: .createdAt)
        updatedAt = try container.decode(Date.self, forKey: .updatedAt)
    }
}

enum ProfileAccountState: String, Decodable {
    case healthy = "Healthy"
    case accountUnavailable = "AccountUnavailable"
}

struct ProfileListItem: Decodable {
    let profile: Profile
    let isActive: Bool
    let usageSummary: UsageSnapshot?
    let currentFailureEvents: [FailureEvent]
}

struct UsageSnapshot: Decodable {
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
    let remoteError: UsageRemoteError?
    let planHint: String?
}

struct UsageRemoteError: Decodable, Equatable {
    let kind: UsageRemoteErrorKind
    let httpStatus: Int?
}

enum UsageRemoteErrorKind: String, Decodable, Equatable {
    case account = "Account"
    case network = "Network"
    case other = "Other"
}

struct CodexSettingsDraft: Encodable {
    let sourceMode: UsageSourceMode?
}

struct UsageWindow: Decodable {
    let usedPercent: Double?
    let windowMinutes: Int?
    let resetAt: Date?
    let status: UsageStatus
    let exact: Bool
}

struct FailureEvent: Decodable, Identifiable {
    let id: String
    let profileId: String?
    let reason: FailureReason
    let message: String
    let cooldownUntil: Date?
    let resolvedAt: Date?
    let createdAt: Date
}

struct LogTail: Decodable {
    let path: String
    let lines: [String]
}

struct DiagnosticsExport: Decodable {
    let archivePath: String
    let bundleDir: String
    let createdAt: Date
}

struct SwitchReport: Decodable {
    let profileId: String
    let previousProfileId: String?
    let checkpointId: String
    let rollbackPerformed: Bool
    let switchedAt: Date
    let message: String
}

struct RPCRequestEnvelope<Params: Encodable & Sendable>: Encodable {
    let jsonrpc = "2.0"
    let id: String
    let method: String
    let params: Params
}

struct RPCResponseEnvelope<Result: Decodable & Sendable>: Decodable {
    let jsonrpc: String
    let id: String
    let result: Result
}

struct RPCErrorEnvelope: Decodable {
    let jsonrpc: String
    let id: String?
    let error: RPCErrorObject
}

struct RPCErrorObject: Decodable {
    let code: Int
    let message: String
    let data: RPCErrorData?
}

struct RPCErrorData: Decodable {
    let relayErrorCode: String?
}

struct RPCInitializeParams: Encodable {
    let protocolVersion = "1"
    let clientInfo = RPCClientInfo(name: "relay-macos", version: "0.1.0")
    let capabilities = RPCClientCapabilities(
        supportsSubscriptions: true,
        supportsHealthUpdates: true)
}

struct RPCClientInfo: Encodable {
    let name: String
    let version: String
}

struct RPCClientCapabilities: Encodable {
    let supportsSubscriptions: Bool
    let supportsHealthUpdates: Bool
}

struct RPCInitializeResult: Decodable {
    let protocolVersion: String
    let initialState: RPCInitialState
}

struct RPCInitialState: Decodable {
    let status: StatusReport
    let profiles: [ProfileListItem]
    let codexSettings: CodexSettings
    let engine: RPCEngineState
}

struct RPCEngineState: Decodable {
    let startedAt: Date
    let connectionState: EngineConnectionState
}

enum EngineConnectionState: String, Decodable {
    case starting = "Starting"
    case ready = "Ready"
    case degraded = "Degraded"
}

struct RPCSubscribeParams: Encodable {
    let topics: [String]
}

struct RPCUsageRefreshResult: Decodable {
    let snapshots: [UsageSnapshot]
}

struct UsageResult: Decodable {
    let snapshot: UsageSnapshot
}

struct RPCSettingsResult: Decodable {
    let app: AppSettings
    let codex: CodexSettings
}

struct AppSettingsPatch: Encodable {
    let autoSwitchEnabled: Bool?
    let cooldownSeconds: Int?
    let refreshIntervalSeconds: Int?
    let networkQueryConcurrency: Int?
    let proxyMode: String?
}

struct RPCSettingsUpdatePayload: Encodable {
    let app: AppSettingsPatch?
    let codex: CodexSettingsDraft?
}

struct RPCEventsResult: Decodable {
    let events: [FailureEvent]
}

struct RPCLogsResult: Decodable {
    let logs: LogTail
}

struct RPCActivityRefreshResult: Decodable {
    let events: [FailureEvent]
    let logs: LogTail
}

enum RelaySessionUpdate {
    case usageUpdated(UsageUpdatedNotification)
    case queryStateUpdated(QueryStateUpdatedNotification)
    case activeStateUpdated(ActiveStateUpdatedNotification)
    case settingsUpdated(SettingsUpdatedNotification)
    case profilesUpdated(ProfilesUpdatedNotification)
    case activityEventsUpdated(ActivityEventsUpdatedNotification)
    case activityLogsUpdated(ActivityLogsUpdatedNotification)
    case doctorUpdated(DoctorUpdatedNotification)
    case switchCompleted(SwitchCompletedNotification)
    case switchFailed(SwitchFailedNotification)
    case taskUpdated(TaskUpdatedNotification)
    case healthUpdated(HealthUpdatedNotification)
}

struct UsageUpdatedNotification: Decodable {
    let snapshots: [UsageSnapshot]
    let trigger: UsageUpdateTrigger
}

struct QueryStateUpdatedNotification: Decodable {
    let states: [QueryStateItem]
}

struct QueryStateItem: Decodable, Hashable {
    let key: QueryStateKey
    let status: QueryStateStatus
    let trigger: QueryStateTrigger
    let errorCode: String?
    let message: String?
    let updatedAt: Date
}

struct QueryStateKey: Decodable, Hashable {
    let kind: QueryStateKind
    let profileId: String?
}

enum QueryStateKind: String, Decodable, Hashable {
    case usageProfile = "UsageProfile"
}

enum QueryStateStatus: String, Decodable, Hashable {
    case pending = "Pending"
    case error = "Error"
}

enum QueryStateTrigger: String, Decodable, Hashable {
    case startup = "Startup"
    case interval = "Interval"
    case manual = "Manual"
    case postSwitch = "PostSwitch"
}

struct ActiveStateUpdatedNotification: Decodable {
    let activeState: ActiveState
    let activeProfile: ProfileListItem?
}

struct SettingsUpdatedNotification: Decodable {
    let settings: RPCSettingsResult
}

struct ProfilesUpdatedNotification: Decodable {
    let profiles: [ProfileListItem]
}

struct ActivityEventsUpdatedNotification: Decodable {
    let events: [FailureEvent]
}

struct ActivityLogsUpdatedNotification: Decodable {
    let logs: LogTail
}

struct DoctorUpdatedNotification: Decodable {
    let report: DoctorReport
}

struct SwitchCompletedNotification: Decodable {
    let report: SwitchReport
    let trigger: SwitchTrigger
}

struct SwitchFailedNotification: Decodable {
    let errorCode: String
    let message: String
    let profileId: String?
    let trigger: SwitchTrigger
}

struct TaskStartResult: Decodable {
    let taskId: String
    let kind: RelayTaskKind
    let accepted: Bool
}

struct TaskCancelResult: Decodable {
    let accepted: Bool
}

struct TaskUpdatedNotification: Decodable {
    let task: RelayTaskUpdate
}

struct RelayTaskUpdate: Decodable {
    let taskId: String
    let kind: RelayTaskKind
    let status: RelayTaskStatus
    let startedAt: Date
    let finishedAt: Date?
    let message: String?
    let errorCode: String?
    let profileLoginResult: AgentLinkResult?

    private enum CodingKeys: String, CodingKey {
        case taskId
        case kind
        case status
        case startedAt
        case finishedAt
        case message
        case errorCode
        case result
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        taskId = try container.decode(String.self, forKey: .taskId)
        kind = try container.decode(RelayTaskKind.self, forKey: .kind)
        status = try container.decode(RelayTaskStatus.self, forKey: .status)
        startedAt = try container.decode(Date.self, forKey: .startedAt)
        finishedAt = try container.decodeIfPresent(Date.self, forKey: .finishedAt)
        message = try container.decodeIfPresent(String.self, forKey: .message)
        errorCode = try container.decodeIfPresent(String.self, forKey: .errorCode)

        switch kind {
        case .profileLogin:
            profileLoginResult = try container.decodeIfPresent(AgentLinkResult.self, forKey: .result)
        }
    }

    init(
        taskId: String,
        kind: RelayTaskKind,
        status: RelayTaskStatus,
        startedAt: Date,
        finishedAt: Date?,
        message: String?,
        errorCode: String?,
        profileLoginResult: AgentLinkResult?)
    {
        self.taskId = taskId
        self.kind = kind
        self.status = status
        self.startedAt = startedAt
        self.finishedAt = finishedAt
        self.message = message
        self.errorCode = errorCode
        self.profileLoginResult = profileLoginResult
    }

    var isTerminal: Bool {
        switch status {
        case .succeeded, .failed, .cancelled:
            true
        case .pending:
            false
        }
    }
}

enum RelayTaskKind: String, Decodable {
    case profileLogin = "ProfileLogin"
}

enum RelayTaskStatus: String, Decodable {
    case pending = "Pending"
    case succeeded = "Succeeded"
    case failed = "Failed"
    case cancelled = "Cancelled"
}

struct HealthUpdatedNotification: Decodable {
    let state: EngineConnectionState
    let detail: String?
}

enum UsageUpdateTrigger: String, Decodable {
    case startup = "Startup"
    case interval = "Interval"
    case manual = "Manual"
    case postSwitch = "PostSwitch"
}

enum SwitchTrigger: String, Decodable {
    case manual = "Manual"
    case auto = "Auto"
}

struct ProfileProbeIdentity: Decodable {
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
                debugDescription: "Missing probe identity account/principal identifier")
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

struct AgentLinkResult: Decodable {
    let profile: Profile
    let probeIdentity: ProfileProbeIdentity
    let activated: Bool
}

struct RPCTaskID: Encodable {
    let taskId: String
}

enum AgentKind: String, Codable {
    case codex = "Codex"

    var cliArgument: String {
        rawValue.lowercased()
    }

    var displayName: String {
        rawValue
    }
}

enum AuthMode: String, Codable, CaseIterable {
    case configFilesystem = "ConfigFilesystem"
    case envReference = "EnvReference"
    case keychainReference = "KeychainReference"
}

enum FailureReason: String, Decodable {
    case sessionExhausted = "SessionExhausted"
    case weeklyExhausted = "WeeklyExhausted"
    case accountUnavailable = "AccountUnavailable"
    case authInvalid = "AuthInvalid"
    case quotaExhausted = "QuotaExhausted"
    case rateLimited = "RateLimited"
    case commandFailed = "CommandFailed"
    case validationFailed = "ValidationFailed"
    case unknown = "Unknown"

    var displayName: String {
        switch self {
        case .sessionExhausted:
            "Session Exhausted"
        case .weeklyExhausted:
            "Weekly Exhausted"
        case .accountUnavailable:
            "Account Unavailable"
        case .authInvalid:
            "Authentication Invalid"
        case .quotaExhausted:
            "Quota Exhausted"
        case .rateLimited:
            "Rate Limited"
        case .commandFailed:
            "Command Failed"
        case .validationFailed:
            "Validation Failed"
        case .unknown:
            "Unknown"
        }
    }
}

enum SwitchOutcome: String, Decodable {
    case notRun = "NotRun"
    case success = "Success"
    case failed = "Failed"
}

enum UsageSource: String, Decodable {
    case local = "Local"
    case fallback = "Fallback"
    case webEnhanced = "WebEnhanced"

    var displayName: String {
        switch self {
        case .local:
            "On-device"
        case .fallback:
            "Fallback"
        case .webEnhanced:
            "Web"
        }
    }
}

enum UsageSourceMode: String, CaseIterable, Decodable, Encodable {
    case auto = "Auto"
    case local = "Local"
    case webEnhanced = "WebEnhanced"

    var cliValue: String {
        switch self {
        case .auto:
            "auto"
        case .local:
            "local"
        case .webEnhanced:
            "web-enhanced"
        }
    }

    var displayName: String {
        switch self {
        case .auto:
            "Auto"
        case .local:
            "On-device"
        case .webEnhanced:
            "Web"
        }
    }

    var helpText: String {
        switch self {
        case .auto:
            "AgentRelay chooses the best available source for Codex usage."
        case .local:
            "Read usage from the local Codex environment only."
        case .webEnhanced:
            "Prefer the remote web-backed source when available."
        }
    }
}

enum UsageConfidence: String, Decodable {
    case high = "High"
    case medium = "Medium"
    case low = "Low"
}

enum UsageStatus: String, Decodable {
    case healthy = "Healthy"
    case warning = "Warning"
    case exhausted = "Exhausted"
    case unknown = "Unknown"
}

extension UsageSnapshot {
    var userFacingNote: String? {
        message?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nilIfEmpty
    }
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}

extension AuthMode {
    var cliArgument: String {
        switch self {
        case .configFilesystem:
            "config-filesystem"
        case .envReference:
            "env-reference"
        case .keychainReference:
            "keychain-reference"
        }
    }

    var displayName: String {
        switch self {
        case .configFilesystem:
            "Config Filesystem"
        case .envReference:
            "Environment Reference"
        case .keychainReference:
            "Keychain Reference"
        }
    }
}

struct ProfileDraft {
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
        clearConfigPath: Bool)
    {
        self.nickname = nickname
        self.priority = priority
        self.agentHome = agentHome
        self.configPath = configPath
        self.authMode = authMode
        self.clearAgentHome = clearAgentHome
        self.clearConfigPath = clearConfigPath
    }

    init(profile: Profile) {
        nickname = profile.nickname
        priority = profile.priority
        agentHome = profile.agentHome ?? ""
        configPath = profile.configPath ?? ""
        authMode = profile.authMode
        clearAgentHome = false
        clearConfigPath = false
    }
}

struct EditProfilePayload: Encodable {
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

struct ProfileIdPayload: Encodable {
    let id: String
}

struct ImportProfilePayload: Encodable {
    let nickname: String?
    let priority: Int
}

struct LoginProfilePayload: Encodable {
    let nickname: String?
    let priority: Int
}

struct AutoSwitchPayload: Encodable {
    let enabled: Bool
}

struct EventsListPayload: Encodable {
    let limit: Int
}

struct LogsTailPayload: Encodable {
    let lines: Int
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
                debugDescription: "Invalid RFC3339 date: \(value)")
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
