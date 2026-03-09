import Defaults

extension Defaults.Keys {
    static let selectedSettingsSection = Key<String>("selectedSettingsSection", default: SettingsPaneID.general.rawValue)
    static let selectedProfileId = Key<String?>("selectedProfileId", default: nil)
}

public enum SettingsPaneID: String, CaseIterable, Identifiable, Sendable {
    case general
    case profiles
    case activity
    case about

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .general:
            return "General"
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
        case .general:
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
            SettingsPaneID(rawValue: Defaults[.selectedSettingsSection]) ?? .general
        }
        set {
            Defaults[.selectedSettingsSection] = newValue.rawValue
        }
    }
}
