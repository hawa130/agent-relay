import AppKit
import Combine
import SwiftUI

@MainActor
public final class RelayStatusItemController: NSObject, NSMenuDelegate {
    private enum Metrics {
        static let contentWidth: CGFloat = 300
    }

    private let model: RelayAppModel
    private let openSettings: () -> Void
    private let statusItem: NSStatusItem
    private let menu: NSMenu
    private var menuIsOpen = false
    private var cancellables: Set<AnyCancellable> = []
    private var presenter: MenuBarPresenter {
        MenuBarPresenter(session: model)
    }

    public init(
        model: RelayAppModel,
        openSettings: @escaping () -> Void,
        statusBar: NSStatusBar = .system
    ) {
        self.model = model
        self.openSettings = openSettings
        self.statusItem = statusBar.statusItem(withLength: NSStatusItem.squareLength)
        self.menu = NSMenu()
        super.init()

        self.menu.autoenablesItems = false
        self.menu.delegate = self
        self.statusItem.menu = self.menu

        configureStatusButton()
        observeModel()
        rebuildMenu()
    }

    public func menuWillOpen(_ menu: NSMenu) {
        guard menu === self.menu else {
            return
        }

        menuIsOpen = true
        rebuildMenu()
    }

    public func menuDidClose(_ menu: NSMenu) {
        for menuItem in menu.items {
            (menuItem.view as? RelayMenuItemHighlighting)?.setHighlighted(false)
        }

        guard menu === self.menu else {
            return
        }

        menuIsOpen = false
    }

    public func menu(_ menu: NSMenu, willHighlight item: NSMenuItem?) {
        for menuItem in menu.items {
            let highlighted = menuItem == item && menuItem.isEnabled
            (menuItem.view as? RelayMenuItemHighlighting)?.setHighlighted(highlighted)
        }
    }

    private func configureStatusButton() {
        guard let button = statusItem.button else {
            return
        }

        button.imagePosition = .imageOnly
        button.imageScaling = .scaleProportionallyDown
        updateStatusButton()
    }

