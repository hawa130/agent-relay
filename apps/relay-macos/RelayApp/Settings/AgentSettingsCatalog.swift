import AppKit
import SwiftUI

struct AgentSettingsDescriptor: Identifiable {
    let agent: AgentKind
    let title: String
    let vendorTitle: String
    let subtitle: String
    let iconResourceName: String
    let accentColor: Color
    let visualScale: CGFloat

    var id: String {
        agent.cliArgument
    }

    func iconImage(template: Bool = true) -> NSImage? {
        guard let url = Bundle.module.url(forResource: iconResourceName, withExtension: "svg"),
              let image = NSImage(contentsOf: url)
        else {
            return nil
        }

        image.size = NSSize(width: 18, height: 18)
        image.isTemplate = template
        return image
    }
}

enum AgentSettingsCatalog {
    static let supportedAgents: [AgentSettingsDescriptor] = [
        AgentSettingsDescriptor(
            agent: .codex,
            title: "Codex",
            vendorTitle: "OpenAI",
            subtitle: "Configure usage source and other Codex-specific behavior.",
            iconResourceName: "ProviderIcon-codex",
            accentColor: Color(red: 73 / 255, green: 163 / 255, blue: 176 / 255),
            visualScale: 1.16)
    ]

    static func descriptor(for agent: AgentKind) -> AgentSettingsDescriptor? {
        supportedAgents.first { $0.agent == agent }
    }
}

struct AgentIcon: View {
    let agent: AgentKind
    var size: CGFloat = 18
    var tint: Color?

    var body: some View {
        let foreground = tint ?? .secondary
        if let descriptor = AgentSettingsCatalog.descriptor(for: agent),
           let image = descriptor.iconImage()
        {
            iconImage(image, descriptor: descriptor, foreground: foreground)
        } else {
            Image(systemName: "terminal")
                .font(.system(size: size, weight: .semibold))
                .foregroundStyle(foreground)
                .frame(width: size, height: size)
        }
    }

    private func iconImage(_ image: NSImage, descriptor: AgentSettingsDescriptor, foreground: Color) -> some View {
        Image(nsImage: image)
            .resizable()
            .aspectRatio(contentMode: .fit)
            .frame(
                width: size * descriptor.visualScale,
                height: size * descriptor.visualScale)
            .foregroundStyle(foreground)
    }
}
