import SwiftUI

enum UsageCardNoteResolver {
    static func note(usage: UsageSnapshot?, usageRefreshError: String?) -> UsageCardNote? {
        if let note = usage?.userFacingNote {
            return UsageCardNote(
                text: note,
                severity: severity(for: usage))
        }

        guard let error = usageRefreshError?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .nilIfEmpty
        else {
            return nil
        }

        return UsageCardNote(text: error, severity: .warning)
    }

    static func severity(for usage: UsageSnapshot?) -> UsageAlertSeverity? {
        if usage?.remoteError != nil {
            return .warning
        }

        if usage?.stale == true || usage?.source != .webEnhanced {
            return .warning
        }

        return nil
    }

    static func color(for note: UsageCardNote) -> Color {
        guard let severity = note.severity else {
            return .secondary
        }

        switch severity {
        case .warning:
            return NativePreferencesTheme.Colors.statusText(.warning)
        }
    }
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}
