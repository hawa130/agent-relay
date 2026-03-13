import Foundation

enum RelayClientError: LocalizedError {
    case relayNotFound([String])
    case launchFailed(String)
    case invalidResponse(String)
    case commandFailed(code: String?, message: String)

    var errorDescription: String? {
        switch self {
        case let .relayNotFound(candidates):
            if candidates.isEmpty {
                return "AgentRelay CLI not found. Rebuild the app bundle or set AGRELAY_CLI_PATH to an agrelay executable."
            }
            return "AgentRelay CLI not found. Checked: \(candidates.joined(separator: ", "))"
        case let .launchFailed(message):
            return "Failed to launch AgentRelay CLI: \(message)"
        case let .invalidResponse(message):
            return "AgentRelay returned an unexpected payload: \(message)"
        case let .commandFailed(code, message):
            if let code {
                return "\(code): \(message)"
            }
            return message
        }
    }
}