    private func observeModel() {
        model.objectWillChange
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                guard let self else {
                    return
                }
                self.updateStatusButton()
                if self.menuIsOpen {
                    self.rebuildMenu()
                }
            }
            .store(in: &cancellables)
    }

    private func updateStatusButton() {
        guard let button = statusItem.button else {
            return
        }

        button.title = ""
        button.image = statusButtonImage()
        button.toolTip = presenter.title
    }

    private func statusButtonImage() -> NSImage? {
        guard let image = NSImage(systemSymbolName: presenter.symbolName, accessibilityDescription: nil) else {
            return nil
        }

        image.isTemplate = true
        image.size = NSSize(width: 14, height: 14)
        return image
    }

    private func rebuildMenu() {
        menu.removeAllItems()

        addCurrentCard(to: menu)
        menu.addItem(.separator())
        addProfilesMenu(to: menu)
        menu.addItem(.separator())
        addActionItems(to: menu)
    }

    private func addCurrentCard(to menu: NSMenu) {
        if let profile = model.activeProfile {
            let usage = model.usageSnapshot(for: profile.id)
            let card = MenuBarCurrentProfileCard(model: currentCardModel(profile: profile, usage: usage))
            menu.addItem(makeHostingItem(for: card, width: Metrics.contentWidth))
            return
        }

        let item = NSMenuItem(title: "No active profile", action: nil, keyEquivalent: "")
        item.isEnabled = false
        menu.addItem(item)
    }

    private func addProfilesMenu(to menu: NSMenu) {
        let item = NSMenuItem(title: "Profiles", action: nil, keyEquivalent: "")
        item.image = menuSymbol("person.2")
        item.submenu = profilesSubmenu()
        menu.addItem(item)
    }

    private func profilesSubmenu() -> NSMenu {
        let submenu = NSMenu()
        submenu.autoenablesItems = false
        submenu.delegate = self

        if model.profiles.isEmpty {
            let empty = NSMenuItem(title: "No profiles configured", action: nil, keyEquivalent: "")
            empty.isEnabled = false
            submenu.addItem(empty)
            return submenu
        }

        for profile in model.profiles {
            submenu.addItem(makeProfileMenuItem(profile))
        }

        return submenu
    }

    private func addActionItems(to menu: NSMenu) {
        menu.addItem(makeActionItem(
            title: "Refresh",
            systemImage: "arrow.clockwise",
            action: #selector(refreshAll)
        ))

        menu.addItem(makeActionItem(
            title: "Settings...",
            systemImage: "gearshape",
            action: #selector(showSettings)
        ))

        let quit = makeActionItem(
            title: "Quit Relay",
            systemImage: "power",
            action: #selector(quitApp)
        )
        quit.keyEquivalent = "q"
        menu.addItem(quit)
    }

    private func makeActionItem(title: String, systemImage: String, action: Selector) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        item.image = menuSymbol(systemImage)
        return item
    }

    private func menuSymbol(_ systemName: String) -> NSImage? {
        guard let image = NSImage(systemSymbolName: systemName, accessibilityDescription: nil) else {
            return nil
        }

        image.isTemplate = true
        image.size = NSSize(width: 16, height: 16)
        return image
    }

    private func makeHostingItem<Content: View>(for view: Content, width: CGFloat) -> NSMenuItem {
        let hostingView = RelayMenuHostingView(rootView: view)
        let measuredHeight = hostingView.measuredHeight(width: width)
        hostingView.frame = NSRect(x: 0, y: 0, width: width, height: measuredHeight)

        let item = NSMenuItem()
        item.view = hostingView
        item.isEnabled = false
        return item
    }

    private func currentCardModel(profile: Profile, usage: UsageSnapshot?) -> MenuBarCurrentCardModel {
        MenuBarCurrentCardModel(
            providerName: profile.agent.rawValue,
            email: profile.nickname,
            subtitleText: presenter.currentCardSubtitle,
            planText: usage?.source.rawValue,
            metrics: currentMetricRows(usage: usage),
            placeholder: usage == nil ? "No usage yet" : nil,
            usageNotes: presenter.currentCardNotes(usage: usage)
        )
    }

    private func currentMetricRows(usage: UsageSnapshot?) -> [MenuBarMetricRowModel] {
        guard let usage else {
            return []
        }

        return [
            metricRowModel(id: "session", title: "Session", window: usage.session),
            metricRowModel(id: "weekly", title: "Weekly", window: usage.weekly)
        ]
    }

    private func metricRowModel(id: String, title: String, window: UsageWindow) -> MenuBarMetricRowModel {
        MenuBarMetricRowModel(
            id: id,
            title: title,
            percent: window.menuBarProgressPercent,
            percentLabel: "\(window.menuBarDisplayValue) used",
            resetText: window.resetAt.map { "Resets \(preciseResetDescription(for: $0))" },
            detailLeftText: nil,
            detailRightText: nil,
            tint: window.status.menuBarTint
        )
    }

    private func makeProfileMenuItem(_ profile: Profile) -> NSMenuItem {
        let isActive = model.activeProfileId == profile.id
        let canSelect = profile.enabled && !model.isSwitching && !isActive
        let usage = model.usageSnapshot(for: profile.id)
        let highlightState = RelayMenuItemHighlightState()
        let row = RelayMenuItemContainerView(highlightState: highlightState) {
            MenuBarProfilePickerItem(
                profileName: profile.nickname,
                statusText: presenter.profileStatusText(profile: profile, usage: usage, isActive: isActive),
                sessionText: usageText(title: "Session", window: usage?.session),
                sessionResetText: usage?.session.resetAt.map { "Resets \(preciseResetDescription(for: $0))" },
                weeklyText: usageText(title: "Weekly", window: usage?.weekly),
                weeklyResetText: usage?.weekly.resetAt.map { "Resets \(preciseResetDescription(for: $0))" },
                footerText: presenter.profileFooterText(profile: profile, usage: usage),
                symbolName: presenter.profileSymbolName(profile: profile, usage: usage, isActive: isActive),
                isDimmed: !profile.enabled
            )
        }

        let hostingView = RelayInteractiveMenuHostingView(
            rootView: row,
            highlightState: highlightState,
            onClick: canSelect ? { [weak self] in
                self?.selectProfile(id: profile.id)
            } : nil
        )
        let width: CGFloat = Metrics.contentWidth
        let measuredHeight = hostingView.measuredHeight(width: width)
        hostingView.frame = NSRect(x: 0, y: 0, width: width, height: measuredHeight)

        let item = NSMenuItem()
        item.view = hostingView
        item.representedObject = profile.id
        item.isEnabled = canSelect
        return item
    }

    private func usageText(title: String, window: UsageWindow?) -> String? {
        guard let window else {
            return nil
        }

        return "\(title) \(window.menuBarDisplayValue)"
    }

    private func preciseResetDescription(for date: Date) -> String {
        let interval = date.timeIntervalSinceNow

        if interval <= 0 {
            return "now"
        }

        let totalMinutes = max(1, Int(ceil(interval / 60)))
        let days = totalMinutes / (24 * 60)
        let hours = (totalMinutes % (24 * 60)) / 60
        let minutes = totalMinutes % 60

        var parts: [String] = []
        if days > 0 {
            parts.append("\(days)d")
        }
        if hours > 0 || !parts.isEmpty {
            parts.append("\(hours)h")
        }
        parts.append("\(minutes)m")

        return "in \(parts.joined(separator: " "))"
    }

    @objc private func selectProfile(_ sender: NSMenuItem) {
        guard let profileId = sender.representedObject as? String else {
            return
        }

        selectProfile(id: profileId)
    }

    private func selectProfile(id profileId: String) {
        menu.cancelTracking()

        Task {
            await model.switchToProfile(profileId)
        }
    }

    @objc private func refreshAll() {
        Task {
            await model.refreshEnabledUsage()
        }
    }

    @objc private func showSettings() {
        openSettings()
    }

    @objc private func quitApp() {
        NSApplication.shared.terminate(nil)
    }
}

