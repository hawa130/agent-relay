import XCTest
@testable import RelayMacOSUI

final class MenuBarUsageIconTests: XCTestCase {
    func testDescriptorMapsSessionToOuterRingAndWeeklyToInnerRing() {
        let descriptor = MenuBarUsageIconDescriptor(usage: usageSnapshot())

        XCTAssertEqual(Double(descriptor.sessionRing.progress), 0.54, accuracy: 0.0001)
        XCTAssertEqual(Double(descriptor.weeklyRing.progress), 0.8, accuracy: 0.0001)
        XCTAssertGreaterThan(descriptor.sessionRing.strokeAlpha, descriptor.sessionRing.trackAlpha)
        XCTAssertGreaterThan(descriptor.weeklyRing.strokeAlpha, descriptor.weeklyRing.trackAlpha)
    }

    func testDescriptorDimsStaleUsage() {
        let fresh = MenuBarUsageIconDescriptor(usage: usageSnapshot(stale: false))
        let stale = MenuBarUsageIconDescriptor(usage: usageSnapshot(stale: true))

        XCTAssertLessThan(stale.sessionRing.strokeAlpha, fresh.sessionRing.strokeAlpha)
        XCTAssertLessThan(stale.weeklyRing.trackAlpha, fresh.weeklyRing.trackAlpha)
    }

    func testRendererReturnsTemplateImageSizedForMenuBar() {
        let image = MenuBarUsageIconRenderer.makeImage(usage: nil)

        XCTAssertTrue(image.isTemplate)
        XCTAssertEqual(image.size.width, MenuBarUsageIconRenderer.imageSize.width, accuracy: 0.001)
        XCTAssertEqual(image.size.height, MenuBarUsageIconRenderer.imageSize.height, accuracy: 0.001)
    }

    func testRendererUsesClosedRingsStartingAtTwelveOClock() {
        XCTAssertEqual(Double(MenuBarUsageIconRenderer.startAngle), .pi / 2, accuracy: 0.0001)
        XCTAssertEqual(Double(MenuBarUsageIconRenderer.sweepAngle), .pi * 2, accuracy: 0.0001)
    }

    private func usageSnapshot(stale: Bool = false) -> UsageSnapshot {
        UsageSnapshot(
            profileId: "p1",
            profileName: "Primary",
            source: .webEnhanced,
            confidence: .high,
            stale: stale,
            lastRefreshedAt: Date(timeIntervalSince1970: 100),
            nextResetAt: nil,
            session: UsageWindow(
                usedPercent: 54,
                windowMinutes: 300,
                resetAt: Date(timeIntervalSince1970: 200),
                status: .healthy,
                exact: true
            ),
            weekly: UsageWindow(
                usedPercent: 80,
                windowMinutes: 10_080,
                resetAt: Date(timeIntervalSince1970: 300),
                status: .warning,
                exact: true
            ),
            autoSwitchReason: nil,
            canAutoSwitch: true,
            message: nil,
            remoteError: nil
        )
    }
}
