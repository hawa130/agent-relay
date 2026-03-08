import Defaults

extension Defaults.Keys {
    static let selectedSettingsSection = Key<String>("selectedSettingsSection", default: SettingsSection.general.rawValue)
    static let selectedProfileID = Key<String?>("selectedProfileID", default: nil)
}

enum SettingsSection: String, CaseIterable, Identifiable, Sendable {
    case general
    case profiles
    case activity

    var id: String { rawValue }

    var title: String {
        switch self {
        case .general:
            return "General"
        case .profiles:
            return "Profiles"
        case .activity:
            return "Activity"
        }
    }

    var symbol: String {
        switch self {
        case .general:
            return "slider.horizontal.3"
        case .profiles:
            return "person.3"
        case .activity:
            return "waveform.path.ecg"
        }
    }
}
