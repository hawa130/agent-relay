import SwiftUI

struct ResetRelativeDateText: View {
    let date: Date

    var body: some View {
        TimelineView(.everyMinute) { context in
            Text("Resets \(formattedRelativeDescription(relativeTo: context.date))")
        }
    }

    private func formattedRelativeDescription(relativeTo now: Date) -> String {
        let interval = date.timeIntervalSince(now)

        if interval <= 0 {
            return "now"
        }

        let totalMinutes = max(1, Int(ceil(interval / 60)))

        if totalMinutes >= (24 * 60) {
            let totalHours = (totalMinutes + 59) / 60
            let days = totalHours / 24
            let hours = totalHours % 24

            if hours > 0 {
                return "in \(days)d \(hours)h"
            }

            return "in \(days)d"
        }

        let days = totalMinutes / (24 * 60)
        let hours = (totalMinutes % (24 * 60)) / 60
        let minutes = totalMinutes % 60

        var parts: [String] = []
        if days > 0 {
            parts.append("\(days)d")
        }
        if hours > 0 || !parts.isEmpty {
            parts.append("\(hours)h")
        }
        parts.append("\(minutes)m")

        return "in \(parts.joined(separator: " "))"
    }
}
