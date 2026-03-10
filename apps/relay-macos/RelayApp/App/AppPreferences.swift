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

enum ProfilesSidebarFilter: String, CaseIterable, Hashable, Identifiable, Sendable {
    case all
    case codex

    var id: String { rawValue }

    var title: String {
        switch self {
        case .all:
            return "All"
        case .codex:
            return "Codex"
        }
    }

    var icon: String {
        switch self {
        case .all:
            return "square.grid.2x2"
        case .codex:
            return "command.square"
        }
    }

    var emptyStateDescription: String {
        switch self {
        case .all:
            return "Add an account from the toolbar to create your first profile."
        case .codex:
            return "No Codex profiles are available in this view yet."
        }
    }
}
