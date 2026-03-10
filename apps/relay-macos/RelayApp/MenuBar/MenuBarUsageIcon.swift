import AppKit

struct MenuBarUsageIconDescriptor: Equatable {
    struct Ring: Equatable {
        let progress: CGFloat
        let trackAlpha: CGFloat
        let strokeAlpha: CGFloat
    }

    let sessionRing: Ring
    let weeklyRing: Ring

    init(usage: UsageSnapshot?) {
        if let usage {
            sessionRing = Ring(window: usage.session, stale: usage.stale, emphasis: 1)
            weeklyRing = Ring(window: usage.weekly, stale: usage.stale, emphasis: 0.82)
        } else {
            sessionRing = Ring(progress: 0, trackAlpha: 0.28, strokeAlpha: 0.34)
            weeklyRing = Ring(progress: 0, trackAlpha: 0.22, strokeAlpha: 0.28)
        }
    }
}

private extension MenuBarUsageIconDescriptor.Ring {
    init(window: UsageWindow, stale: Bool, emphasis: CGFloat) {
        let freshnessAlpha: CGFloat = stale ? 0.72 : 1
        let exactnessAlpha: CGFloat = window.exact ? 1 : 0.88
        let statusAlpha: CGFloat

        switch window.status {
        case .healthy:
            statusAlpha = 0.96
        case .warning:
            statusAlpha = 1
        case .exhausted:
            statusAlpha = 1
        case .unknown:
            statusAlpha = 0.58
        }

        let resolvedStrokeAlpha = statusAlpha * freshnessAlpha * exactnessAlpha * emphasis
        self.init(
            progress: CGFloat(window.ringProgress),
            trackAlpha: max(0.14, 0.26 * freshnessAlpha * emphasis),
            strokeAlpha: max(0.2, resolvedStrokeAlpha)
        )
    }
}

enum MenuBarUsageIconRenderer {
    static let imageSize = NSSize(width: 19, height: 19)
    static let startAngle = CGFloat.pi / 2
    static let sweepAngle = 2 * CGFloat.pi

    static func makeImage(usage: UsageSnapshot?) -> NSImage {
        let descriptor = MenuBarUsageIconDescriptor(usage: usage)
        let image = NSImage(size: imageSize, flipped: false) { bounds in
            draw(descriptor: descriptor, in: bounds)
            return true
        }

        image.isTemplate = true
        image.size = imageSize
        return image
    }

    private static func draw(
        descriptor: MenuBarUsageIconDescriptor,
        in bounds: CGRect
    ) {
        guard let context = NSGraphicsContext.current?.cgContext else {
            return
        }

        context.setAllowsAntialiasing(true)
        context.setShouldAntialias(true)

        let center = CGPoint(x: bounds.midX, y: bounds.midY)
        drawRing(
            descriptor.sessionRing,
            center: center,
            radius: 6.55,
            lineWidth: 2.55,
            in: context
        )
        drawRing(
            descriptor.weeklyRing,
            center: center,
            radius: 3.5,
            lineWidth: 2.6,
            in: context
        )
    }

    private static func drawRing(
        _ ring: MenuBarUsageIconDescriptor.Ring,
        center: CGPoint,
        radius: CGFloat,
        lineWidth: CGFloat,
        in context: CGContext
    ) {
        context.setLineWidth(lineWidth)
        context.setLineCap(.round)

        context.setStrokeColor(NSColor.black.withAlphaComponent(ring.trackAlpha).cgColor)
        context.addEllipse(in: CGRect(
            x: center.x - radius,
            y: center.y - radius,
            width: radius * 2,
            height: radius * 2
        ))
        context.strokePath()

        guard ring.progress > 0 else {
            return
        }

        context.setStrokeColor(NSColor.black.withAlphaComponent(ring.strokeAlpha).cgColor)
        if ring.progress >= 0.999 {
            context.addEllipse(in: CGRect(
                x: center.x - radius,
                y: center.y - radius,
                width: radius * 2,
                height: radius * 2
            ))
        } else {
            context.addArc(
                center: center,
                radius: radius,
                startAngle: startAngle,
                endAngle: startAngle - (sweepAngle * ring.progress),
                clockwise: true
            )
        }
        context.strokePath()
    }
}
