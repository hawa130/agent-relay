import Defaults

extension Defaults.Keys {
    static let selectedProfileId = Key<String?>("selectedProfileId", default: nil)
}

public enum RelayWindowID: String, CaseIterable, Identifiable, Sendable {
    case settings
    case profiles

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .settings:
            return "Settings"
        case .profiles:
            return "Profiles"
        }
    }

    public var symbol: String {
        switch self {
        case .settings:
            return "gearshape"
        case .profiles:
            return "square.grid.2x2"
        }
    }
}

enum SettingsSidebarSelection: Hashable, Identifiable, Sendable {
    case general
    case agent(AgentKind)
    var id: String {
        switch self {
        case .general:
            return "general"
        case let .agent(agent):
            return "agent:\(agent.cliArgument)"
        }
    }
}
