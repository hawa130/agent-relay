enum MultiRingProgressAccessibility {
    static func summary(for items: [RingProgressItem]) -> String {
        let parts = items.map(summaryPart(for:))
            .filter { !$0.isEmpty }

        guard !parts.isEmpty else {
            return "Progress unavailable"
        }

        return parts.joined(separator: "; ")
    }

    private static func summaryPart(for item: RingProgressItem) -> String {
        var summary = item.label

        if let valueText = item.valueText, !valueText.isEmpty {
            summary += " \(valueText)"
        }

        if let detailText = item.detailText, !detailText.isEmpty {
            summary += ", \(detailText)"
        }

        return summary
    }
}
