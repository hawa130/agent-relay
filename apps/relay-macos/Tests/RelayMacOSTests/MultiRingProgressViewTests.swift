@testable import AgentRelayUI
import XCTest

final class MultiRingProgressViewTests: XCTestCase {
    func testAccessibilitySummaryUsesAllRingValuesWhenPresent() {
        let items = [
            RingProgressItem(
                id: "session",
                label: "Session",
                shortLabel: "S",
                progress: 0.54,
                tone: .positive,
                valueText: "54%",
                detailText: "Resets in 2h"),
            RingProgressItem(
                id: "weekly",
                label: "Weekly",
                shortLabel: "W",
                progress: 0.92,
                tone: .warning,
                valueText: "92%",
                detailText: "Resets tomorrow")
        ]

        XCTAssertEqual(
            MultiRingProgressAccessibility.summary(for: items),
            "Session 54%, Resets in 2h; Weekly 92%, Resets tomorrow")
    }

    func testAccessibilitySummaryFallsBackWhenNoItemsExist() {
        XCTAssertEqual(
            MultiRingProgressAccessibility.summary(for: []),
            "Progress unavailable")
    }

    func testRingLayoutUsesExpectedDiametersForRegularSize() {
        XCTAssertEqual(
            RingProgressLayout.ringDiameter(size: .regular, ringIndex: 0),
            112,
            accuracy: 0.001)
        XCTAssertEqual(
            RingProgressLayout.ringDiameter(size: .regular, ringIndex: 1),
            80,
            accuracy: 0.001)
        XCTAssertEqual(
            RingProgressLayout.ringDiameter(size: .regular, ringIndex: 2),
            48,
            accuracy: 0.001)
        XCTAssertEqual(
            RingProgressLayout.centerDiameter(size: .regular, ringCount: 2),
            50,
            accuracy: 0.001)
    }

    func testFocusedItemFallsBackToFirstRing() {
        let items = [
            RingProgressItem(id: "session", label: "Session", shortLabel: "S", progress: 0.54, tone: .positive),
            RingProgressItem(id: "weekly", label: "Weekly", shortLabel: "W", progress: 0.92, tone: .warning)
        ]

        XCTAssertEqual(
            RingProgressLayout.focusedItem(in: items, focusedRingID: nil)?.id,
            "session")
        XCTAssertEqual(
            RingProgressLayout.focusedItem(in: items, focusedRingID: "weekly")?.id,
            "weekly")
        XCTAssertEqual(
            RingProgressLayout.focusedItem(in: items, focusedRingID: "missing")?.id,
            "session")
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
                exact: true),
            weekly: UsageWindow(
                usedPercent: nil,
                windowMinutes: nil,
                resetAt: nil,
                status: .unknown,
                exact: false),
            autoSwitchReason: nil,
            canAutoSwitch: true,
            message: nil,
            remoteError: nil)

        let items = snapshot.ringProgressItems

        XCTAssertEqual(items.map(\.id), ["session", "weekly"])
        XCTAssertEqual(items[0].label, "Session")
        XCTAssertEqual(items[0].valueText, "54%")
        XCTAssertEqual(items[0].tone, .positive)
        XCTAssertTrue(items[0].isDimmed)
        XCTAssertEqual(items[0].progress, 0.54, accuracy: 0.0001)
        XCTAssertTrue(items[0].detailText?.hasPrefix("Resets ") ?? false)

        XCTAssertEqual(items[1].label, "Weekly")
        XCTAssertEqual(items[1].valueText, "?")
        XCTAssertEqual(items[1].tone, .neutral)
        XCTAssertEqual(items[1].progress, 0, accuracy: 0.0001)
    }
}
