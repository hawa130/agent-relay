import Foundation
import SwiftUI
import Defaults

@MainActor
final class RelayAppModel: ObservableObject {
    @Published private(set) var status: StatusReport?
    @Published private(set) var doctor: DoctorReport?
    @Published private(set) var profiles: [Profile] = []
    @Published private(set) var events: [FailureEvent] = []
    @Published private(set) var logTail: LogTail?
    @Published private(set) var diagnosticsExport: DiagnosticsExport?
    @Published private(set) var lastRefresh: Date?
    @Published private(set) var isRefreshing = false
    @Published private(set) var isSwitching = false
    @Published private(set) var isMutatingProfiles = false
    @Published var lastErrorMessage: String?
    private let client = RelayCLIClient()
    private let notificationService = RelayNotificationService()
    private var pollTask: Task<Void, Never>?

    init() {
        Task {
            await notificationService.requestAuthorizationIfNeeded()
            await refresh()
            startPolling()
        }
    }

    deinit {
        pollTask?.cancel()
    }

    var menuBarTitle: String {
        activeProfile?.nickname ?? "Relay"
    }

    var menuBarSymbol: String {
        switch status?.activeState.lastSwitchResult {
        case .success:
            return "bolt.circle.fill"
        case .failed:
            return "exclamationmark.triangle.fill"
        case .notRun, .none:
            return "bolt.circle"
        }
    }

    var activeProfileID: String? {
        status?.activeState.activeProfileID
    }

    var activeProfile: Profile? {
        guard let activeProfileID else {
            return nil
        }
        return profiles.first { $0.id == activeProfileID }
    }

    var autoSwitchEnabled: Bool {
        status?.settings.autoSwitchEnabled ?? false
    }

    func refresh(notifyOnFailure: Bool = false) async {
        guard !isRefreshing else {
            return
        }

        isRefreshing = true
        defer {
            isRefreshing = false
        }

        do {
            async let statusTask = client.fetchStatus()
            async let doctorTask = client.fetchDoctor()
            async let profilesTask = client.fetchProfiles()
            async let eventsTask = client.fetchEvents(limit: 10)
            async let logsTask = client.fetchLogs(lines: 25)

            status = try await statusTask
            doctor = try await doctorTask
            profiles = try await profilesTask
            events = try await eventsTask
            logTail = try await logsTask
            if Defaults[.selectedProfileID] == nil {
                Defaults[.selectedProfileID] = profiles.first?.id
            }
            lastRefresh = Date()
            lastErrorMessage = nil
        } catch {
            lastErrorMessage = error.localizedDescription
            if notifyOnFailure {
                await notificationService.post(
                    title: "Relay refresh failed",
                    body: error.localizedDescription
                )
            }
        }
    }

    func switchToProfile(_ profileID: String) async {
        guard !isSwitching else {
            return
        }

        isSwitching = true
        defer {
            isSwitching = false
        }

        do {
            let report = try await client.switchToProfile(profileID)
            Defaults[.selectedProfileID] = profileID
            await refresh()
            await notificationService.post(
                title: "Relay switched profile",
                body: report.message
            )
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay switch failed",
                body: error.localizedDescription
            )
        }
    }

    func setAutoSwitch(enabled: Bool) async {
        do {
            _ = try await client.setAutoSwitch(enabled: enabled)
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay settings update failed",
                body: error.localizedDescription
            )
        }
    }

    func setProfileEnabled(_ profileID: String, enabled: Bool) async {
        await performProfileMutation { [self] in
            _ = try await self.client.setProfileEnabled(profileID: profileID, enabled: enabled)
        }
    }

    func addProfile(_ draft: ProfileDraft) async {
        await performProfileMutation { [self] in
            _ = try await self.client.addProfile(draft)
        }
    }

    func editProfile(profileID: String, draft: ProfileDraft) async {
        await performProfileMutation { [self] in
            _ = try await self.client.editProfile(profileID: profileID, draft: draft)
        }
    }

    func removeProfile(_ profileID: String) async {
        await performProfileMutation { [self] in
            _ = try await self.client.removeProfile(profileID: profileID)
        }
    }

    func importCodexProfile(nickname: String?, priority: Int) async {
        await performProfileMutation { [self] in
            _ = try await self.client.importCodexProfile(nickname: nickname, priority: priority)
        }
    }

    func exportDiagnostics() async {
        do {
            diagnosticsExport = try await client.exportDiagnostics()
            await refresh()
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay diagnostics export failed",
                body: error.localizedDescription
            )
        }
    }

    private func performProfileMutation(
        _ operation: @escaping @Sendable () async throws -> Void
    ) async {
        guard !isMutatingProfiles else {
            return
        }

        isMutatingProfiles = true
        defer {
            isMutatingProfiles = false
        }

        do {
            try await operation()
            await refresh()
            lastErrorMessage = nil
        } catch {
            lastErrorMessage = error.localizedDescription
            await notificationService.post(
                title: "Relay profile update failed",
                body: error.localizedDescription
            )
        }
    }

    private func startPolling() {
        guard pollTask == nil else {
            return
        }

        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(15))
                guard let self else {
                    break
                }
                await self.refresh()
            }
        }
    }
}
