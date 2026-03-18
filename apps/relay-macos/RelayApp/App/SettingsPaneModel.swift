import Combine
import Foundation

@MainActor
public final class SettingsPaneModel: ObservableObject {
    private let session: RelayAppModel
    private var cancellables: Set<AnyCancellable> = []
    @Published private(set) var selectedItem: SettingsSidebarSelection

    public init(session: RelayAppModel) {
        self.session = session
        selectedItem = .general
        bindSession()
    }

    var autoSwitchEnabled: Bool {
        session.autoSwitchEnabled
    }

    var refreshIntervalSeconds: Int {
        session.refreshIntervalSeconds
    }

    var networkQueryConcurrency: Int {
        session.networkQueryConcurrency
    }

    var proxyMode: String {
        session.proxyMode
    }

    var proxyPickerMode: String {
        session.proxyPickerMode
    }

    var proxyCustomUrl: String {
        session.proxyCustomUrl
    }

    var profilesCount: Int {
        session.profiles.count
    }

    var agents: [AgentSettingsDescriptor] {
        AgentSettingsCatalog.supportedAgents
    }

    var codexSettings: CodexSettings? {
        session.codexSettings
    }

    var engineConnectionState: EngineConnectionState {
        session.engineConnectionState
    }

    func selectItem(_ item: SettingsSidebarSelection) {
        guard selectedItem != item else {
            return
        }

        selectedItem = item
    }

    func setAutoSwitch(enabled: Bool) async {
        await session.setAutoSwitch(enabled: enabled)
    }

    func setUsageSourceMode(_ mode: UsageSourceMode) async {
        await session.setCodexUsageSourceMode(mode)
    }

    func setRefreshInterval(seconds: Int) async {
        await session.setRefreshInterval(seconds: seconds)
    }

    func setNetworkQueryConcurrency(_ value: Int) async {
        await session.setNetworkQueryConcurrency(value: value)
    }

    func setProxyMode(_ mode: String) async {
        await session.setProxyMode(mode)
    }

    func restartEngine() async {
        await session.restartEngine()
    }

    public func refreshIfStale() async {
        await session.refreshIfStale(maxAge: 30)
    }

    private func bindSession() {
        session.objectWillChange
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.objectWillChange.send()
            }
            .store(in: &cancellables)
    }
}
