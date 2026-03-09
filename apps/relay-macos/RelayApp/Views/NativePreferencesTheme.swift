import AppKit
import SwiftUI

public enum NativePreferencesTheme {
    public enum Metrics {
        public static let windowWidth: CGFloat = 940
        public static let windowHeight: CGFloat = 660
        public static let paneHorizontalPadding: CGFloat = 22
        public static let paneVerticalPadding: CGFloat = 18
        public static let sectionSpacing: CGFloat = 18
        public static let sectionContentSpacing: CGFloat = 10
        public static let groupedPadding: CGFloat = 14
        public static let controlSpacing: CGFloat = 8
        public static let sectionCornerRadius: CGFloat = 10
        public static let sidebarWidth: CGFloat = 300
        public static let rowCornerRadius: CGFloat = 9
        public static let usageBarHeight: CGFloat = 7
        public static let detailLabelWidth: CGFloat = 82
    }

    public enum Typography {
        public static let paneTitle = Font.system(size: 23, weight: .semibold, design: .rounded)
        public static let paneSubtitle = Font.system(size: 12)
        public static let sectionLabel = Font.system(size: 11, weight: .semibold)
        public static let body = Font.system(size: 13)
        public static let detail = Font.system(size: 11)
    }

    public enum Colors {
        public static let paneBackground = Color(nsColor: .windowBackgroundColor)
        public static let groupedBackground = Color(nsColor: .controlBackgroundColor)
        public static let sidebarBackground = Color(nsColor: .underPageBackgroundColor)
        public static let rowBackground = Color(nsColor: .controlBackgroundColor)
        public static let mutedText = Color.secondary
        public static let sectionBorder = Color(nsColor: .separatorColor).opacity(0.55)
    }
}
