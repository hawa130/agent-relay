import LaunchAtLogin
import SwiftUI

struct GeneralSettingsDetailView: View {
    @ObservedObject var model: SettingsPaneModel
    @State private var proxyPickerSelection: String = "system"
    @State private var customProxyUrl: String = ""
    @State private var proxyInitialized = false

    private var refreshIntervalOptions: [Int] {
        Array(Set([0, 15, 30, 60, 120, 180, 300, 600, 900, model.refreshIntervalSeconds])).sorted()
    }

    private var networkQueryConcurrencyOptions: [Int] {
        Array(Set([1, 2, 4, 6, 8, 10, 12, 16, 24, 32, model.networkQueryConcurrency])).sorted()
    }

    var body: some View {
        Form {
            Section("Behavior") {
                Toggle(
                    "Enable automatic failover",
                    isOn: Binding(
                        get: { model.autoSwitchEnabled },
                        set: { enabled in
                            Task {
                                await model.setAutoSwitch(enabled: enabled)
                            }
                        }))

                LaunchAtLogin.Toggle("Launch at login")

                Picker(
                    "Background refresh",
                    selection: Binding(
                        get: { model.refreshIntervalSeconds },
                        set: { seconds in
                            Task {
                                await model.setRefreshInterval(seconds: seconds)
                            }
                        })) {
                    ForEach(refreshIntervalOptions, id: \.self) { seconds in
                        Text(refreshIntervalLabel(for: seconds)).tag(seconds)
                    }
                }

                Picker(
                    "Network query concurrency",
                    selection: Binding(
                        get: { model.networkQueryConcurrency },
                        set: { value in
                            Task {
                                await model.setNetworkQueryConcurrency(value)
                            }
                        })) {
                    ForEach(networkQueryConcurrencyOptions, id: \.self) { value in
                        Text("\(value)").tag(value)
                    }
                }
            }

            Section("Network") {
                Picker(
                    "Proxy",
                    selection: $proxyPickerSelection) {
                    Text("System Proxy").tag("system")
                    Text("No Proxy").tag("none")
                    Text("Custom").tag("custom")
                }
                .onAppear {
                    syncProxyStateFromModel()
                }
                .onChange(of: model.proxyPickerMode) { _, _ in
                    syncProxyStateFromModel()
                }
                .onChange(of: proxyPickerSelection) { oldValue, newValue in
                    guard proxyInitialized, oldValue != newValue else {
                        return
                    }
                    switch newValue {
                    case "system":
                        Task { await model.setProxyMode("system") }
                    case "none":
                        Task { await model.setProxyMode("none") }
                    case "custom":
                        break
                    default:
                        break
                    }
                }

                if proxyPickerSelection == "custom" {
                    NativeDebouncedTextField(
                        title: "Proxy URL",
                        prompt: "http://127.0.0.1:7890",
                        value: $customProxyUrl) { url in
                        let target = "custom:\(url)"
                        guard target != model.proxyMode else {
                            return
                        }
                        Task { await model.setProxyMode(target) }
                    }
                }
            }

            Section("Engine") {
                NativeDetailRow(title: "Connection", value: engineStateLabel)

                Button("Restart AgentRelay Engine") {
                    Task {
                        await model.restartEngine()
                    }
                }
                .disabled(model.engineConnectionState == .starting)
            }

            Section("Application") {
                NativeDetailRow(title: "Profiles", value: "\(model.profilesCount)")
            }

            Section("About") {
                NativeDetailRow(title: "Version", value: appVersion)
            }
        }
        .formStyle(.grouped)
    }

    private func syncProxyStateFromModel() {
        proxyPickerSelection = model.proxyPickerMode
        customProxyUrl = model.proxyCustomUrl
        proxyInitialized = true
    }

    private func refreshIntervalLabel(for seconds: Int) -> String {
        if seconds == 0 {
            return "Off"
        }

        if seconds < 60 {
            return "\(seconds) sec"
        }

        let minutes = seconds / 60
        return minutes == 1 ? "1 min" : "\(minutes) min"
    }

    private var appVersion: String {
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
        let build = Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String

        switch (version, build) {
        case let (version?, build?) where version != build:
            return "\(version) (\(build))"
        case let (version?, _):
            return version
        case let (_, build?):
            return build
        default:
            return "Development"
        }
    }

    private var engineStateLabel: String {
        switch model.engineConnectionState {
        case .starting:
            "Starting"
        case .ready:
            "Connected"
        case .degraded:
            "Degraded"
        }
    }
}
