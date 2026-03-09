import Defaults

extension Defaults.Keys {
    static let selectedSettingsSection = Key<String>("selectedSettingsSection", default: SettingsPaneID.settings.rawValue)
    static let selectedSettingsItem = Key<String?>("selectedSettingsItem", default: nil)
    static let selectedProfileId = Key<String?>("selectedProfileId", default: nil)
}

public enum SettingsPaneID: String, CaseIterable, Identifiable, Sendable {
    case settings
    case profiles
    case activity
    case about

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .settings:
            return "Settings"
        case .profiles:
            return "Profiles"
        case .activity:
            return "Activity"
        case .about:
            return "About"
        }
    }

    public var symbol: String {
        switch self {
        case .settings:
            return "gearshape"
        case .profiles:
            return "square.grid.2x2"
        case .activity:
            return "eye"
        case .about:
            return "info.circle"
        }
    }

    public static var persistedSelection: SettingsPaneID {
        get {
            Self.storedValue(Defaults[.selectedSettingsSection])
        }
        set {
            Defaults[.selectedSettingsSection] = newValue.rawValue
        }
    }
}

enum SettingsSidebarSelection: Hashable, Identifiable, Sendable {
    case general
    case agent(AgentKind)

    var id: String { storageValue }

    var storageValue: String {
        switch self {
        case .general:
            return "general"
        case let .agent(agent):
            return "agent:\(agent.cliArgument)"
        }
    }

    static var persistedSelection: SettingsSidebarSelection {
        get {
            Self.storedValue(
                Defaults[.selectedSettingsItem],
                legacyPaneValue: Defaults[.selectedSettingsSection]
            )
        }
        set {
            Defaults[.selectedSettingsItem] = newValue.storageValue
        }
    }
}

extension SettingsPaneID {
    static func storedValue(_ value: String?) -> SettingsPaneID {
        switch value {
        case SettingsPaneID.profiles.rawValue:
            return .profiles
        case SettingsPaneID.activity.rawValue:
            return .activity
        case SettingsPaneID.about.rawValue:
            return .about
        case "general", "codex", SettingsPaneID.settings.rawValue, .none:
            return .settings
        default:
            return .settings
        }
    }
}

extension SettingsSidebarSelection {
    static func storedValue(
        _ value: String?,
        legacyPaneValue: String?
    ) -> SettingsSidebarSelection {
        switch value {
        case "general":
            return .general
        case "agent:codex":
            return .agent(.codex)
        case .none:
            if legacyPaneValue == "codex" {
                return .agent(.codex)
            }
            return .general
        default:
            return .general
        }
    }
}
