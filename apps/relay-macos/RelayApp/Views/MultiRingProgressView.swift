import SwiftUI

public enum RingProgressTone: Hashable, Sendable {
    case positive
    case warning
    case critical
    case neutral
}

public struct RingProgressItem: Identifiable, Hashable, Sendable {
    public let id: String
    public let label: String
    public let shortLabel: String
    public let progress: Double
    public let tone: RingProgressTone
    public let isDimmed: Bool
    public let valueText: String?
    public let detailText: String?

    public init(
        id: String,
        label: String,
        shortLabel: String,
        progress: Double,
        tone: RingProgressTone,
        isDimmed: Bool = false,
        valueText: String? = nil,
        detailText: String? = nil
    ) {
        self.id = id
        self.label = label
        self.shortLabel = shortLabel
        self.progress = progress
        self.tone = tone
        self.isDimmed = isDimmed
        self.valueText = valueText
        self.detailText = detailText
    }

    var clampedProgress: Double {
        min(max(progress, 0), 1)
    }
}

public struct RingProgressSize {
    public let diameter: CGFloat
    public let ringThickness: CGFloat
    public let ringSpacing: CGFloat
    public let centerPadding: CGFloat

    public init(
        diameter: CGFloat,
        ringThickness: CGFloat,
        ringSpacing: CGFloat,
        centerPadding: CGFloat
    ) {
        self.diameter = diameter
        self.ringThickness = ringThickness
        self.ringSpacing = ringSpacing
        self.centerPadding = centerPadding
    }

    public static var compact: RingProgressSize {
        RingProgressSize(
            diameter: 60,
            ringThickness: 6,
            ringSpacing: 4,
            centerPadding: 6
        )
    }

    public static var mini: RingProgressSize {
        RingProgressSize(
            diameter: 26,
            ringThickness: 3,
            ringSpacing: 1,
            centerPadding: 2
        )
    }

    public static var regular: RingProgressSize {
        RingProgressSize(
            diameter: 112,
            ringThickness: 10,
            ringSpacing: 6,
            centerPadding: 10
        )
    }

    public static var large: RingProgressSize {
        RingProgressSize(
            diameter: 148,
            ringThickness: 14,
            ringSpacing: 8,
            centerPadding: 14
        )
    }
}

public struct RingProgressStyle {
    public let startAngle: Angle
    public let lineCap: CGLineCap
    public let trackColor: Color
    public let trackOpacity: Double
    public let dimmedOpacity: Double
    public let animation: Animation?

    public init(
        startAngle: Angle = .degrees(-90),
        lineCap: CGLineCap = .round,
        trackColor: Color = Color.secondary,
        trackOpacity: Double = 0.14,
        dimmedOpacity: Double = 0.45,
        animation: Animation? = .spring(duration: 0.45, bounce: 0.18)
    ) {
        self.startAngle = startAngle
        self.lineCap = lineCap
        self.trackColor = trackColor
        self.trackOpacity = trackOpacity
        self.dimmedOpacity = dimmedOpacity
        self.animation = animation
    }

    public static var `default`: RingProgressStyle {
        RingProgressStyle()
    }
}

enum RingProgressLayout {
    static func focusedItem(
        in items: [RingProgressItem],
        focusedRingID: String?
    ) -> RingProgressItem? {
        guard !items.isEmpty else {
            return nil
        }

        if let focusedRingID, let match = items.first(where: { $0.id == focusedRingID }) {
            return match
        }

        return items.first
    }

    static func ringDiameter(
        size: RingProgressSize,
        ringIndex: Int
    ) -> CGFloat {
        let step = 2 * (size.ringThickness + size.ringSpacing)
        return max(size.ringThickness, size.diameter - CGFloat(ringIndex) * step)
    }

    static func centerDiameter(
        size: RingProgressSize,
        ringCount: Int
    ) -> CGFloat {
        guard ringCount > 0 else {
            return max(0, size.diameter - (size.centerPadding * 2))
        }

        let innerDiameter = ringDiameter(size: size, ringIndex: ringCount - 1)
        return max(0, innerDiameter - size.ringThickness - (size.centerPadding * 2))
    }
}

