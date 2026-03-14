import SwiftUI

struct NativeToolbarSymbolButton: View {
    let title: String
    let systemImage: String
    let role: ButtonRole?
    let isEnabled: Bool
    let helpText: String
    let action: () -> Void

    init(
        _ title: String,
        systemImage: String,
        role: ButtonRole? = nil,
        isEnabled: Bool = true,
        helpText: String? = nil,
        action: @escaping () -> Void)
    {
        self.title = title
        self.systemImage = systemImage
        self.role = role
        self.isEnabled = isEnabled
        self.helpText = helpText ?? title
        self.action = action
    }

    var body: some View {
        Button(role: role, action: action) {
            Label(title, systemImage: systemImage)
        }
        .accessibilityLabel(title)
        .help(helpText)
        .disabled(!isEnabled)
    }
}
