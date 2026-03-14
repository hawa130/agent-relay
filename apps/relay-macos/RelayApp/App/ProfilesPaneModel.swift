import Combine
import Foundation

@MainActor
public final class ProfilesPaneModel: ObservableObject {
    private let session: RelayAppModel
    private var cancellables: Set<AnyCancellable> = []
    @Published var isPresentingAddSheet = false
    @Published var editingProfile: Profile?
    @Published private(set) var selectedFilter: ProfilesSidebarFilter

    public init(session: RelayAppModel) {
        self.session = session
        selectedFilter = .all
        bindSession()
    }

    var profiles: [Profile] {
        session.profiles
    }

    var filteredProfiles: [Profile] {
        switch selectedFilter {
        case .all:
            session.profiles
        case .codex:
            session.profiles.filter { $0.agent == .codex }
        }
    }

    var agents: [AgentSettingsDescriptor] {
        AgentSettingsCatalog.supportedAgents
    }

    var selectedProfileId: String? {
        session.selectedProfileId
    }

    var activeProfileId: String? {
        session.activeProfileId
    }

    var selectedProfile: Profile? {
        guard let selectedProfileId else {
            return filteredProfiles.first
        }
        return filteredProfiles.first { $0.id == selectedProfileId }
    }

    var isSwitching: Bool {
        session.isSwitching
    }

    var isMutatingProfiles: Bool {
        session.isMutatingProfiles
    }

    var isLoggingIn: Bool {
        session.isLoggingIn
    }

    var isFetchingEnabledUsage: Bool {
        session.isFetchingEnabledUsage
    }

    var selectedFilterProfileCount: Int {
        filteredProfiles.count
    }

    var selectedFilterEmptyStateDescription: String {
        selectedFilter.emptyStateDescription
    }

    func isFetchingUsage(profileId: String) -> Bool {
        session.isFetchingUsage(profileId: profileId)
    }

    func usageRefreshError(profileId: String) -> String? {
        session.usageRefreshError(profileId: profileId)
    }

    func usageSnapshot(for profileId: String) -> UsageSnapshot? {
        session.usageSnapshot(for: profileId)
    }

    func profileCount(for agent: AgentKind) -> Int {
        session.profiles.filter { $0.agent == agent }.count
    }

    func currentFailureEvents(for profileId: String) -> [FailureEvent] {
        session.currentFailureEventsByProfile[profileId] ?? []
    }

    func selectFilter(_ filter: ProfilesSidebarFilter) {
        guard selectedFilter != filter else {
            return
        }

        selectedFilter = filter
        normalizeSelection()
    }

    func selectProfile(_ profileId: String?) {
        session.selectProfile(profileId)
        normalizeSelection()
    }

    func profileCount(for filter: ProfilesSidebarFilter) -> Int {
        switch filter {
        case .all:
            session.profiles.count
        case .codex:
            session.profiles.filter { $0.agent == .codex }.count
        }
    }

    public func presentAddSheet() {
        guard !session.isMutatingProfiles else {
            return
        }
        isPresentingAddSheet = true
    }

    public func dismissAddSheet() {
        isPresentingAddSheet = false
    }

    public func presentEditForSelectedProfile() {
        guard !session.isMutatingProfiles, let profile = session.selectedProfile else {
            return
        }
        editingProfile = profile
    }

    public func dismissEditSheet() {
        editingProfile = nil
    }

    func addAccount(agent: AgentKind, priority: Int = 100) async -> AddAccountResult {
        await session.addAccount(agent: agent, priority: priority)
    }

    func cancelAddAccount() async {
        await session.cancelLogin()
    }

    func importProfile(agent: AgentKind, nickname: String?, priority: Int) async {
        await session.importProfile(agent: agent, nickname: nickname, priority: priority)
    }

    func editProfile(profileId: String, draft: ProfileDraft) async {
        await session.editProfile(profileId: profileId, draft: draft)
    }

    func removeProfile(_ profileId: String) async {
        await session.removeProfile(profileId)
    }

    func setProfileEnabled(_ profileId: String, enabled: Bool) async {
        await session.setProfileEnabled(profileId, enabled: enabled)
    }

    func switchToProfile(_ profileId: String) async {
        await session.switchToProfile(profileId)
    }

    func refreshUsage(profileId: String) async {
        await session.refreshUsage(profileId: profileId)
    }

    func refreshEnabledUsage() async {
        await session.refreshEnabledUsage()
    }

    func refreshAllUsage() async {
        await session.refreshAllUsage()
    }

    public func refreshIfStale() async {
        await session.refreshIfStale(maxAge: 30)
    }

    private func bindSession() {
        session.$profiles
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.normalizeSelection()
            }
            .store(in: &cancellables)

        session.$selectedProfileId
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.normalizeSelection()
            }
            .store(in: &cancellables)

        session.objectWillChange
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.objectWillChange.send()
            }
            .store(in: &cancellables)
    }

    private func normalizeSelection() {
        guard !filteredProfiles.isEmpty else {
            if session.selectedProfileId != nil {
                session.selectProfile(nil)
            }
            return
        }

        if let selectedProfileId = session.selectedProfileId,
           filteredProfiles.contains(where: { $0.id == selectedProfileId })
        {
            return
        }

        session.selectProfile(filteredProfiles.first?.id)
    }
}
