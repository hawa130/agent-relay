import AppKit

enum UsageToolbarRefreshScopeResolver {
    static func resolve(modifierFlags: NSEvent.ModifierFlags) -> UsageToolbarRefreshScope {
        modifierFlags.contains(.option) ? .all : .enabled
    }
}
