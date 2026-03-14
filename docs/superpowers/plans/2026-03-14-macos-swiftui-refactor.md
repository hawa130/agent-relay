# macOS SwiftUI Refactor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the macOS SwiftUI UI layer into feature-scoped files and modernize non-functional API, accessibility, and presentation patterns without changing product behavior.

**Architecture:** Keep the current macOS app behavior and RPC surface intact, but decompose oversized files into feature folders and small views. Modernize outdated Swift and SwiftUI usage in the same pass so the new structure lands with current conventions rather than preserving technical debt.

**Tech Stack:** Swift 6.2, SwiftUI, AppKit interop, Swift Package Manager, XCTest, SwiftFormat, SwiftLint

---

## Chunk 1: Lock structure and protect behavior

### Task 1: Add or adjust regression coverage for extracted helpers

**Files:**
- Modify: `apps/relay-macos/Tests/RelayMacOSTests/UsageCardNoteResolverTests.swift`
- Modify: `apps/relay-macos/Tests/RelayMacOSTests/UsageToolbarRefreshScopeResolverTests.swift`
- Modify: `apps/relay-macos/Tests/RelayMacOSTests/MenuBarPresenterTests.swift`

- [ ] Write any missing tests first for helper behavior that will move files.
- [ ] Run `just test-macos` or targeted `swift test --filter` coverage to verify failures if behavior changes are introduced.
- [ ] Keep green tests as the safety net before moving production code.

### Task 2: Split app model companion types out of `FeatureModels.swift`

**Files:**
- Create: `apps/relay-macos/RelayApp/App/SettingsPaneModel.swift`
- Create: `apps/relay-macos/RelayApp/App/ProfilesPaneModel.swift`
- Create: `apps/relay-macos/RelayApp/App/MenuBarPresenter.swift`
- Modify: `apps/relay-macos/RelayApp/App/FeatureModels.swift` or remove it after extraction

- [ ] Move one primary type per file without behavior changes.
- [ ] Keep tests green after extraction.

## Chunk 2: Refactor settings feature structure

### Task 3: Split general settings screen into focused files

**Files:**
- Create: `apps/relay-macos/RelayApp/Settings/General/SettingsPaneView.swift`
- Create: `apps/relay-macos/RelayApp/Settings/General/GeneralSettingsDetailView.swift`
- Create: `apps/relay-macos/RelayApp/Settings/General/AgentSettingsDetailView.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Common/SectionSurfaceCard.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Common/NativePaneScrollView.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Common/NativeDetailRow.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Common/NativeStepperRow.swift`
- Delete: `apps/relay-macos/RelayApp/Settings/SettingsPaneViews.swift`

- [ ] Extract one view per file.
- [ ] Preserve navigation and settings bindings.
- [ ] Re-run settings navigation tests.

## Chunk 3: Refactor profiles feature structure

### Task 4: Split profiles screen into feature-scoped files

**Files:**
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfilesSettingsPaneView.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/UsageAlertSeverity.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/UsageCardNote.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/UsageCardNoteResolver.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/UsageToolbarRefreshScope.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/UsageToolbarRefreshScopeResolver.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileHeroAgentIcon.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileListRow.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileListRowStatusIndicator.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileListUsageLine.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileListAgentLabel.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/UsageMetricRow.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileStateBadge.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileAgentLabel.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileAccountStatusDot.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileCurrentStatusSection.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileStatusBadge.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileInfoBadge.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileEditorMode.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileEditorSheet.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/AddProfileSheet.swift`
- Create: `apps/relay-macos/RelayApp/Settings/Profiles/ProfilesSidebarItemLabel.swift`
- Delete: `apps/relay-macos/RelayApp/Settings/ProfilesSettingsPaneView.swift`

- [ ] Move helpers and leaf views into focused files.
- [ ] Keep existing screen behavior and modal flows unchanged.
- [ ] Preserve test visibility for extracted internal types.

## Chunk 4: Modernize APIs during the extraction

### Task 5: Replace legacy Swift/SwiftUI patterns

**Files:**
- Modify: `apps/relay-macos/RelayApp/Services/RelayDaemonClient.swift`
- Modify: `apps/relay-macos/RelayApp/App/RelayAppModel.swift`
- Modify: `apps/relay-macos/RelayApp/Views/AdaptiveRelativeDateText.swift`
- Modify: `apps/relay-macos/RelayApp/Views/UsageBadgeViews.swift`
- Modify: `apps/relay-macos/RelayApp/MenuBar/MenuBarUsageStyle.swift`
- Modify: `apps/relay-macos/RelayApp/MenuBar/MenuBarUsageProgressBar.swift`
- Modify: `apps/relay-macos/RelayApp/Settings/Profiles/UsageMetricRow.swift`
- Modify: `apps/relay-macos/RelayApp/MenuBar/MenuBarUsageCardSectionView.swift`
- Modify: `apps/relay-macos/RelayApp/Settings/Profiles/ProfilesSettingsPaneView.swift`

- [ ] Replace `Task.sleep(nanoseconds:)` with `Task.sleep(for:)`.
- [ ] Replace delayed GCD work with Swift concurrency.
- [ ] Replace `String(format:)` with modern formatting.
- [ ] Remove `GeometryReader` where `visualEffect()` can do proportional sizing.
- [ ] Fix `.constant(...)` alert presentation state.

### Task 6: Improve accessibility without changing behavior

**Files:**
- Modify: `apps/relay-macos/RelayApp/Settings/Profiles/ProfileAccountStatusDot.swift`
- Modify: `apps/relay-macos/RelayApp/MenuBar/*` as needed

- [ ] Ensure important status is not conveyed by color alone.
- [ ] Preserve or improve accessibility labels for icon-driven controls.

## Chunk 5: Verify and polish

### Task 7: Run formatting, linting, and tests

**Files:**
- Modify: any touched Swift files after formatter/lint adjustments

- [ ] Run `just fmt`.
- [ ] Run `just test-macos`.
- [ ] Run `just lint`.
- [ ] If the workspace remains green, consider `just check` if time allows.

### Task 8: Review and final cleanup

**Files:**
- Review all touched files

- [ ] Confirm no oversized feature files remain in the refactored area.
- [ ] Confirm no old file paths remain duplicated beside new files.
- [ ] Confirm imports and access control remain minimal and correct.