@MainActor
private final class RelayMenuHostingView<Content: View>: NSHostingView<Content> {
    override var allowsVibrancy: Bool {
        true
    }

    func measuredHeight(width: CGFloat) -> CGFloat {
        let controller = NSHostingController(rootView: rootView)
        let measured = controller.sizeThatFits(in: CGSize(width: width, height: .greatestFiniteMagnitude))
        return max(1, ceil(measured.height + 7))
    }
}

@MainActor
private protocol RelayMenuItemHighlighting: AnyObject {
    func setHighlighted(_ highlighted: Bool)
}

@MainActor
private final class RelayMenuItemHighlightState: ObservableObject {
    @Published var isHighlighted = false
}

private struct RelayMenuItemContainerView<Content: View>: View {
    @ObservedObject var highlightState: RelayMenuItemHighlightState
    let content: Content

    init(
        highlightState: RelayMenuItemHighlightState,
        @ViewBuilder content: () -> Content
    ) {
        self._highlightState = ObservedObject(wrappedValue: highlightState)
        self.content = content()
    }

    var body: some View {
        content
            .environment(\.menuItemHighlighted, highlightState.isHighlighted)
            .background(alignment: .topLeading) {
                if highlightState.isHighlighted {
                    RoundedRectangle(cornerRadius: 6, style: .continuous)
                        .fill(MenuBarHighlightStyle.selectionBackground(true))
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                }
            }
    }
}

@MainActor
private final class RelayInteractiveMenuHostingView<Content: View>: NSHostingView<Content>, RelayMenuItemHighlighting {
    private let highlightState: RelayMenuItemHighlightState
    private let onClick: (() -> Void)?

    override var allowsVibrancy: Bool {
        true
    }

    override var intrinsicContentSize: NSSize {
        let size = super.intrinsicContentSize
        guard frame.width > 0 else {
            return size
        }
        return NSSize(width: frame.width, height: size.height)
    }

    init(
        rootView: Content,
        highlightState: RelayMenuItemHighlightState,
        onClick: (() -> Void)? = nil
    ) {
        self.highlightState = highlightState
        self.onClick = onClick
        super.init(rootView: rootView)

        if onClick != nil {
            let recognizer = NSClickGestureRecognizer(target: self, action: #selector(handlePrimaryClick(_:)))
            recognizer.buttonMask = 0x1
            addGestureRecognizer(recognizer)
        }
    }

    @available(*, unavailable)
    required init(rootView: Content) {
        fatalError("init(rootView:) has not been implemented")
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }

    @objc private func handlePrimaryClick(_ recognizer: NSClickGestureRecognizer) {
        guard recognizer.state == .ended else {
            return
        }

        onClick?()
    }

    func measuredHeight(width: CGFloat) -> CGFloat {
        let controller = NSHostingController(rootView: rootView)
        let measured = controller.sizeThatFits(in: CGSize(width: width, height: .greatestFiniteMagnitude))
        return max(1, ceil(measured.height))
    }

    func setHighlighted(_ highlighted: Bool) {
        guard highlightState.isHighlighted != highlighted else {
            return
        }

        highlightState.isHighlighted = highlighted
    }
}
