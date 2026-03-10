import XCTest
@testable import RelayMacOSUI

final class ProfilesNavigationTests: XCTestCase {
    @MainActor
    func testProfilesFilterDefaultsToAll() {
        let model = ProfilesPaneModel(session: RelayAppModel())

        XCTAssertEqual(model.selectedFilter, .all)
        XCTAssertEqual(model.selectedFilterProfileCount, 0)
    }

    @MainActor
    func testProfilesFilterSelectionUpdatesModelState() {
        let model = ProfilesPaneModel(session: RelayAppModel())

        model.selectFilter(ProfilesSidebarFilter.codex)

        XCTAssertEqual(model.selectedFilter, .codex)
        XCTAssertEqual(model.selectedFilterEmptyStateDescription, "No Codex profiles are available in this view yet.")
    }

    @MainActor
    func testProfilesFilterCountsRemainZeroWithoutProfiles() {
        let model = ProfilesPaneModel(session: RelayAppModel())

        XCTAssertEqual(model.profileCount(for: ProfilesSidebarFilter.all), 0)
        XCTAssertEqual(model.profileCount(for: ProfilesSidebarFilter.codex), 0)
    }
}
