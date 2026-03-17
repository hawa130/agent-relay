import AppKit
import Combine
import SwiftUI

@MainActor
public final class RelayStatusItemController: NSObject, NSMenuDelegate {
    private enum Metrics {
        static let contentWidth: CGFloat = 300
    }

    private let model: RelayAppModel
    private let openWindow: (RelayWindowID) -> Void
    private let statusItem: NSStatusItem
    private let menu: NSMenu
    private let currentCardItem = NSMenuItem()
    private let profilesAnchorItem = NSMenuItem()
    private var profileMenuItems: [NSMenuItem] = []
    private var cancellables: Set<AnyCancellable> = []

    public init(
        model: RelayAppModel,
        openWindow: @escaping (RelayWindowID) -> Void,
        statusBar: NSStatusBar = .system)
    {
        self.model = model
        self.openWindow = openWindow
        statusItem = statusBar.statusItem(withLength: NSStatusItem.squareLength)
        menu = NSMenu()
        super.init()

        menu.autoenablesItems = false
        menu.delegate = self
        statusItem.menu = menu

        configureStatusButton()
        configureMenu()
        observeModel()
    }

    public func menuWillOpen(_ menu: NSMenu) {
        guard menu === self.menu else {
            return
        }

        Task { [weak self] in
            await self?.model.refreshForMenuOpen()
        }
    }

    public func menuDidClose(_ menu: NSMenu) {
        for menuItem in menu.items {
            (menuItem.view as? RelayMenuItemHighlighting)?.setHighlighted(false)
        }

        guard menu === self.menu else {
            return
        }
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
        Publishers.MergeMany(
            model.$usage.map { _ in () }.eraseToAnyPublisher(),
            model.$profiles.map { _ in () }.eraseToAnyPublisher(),
            model.$status.map { _ in () }.eraseToAnyPublisher())
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.updateStatusButton()
            }
            .store(in: &cancellables)

        Publishers.MergeMany(
            model.$status.map { _ in () }.eraseToAnyPublisher(),
            model.$usage.map { _ in () }.eraseToAnyPublisher(),
            model.$lastRefresh.map { _ in () }.eraseToAnyPublisher(),
            model.$isRefreshingUsageList.map { _ in () }.eraseToAnyPublisher())
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.rebuildCurrentCardItem()
            }
            .store(in: &cancellables)

        Publishers.MergeMany(
            model.$profiles.map { _ in () }.eraseToAnyPublisher(),
            model.$usageSnapshots.map { _ in () }.eraseToAnyPublisher(),
            model.$status.map { _ in () }.eraseToAnyPublisher(),
            model.$isSwitching.map { _ in () }.eraseToAnyPublisher())
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.rebuildInlineProfileItems()
            }
            .store(in: &cancellables)
    }

    private func updateStatusButton() {
        guard let button = statusItem.button else {
            return
        }

        button.title = ""
        button.image = statusButtonImage()
        button.toolTip = model.menuBarTitle
    }

    private func statusButtonImage() -> NSImage? {
        MenuBarUsageIconRenderer.makeImage(usage: model.usage)
    }

    private func configureMenu() {
        menu.removeAllItems()

        currentCardItem.isEnabled = false
        menu.addItem(currentCardItem)
        menu.addItem(.separator())
        addProfilesSection(to: menu)
        menu.addItem(.separator())
        addActionItems(to: menu)

        rebuildCurrentCardItem()
        rebuildInlineProfileItems()
    }

    private func rebuildCurrentCardItem() {
        currentCardItem.view = makeHostingView(
            for: MenuBarCurrentProfileCard(session: model),
            width: Metrics.contentWidth)
    }

    private func addProfilesSection(to menu: NSMenu) {
        menu.addItem(makeSectionHeader(title: "Profiles"))

        profilesAnchorItem.isHidden = true
        profilesAnchorItem.isEnabled = false
        menu.addItem(profilesAnchorItem)
    }

    private func rebuildInlineProfileItems() {
        for item in profileMenuItems {
            menu.removeItem(item)
        }
        profileMenuItems.removeAll()

        let anchorIndex = menu.index(of: profilesAnchorItem)
        guard anchorIndex >= 0 else {
            return
        }

        if model.profiles.isEmpty {
            let empty = NSMenuItem(title: "No profiles configured", action: nil, keyEquivalent: "")
            empty.isEnabled = false
            menu.insertItem(empty, at: anchorIndex + 1)
            profileMenuItems.append(empty)
            return
        }

        for (offset, profile) in model.profiles.enumerated() {
            let item = makeProfileMenuItem(profileID: profile.id)
            menu.insertItem(item, at: anchorIndex + 1 + offset)
            profileMenuItems.append(item)
        }
    }

    private func addActionItems(to menu: NSMenu) {
        menu.addItem(makeActionItem(
            title: "Manage Profiles...",
            systemImage: RelayWindowID.profiles.symbol,
            action: #selector(showProfiles)))

        menu.addItem(makeActionItem(
            title: "Settings...",
            systemImage: RelayWindowID.settings.symbol,
            action: #selector(showSettings)))

        let quit = makeActionItem(
            title: "Quit",
            systemImage: "power",
            action: #selector(quitApp))
        quit.keyEquivalent = "q"
        menu.addItem(quit)
    }

    private func makeActionItem(title: String, systemImage: String, action: Selector) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        item.image = menuSymbol(systemImage)
        return item
    }

    private func makeSectionHeader(title: String) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.attributedTitle = NSAttributedString(
            string: title,
            attributes: [
                .font: NSFont.systemFont(ofSize: NSFont.smallSystemFontSize - 1, weight: .semibold),
                .foregroundColor: NSColor.secondaryLabelColor
            ])
        item.isEnabled = false
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

    private func makeHostingView<Content: View>(for view: Content, width: CGFloat) -> RelayMenuHostingView<Content> {
        let hostingView = RelayMenuHostingView(rootView: view)
        let measuredHeight = hostingView.measuredHeight(width: width)
        hostingView.frame = NSRect(x: 0, y: 0, width: width, height: measuredHeight)
        return hostingView
    }

    private func makeProfileMenuItem(profileID: String) -> NSMenuItem {
        let profile = model.profiles.first { $0.id == profileID }
        let isActive = model.activeProfileId == profileID
        let canSelect = (profile?.enabled ?? false) && !model.isSwitching && !isActive
        let highlightState = RelayMenuItemHighlightState()
        let row = RelayMenuItemContainerView(highlightState: highlightState) {
            MenuBarProfilePickerItem(session: model, profileID: profileID)
        }

        let hostingView = RelayInteractiveMenuHostingView(
            rootView: row,
            highlightState: highlightState,
            onClick: canSelect ? { [weak self] in
                self?.selectProfile(id: profileID)
            } : nil)
        let width: CGFloat = Metrics.contentWidth
        let measuredHeight = hostingView.measuredHeight(width: width)
        hostingView.frame = NSRect(x: 0, y: 0, width: width, height: measuredHeight)

        let item = NSMenuItem()
        item.view = hostingView
        item.representedObject = profileID
        item.isEnabled = canSelect
        return item
    }

    private func selectProfile(id profileId: String) {
        menu.cancelTracking()

        Task {
            await model.switchToProfile(profileId)
        }
    }

    @objc private func showSettings() {
        openWindow(.settings)
    }

    @objc private func showProfiles() {
        openWindow(.profiles)
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
        @ViewBuilder content: () -> Content)
    {
        _highlightState = ObservedObject(wrappedValue: highlightState)
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
        onClick: (() -> Void)? = nil)
    {
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
