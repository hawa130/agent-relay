# macOS SwiftUI Refactor Design

**Goal**

Refactor the macOS SwiftUI control plane into feature-scoped, maintainable files while preserving existing behavior and improving correctness, modern API usage, accessibility, and testability.

## Current Problems

- `apps/relay-macos/RelayApp/Settings/ProfilesSettingsPaneView.swift` combines screen composition, modal flows, formatting helpers, accessibility state, and many leaf views in a single file.
- `apps/relay-macos/RelayApp/Settings/SettingsPaneViews.swift` mixes top-level navigation, detail sections, and shared settings UI primitives.
- `apps/relay-macos/RelayApp/App/FeatureModels.swift` bundles multiple unrelated types.
- Several files still use older Swift and SwiftUI patterns such as `Task.sleep(nanoseconds:)`, `DispatchQueue`, `String(format:)`, `GeometryReader` for proportional fills, `Text` concatenation, and array materialization for `enumerated()`.
- Some accessibility and presentation state is fragile, notably alert binding driven by `.constant(...)` and status affordances that rely too heavily on color alone.

## Refactor Approach

### Module boundaries

Reshape the macOS UI target by feature and responsibility:

- `RelayApp/App/`: long-lived app session models and coordination helpers
- `RelayApp/Settings/Common/`: reusable settings-scaffold views
- `RelayApp/Settings/General/`: settings screen navigation and general/agent detail panes
- `RelayApp/Settings/Profiles/`: profiles screen, sheets, badges, rows, and supporting helpers
- `RelayApp/MenuBar/`: menu bar-only presentation and rendering helpers
- `RelayApp/Views/`: cross-feature generic views only

Each Swift file should hold one primary type or one tightly related small helper group when the helper exists solely to support that primary type.

### Behavioral constraints

- Preserve UI behavior, RPC behavior, commands, and test expectations.
- Do not change product scope or add new features.
- Keep AppKit interop where required for the macOS menu bar and app delegate lifecycle.

### Modernization targets

- Replace `Task.sleep(nanoseconds:)` with `Task.sleep(for:)`.
- Replace delayed GCD work with `Task`-based concurrency.
- Replace `String(format:)` user-facing formatting with `FormatStyle` APIs.
- Replace `GeometryReader` proportional fill code with `visualEffect()`-based sizing where possible.
- Replace `Text` concatenation with interpolation or direct string composition.
- Remove `Array(...)` wrapping around `enumerated()` in `ForEach`.
- Fix alert presentation bindings so dismissal mutates source state.

### Accessibility targets

- Ensure status indicators expose non-color cues where status is important.
- Preserve text labels for icon-only controls via `Label`-based buttons.
- Keep small utility text readable and avoid introducing more fixed tiny text where not necessary.

## Data Flow

- Keep `RelayAppModel`, `SettingsPaneModel`, and `ProfilesPaneModel` behavior stable during this refactor.
- Retain legacy `ObservableObject`/Combine integration where it bridges directly into AppKit status item infrastructure, since replacing it in the same pass would expand risk beyond the current refactor goals.
- Continue moving view-side logic out of large screen files and into focused helper types or presenter-style utilities.

## Testing Strategy

- Keep existing unit tests green throughout the refactor.
- Add focused regression tests only where behavior becomes easier to verify after extraction or where modernized code changes a fragile path.
- Run `just test-macos` after structural milestones, then `just fmt`, `just lint`, and targeted repo verification once refactor work is complete.

## Expected Result

- Feature files become smaller and easier to navigate.
- Existing UI behavior remains intact.
- SwiftUI code aligns more closely with the project conventions and the SwiftUI review references.
