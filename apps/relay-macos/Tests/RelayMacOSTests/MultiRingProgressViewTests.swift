import XCTest
@testable import RelayMacOSUI

final class MultiRingProgressViewTests: XCTestCase {
    func testRingLayoutUsesExpectedDiametersForRegularSize() {
        XCTAssertEqual(
            RingProgressLayout.ringDiameter(size: .regular, ringIndex: 0),
            112,
            accuracy: 0.001
        )
        XCTAssertEqual(
            RingProgressLayout.ringDiameter(size: .regular, ringIndex: 1),
            80,
            accuracy: 0.001
        )
        XCTAssertEqual(
            RingProgressLayout.ringDiameter(size: .regular, ringIndex: 2),
            48,
            accuracy: 0.001
        )
        XCTAssertEqual(
            RingProgressLayout.centerDiameter(size: .regular, ringCount: 2),
            50,
            accuracy: 0.001
        )
    }

    func testFocusedItemFallsBackToFirstRing() {
        let items = [
            RingProgressItem(id: "session", label: "Session", shortLabel: "S", progress: 0.54, tone: .positive),
            RingProgressItem(id: "weekly", label: "Weekly", shortLabel: "W", progress: 0.92, tone: .warning),
        ]

        XCTAssertEqual(
            RingProgressLayout.focusedItem(in: items, focusedRingID: nil)?.id,
            "session"
        )
        XCTAssertEqual(
            RingProgressLayout.focusedItem(in: items, focusedRingID: "weekly")?.id,
            "weekly"
        )
        XCTAssertEqual(
            RingProgressLayout.focusedItem(in: items, focusedRingID: "missing")?.id,
            "session"
        )
    }

    func testUsageSnapshotMapsToRingItems() {
        let snapshot = UsageSnapshot(
            profileId: "p1",
            profileName: "Primary",
            source: .webEnhanced,
            confidence: .high,
            stale: true,
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
                usedPercent: nil,
                windowMinutes: nil,
                resetAt: nil,
                status: .unknown,
                exact: false
            ),
            autoSwitchReason: nil,
            canAutoSwitch: true,
            message: nil
        )

        let items = snapshot.ringProgressItems

        XCTAssertEqual(items.map(\.id), ["weekly", "session"])
        XCTAssertEqual(items[0].label, "Weekly")
        XCTAssertEqual(items[0].valueText, "?")
        XCTAssertEqual(items[0].tone, .neutral)
        XCTAssertTrue(items[0].isDimmed)
        XCTAssertEqual(items[0].progress, 0, accuracy: 0.0001)

        XCTAssertEqual(items[1].label, "Session")
        XCTAssertEqual(items[1].valueText, "54%")
        XCTAssertEqual(items[1].tone, .positive)
        XCTAssertEqual(items[1].progress, 0.54, accuracy: 0.0001)
        XCTAssertTrue(items[1].detailText?.hasPrefix("Resets ") ?? false)
    }
}