public struct MultiRingProgressView<CenterContent: View>: View {
    private let items: [RingProgressItem]
    private let size: RingProgressSize
    private let style: RingProgressStyle
    private let focusedRingID: String?
    private let centerContent: (RingProgressItem?) -> CenterContent

    public init(
        items: [RingProgressItem],
        size: RingProgressSize = .regular,
        style: RingProgressStyle = .default,
        focusedRingID: String? = nil,
        @ViewBuilder centerContent: @escaping (RingProgressItem?) -> CenterContent
    ) {
        self.items = items
        self.size = size
        self.style = style
        self.focusedRingID = focusedRingID
        self.centerContent = centerContent
    }

    public var body: some View {
        ZStack {
            ForEach(Array(items.enumerated()), id: \.offset) { pair in
                let index = pair.offset
                let item = pair.element
                let diameter = RingProgressLayout.ringDiameter(size: size, ringIndex: index)

                Circle()
                    .stroke(
                        style.trackColor.opacity(style.trackOpacity),
                        style: StrokeStyle(lineWidth: size.ringThickness, lineCap: style.lineCap)
                    )
                    .frame(width: diameter, height: diameter)

                Circle()
                    .trim(from: 0, to: item.clampedProgress)
                    .stroke(
                        toneColor(for: item)
                            .opacity(item.isDimmed ? style.dimmedOpacity : 1),
                        style: StrokeStyle(lineWidth: size.ringThickness, lineCap: style.lineCap)
                    )
                    .rotationEffect(style.startAngle)
                    .frame(width: diameter, height: diameter)
            }

            centerContent(focusedItem)
                .frame(
                    width: RingProgressLayout.centerDiameter(size: size, ringCount: items.count),
                    height: RingProgressLayout.centerDiameter(size: size, ringCount: items.count)
                )
        }
        .frame(width: size.diameter, height: size.diameter)
        .animation(style.animation, value: animationState)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityLabel)
    }

    private var focusedItem: RingProgressItem? {
        RingProgressLayout.focusedItem(in: items, focusedRingID: focusedRingID)
    }

    private var animationState: [Double] {
        items.map(\.clampedProgress)
    }

    private var accessibilityLabel: String {
        guard let focusedItem else {
            return "Progress unavailable"
        }

        if let valueText = focusedItem.valueText {
            return "\(focusedItem.label) \(valueText)"
        }

        return focusedItem.label
    }

    private func toneColor(for item: RingProgressItem) -> Color {
        switch item.tone {
        case .positive:
            return NativePreferencesTheme.Colors.usageTint(.healthy)
        case .warning:
            return NativePreferencesTheme.Colors.usageTint(.warning)
        case .critical:
            return NativePreferencesTheme.Colors.usageTint(.exhausted)
        case .neutral:
            return NativePreferencesTheme.Colors.usageTint(.unknown)
        }
    }
}

public extension MultiRingProgressView where CenterContent == DefaultRingProgressCenterContent {
    init(
        items: [RingProgressItem],
        size: RingProgressSize = .regular,
        style: RingProgressStyle = .default,
        focusedRingID: String? = nil
    ) {
        self.init(
            items: items,
            size: size,
            style: style,
            focusedRingID: focusedRingID
        ) { item in
            DefaultRingProgressCenterContent(item: item, size: size)
        }
    }
}

public struct DefaultRingProgressCenterContent: View {
    let item: RingProgressItem?
    let size: RingProgressSize

    public var body: some View {
        VStack(spacing: verticalSpacing) {
            Text(item?.label ?? "Usage")
                .font(.system(size: titleFontSize, weight: .semibold, design: .rounded))
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .minimumScaleFactor(0.8)

            Text(item?.valueText ?? "—")
                .font(.system(size: valueFontSize, weight: .semibold, design: .rounded))
                .lineLimit(1)
                .minimumScaleFactor(0.7)

            if let detailText = item?.detailText {
                Text(detailText)
                    .font(.system(size: detailFontSize))
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var titleFontSize: CGFloat {
        max(10, size.diameter * 0.105)
    }

    private var valueFontSize: CGFloat {
        max(13, size.diameter * 0.18)
    }

    private var detailFontSize: CGFloat {
        max(9, size.diameter * 0.08)
    }

    private var verticalSpacing: CGFloat {
        max(2, size.diameter * 0.03)
    }
}
