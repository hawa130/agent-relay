@testable import RelayMacOSUI
import XCTest

final class AddProfileSheetFlowStateTests: XCTestCase {
    func testRequestingStatePresentsProgressCopy() {
        let state = AddProfileSheetFlowState.requesting

        XCTAssertTrue(state.isRequesting)
        XCTAssertNil(state.primaryActionTitle)
        XCTAssertEqual(state.secondaryActionTitle, "Cancel")
        XCTAssertEqual(state.bodyText, "Complete the browser login, or cancel.")
        XCTAssertEqual(state.statusTitle, "Add Account...")
        XCTAssertEqual(state.statusSubtitle, "Requesting login...")
        XCTAssertNil(state.statusDetail)
        XCTAssertEqual(state.symbolName, "key.fill")
    }

    func testNotSignedInStatePresentsRetryCopy() {
        let state = AddProfileSheetFlowState.notSignedIn(detail: "Sign in from the browser.")

        XCTAssertFalse(state.isRequesting)
        XCTAssertEqual(state.primaryActionTitle, "Try Again")
        XCTAssertEqual(state.secondaryActionTitle, "Back")
        XCTAssertEqual(state.bodyText, "Login did not complete.")
        XCTAssertEqual(state.statusTitle, "Add Account")
        XCTAssertEqual(state.statusSubtitle, "Not signed in")
        XCTAssertEqual(state.statusDetail, "Sign in from the browser.")
        XCTAssertEqual(state.symbolName, "person.crop.circle.badge.xmark")
    }

    func testFailedStateUsesFailureDetailForBodyAndStatus() {
        let state = AddProfileSheetFlowState.failed(detail: "Timed out waiting for login.")

        XCTAssertFalse(state.isRequesting)
        XCTAssertEqual(state.primaryActionTitle, "Try Again")
        XCTAssertEqual(state.secondaryActionTitle, "Back")
        XCTAssertEqual(state.bodyText, "Timed out waiting for login.")
        XCTAssertEqual(state.statusTitle, "Add Account")
        XCTAssertEqual(state.statusSubtitle, "Login failed")
        XCTAssertEqual(state.statusDetail, "Timed out waiting for login.")
        XCTAssertEqual(state.symbolName, "exclamationmark.triangle.fill")
    }
}
