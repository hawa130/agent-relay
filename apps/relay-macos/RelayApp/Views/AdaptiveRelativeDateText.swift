import SwiftUI

struct AdaptiveRelativeDateText: View {
    enum Style {
        case automatic
        case named
    }

    let prefix: String
    let date: Date
    var style: Style = .automatic

    var body: some View {
        switch style {
        case .automatic:
            Text("\(prefix)\(Text(date, style: .relative))")
        case .named:
            TimelineView(AdaptiveRelativeTimelineSchedule(targetDate: date)) { context in
                Text(prefix + namedRelativeText(relativeTo: context.date))
            }
        }
    }

    private func namedRelativeText(relativeTo referenceDate: Date) -> String {
        Self.namedFormatter.localizedString(for: date, relativeTo: referenceDate)
    }

    private static let namedFormatter: RelativeDateTimeFormatter = {
        let formatter = RelativeDateTimeFormatter()
        formatter.dateTimeStyle = .named
        return formatter
    }()
}

private struct AdaptiveRelativeTimelineSchedule: TimelineSchedule {
    let targetDate: Date

    func entries(from startDate: Date, mode: Mode) -> Entries {
        Entries(currentDate: startDate, targetDate: targetDate)
    }

    struct Entries: Sequence, IteratorProtocol {
        private var currentDate: Date
        private let targetDate: Date

        init(currentDate: Date, targetDate: Date) {
            self.currentDate = currentDate
            self.targetDate = targetDate
        }

        mutating func next() -> Date? {
            let nextDate = currentDate
            currentDate = currentDate.addingTimeInterval(nextInterval(after: currentDate))
            return nextDate
        }

        private func nextInterval(after referenceDate: Date) -> TimeInterval {
            abs(referenceDate.timeIntervalSince(targetDate)) < 60 ? 1 : 60
        }
    }
}
