enum AddProfileCatalogProfileText {
    static func text(for descriptor: AgentSettingsDescriptor, profileCount: Int) -> String {
        let profileText = profileCount == 1 ? "1 profile" : "\(profileCount) profiles"
        return "\(descriptor.vendorTitle) • \(profileText)"
    }
}
