import Defaults

extension Defaults.Keys {
    static let selectedProfileId = Key<String?>("selectedProfileId", default: nil)
}

public enum RelayWindowID: String, CaseIterable, Identifiable, Sendable {
    case settings
    case profiles

    public var id: String {
        rawValue
    }

    public var title: String {
        switch self {
        case .settings:
            "Settings"
        case .profiles:
            "Profiles"
        }
    }

    public var symbol: String {
        switch self {
        case .settings:
            "gearshape"
        case .profiles:
            "square.grid.2x2"
        }
    }
}

enum SettingsSidebarSelection: Hashable, Identifiable {
    case general
    case agent(AgentKind)
    var id: String {
        switch self {
        case .general:
            "general"
        case let .agent(agent):
            "agent:\(agent.cliArgument)"
        }
    }
}

enum ProfilesSidebarFilter: String, CaseIterable, Hashable, Identifiable {
    case all
    case codex

    var id: String {
        rawValue
    }

    var title: String {
        switch self {
        case .all:
            "All"
        case .codex:
            "Codex"
        }
    }

    var emptyStateDescription: String {
        switch self {
        case .all:
            "Add an account from the toolbar to create your first profile."
        case .codex:
            "No Codex profiles are available in this view yet."
        }
    }
}
