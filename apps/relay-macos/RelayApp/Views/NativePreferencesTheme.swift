import AppKit
import SwiftUI

public enum NativePreferencesTheme {
    public enum StatusKind {
        case success
        case warning
        case danger
        case info
        case neutral
    }

    public enum Metrics {
        public static let windowWidth: CGFloat = 840
        public static let windowHeight: CGFloat = 600
        public static let paneHorizontalPadding: CGFloat = 18
        public static let paneVerticalPadding: CGFloat = 14
        public static let sectionSpacing: CGFloat = 14
        public static let sectionContentSpacing: CGFloat = 8
        public static let groupedPadding: CGFloat = 12
        public static let controlSpacing: CGFloat = 6
        public static let sectionCornerRadius: CGFloat = 10
        public static let sidebarWidth: CGFloat = 268
        public static let rowCornerRadius: CGFloat = 9
        public static let usageBarHeight: CGFloat = 7
        public static let detailLabelWidth: CGFloat = 74
    }

    public enum Typography {
        public static let paneTitle = Font.system(size: 23, weight: .semibold, design: .rounded)
        public static let paneSubtitle = Font.system(size: 12)
        public static let sectionLabel = Font.system(size: 11, weight: .semibold)
        public static let body = Font.system(size: 13)
        public static let detail = Font.system(size: 11)
        public static let meta = Font.system(size: 10)
        public static let badge = Font.system(size: 10, weight: .semibold)
        public static let badgeValue = Font.system(size: 10, weight: .regular)
    }

    public enum Colors {
        public static let paneBackground = Color(nsColor: .windowBackgroundColor)
        public static let groupedBackground = Color(nsColor: .controlBackgroundColor)
        public static let sidebarBackground = Color(nsColor: .underPageBackgroundColor)
        public static let rowBackground = Color(nsColor: .controlBackgroundColor)
        public static let mutedText = Color.secondary
        public static let sectionBorder = Color(nsColor: .separatorColor).opacity(0.55)
        public static let subtleFill = Color.secondary.opacity(0.12)
        public static let pendingFill = Color.secondary.opacity(0.10)
        public static let progressTrack = Color.secondary.opacity(0.14)
        public static let disabledIndicator = Color.secondary.opacity(0.7)

        static func usageTint(_ status: UsageStatus) -> Color {
            switch status {
            case .healthy:
                .green
            case .warning:
                .orange
            case .exhausted:
                .red
            case .unknown:
                .gray
            }
        }

        static func statusIcon(_ kind: NativePreferencesTheme.StatusKind) -> Color {
            switch kind {
            case .success:
                Color(nsColor: .systemGreen)
            case .warning:
                Color(nsColor: .systemOrange)
            case .danger:
                Color(nsColor: .systemRed)
            case .info:
                Color(nsColor: .systemBlue)
            case .neutral:
                .secondary
            }
        }

        static func statusText(_ kind: NativePreferencesTheme.StatusKind) -> Color {
            switch kind {
            case .success:
                Color(nsColor: .systemGreen)
            case .warning:
                Color(nsColor: .systemOrange)
            case .danger:
                Color(nsColor: .systemRed)
            case .info:
                Color(nsColor: .systemBlue)
            case .neutral:
                .secondary
            }
        }

        static func statusFill(_ kind: NativePreferencesTheme.StatusKind) -> Color {
            switch kind {
            case .success:
                dynamicColor(
                    light: NSColor(red: 0.82, green: 0.92, blue: 0.82, alpha: 1),
                    dark: NSColor(red: 0.16, green: 0.29, blue: 0.18, alpha: 1))
            case .warning:
                dynamicColor(
                    light: NSColor(red: 0.98, green: 0.89, blue: 0.74, alpha: 1),
                    dark: NSColor(red: 0.34, green: 0.24, blue: 0.08, alpha: 1))
            case .danger:
                dynamicColor(
                    light: NSColor(red: 0.97, green: 0.82, blue: 0.82, alpha: 1),
                    dark: NSColor(red: 0.37, green: 0.13, blue: 0.14, alpha: 1))
            case .info:
                dynamicColor(
                    light: NSColor(red: 0.81, green: 0.88, blue: 0.98, alpha: 1),
                    dark: NSColor(red: 0.15, green: 0.24, blue: 0.40, alpha: 1))
            case .neutral:
                dynamicColor(
                    light: NSColor(red: 0.88, green: 0.88, blue: 0.89, alpha: 1),
                    dark: NSColor(red: 0.25, green: 0.25, blue: 0.26, alpha: 1))
            }
        }

        private static func dynamicColor(light: NSColor, dark: NSColor) -> Color {
            Color(
                nsColor: NSColor(name: nil) { appearance in
                    switch appearance.bestMatch(from: [.darkAqua, .aqua]) {
                    case .darkAqua:
                        dark
                    default:
                        light
                    }
                })
        }
    }

    public enum Badge {
        public enum Kind {
            case success
            case warning
            case danger
            case info
            case neutral
        }

        public static func fill(_ kind: Kind) -> Color {
            switch kind {
            case .success:
                dynamicColor(light: NSColor(red: 0.82, green: 0.92, blue: 0.82, alpha: 1), dark: NSColor(red: 0.16, green: 0.29, blue: 0.18, alpha: 1))
            case .warning:
                dynamicColor(light: NSColor(red: 0.98, green: 0.89, blue: 0.74, alpha: 1), dark: NSColor(red: 0.34, green: 0.24, blue: 0.08, alpha: 1))
            case .danger:
                dynamicColor(light: NSColor(red: 0.97, green: 0.82, blue: 0.82, alpha: 1), dark: NSColor(red: 0.37, green: 0.13, blue: 0.14, alpha: 1))
            case .info:
                dynamicColor(light: NSColor(red: 0.81, green: 0.88, blue: 0.98, alpha: 1), dark: NSColor(red: 0.15, green: 0.24, blue: 0.40, alpha: 1))
            case .neutral:
                dynamicColor(light: NSColor(red: 0.88, green: 0.88, blue: 0.89, alpha: 1), dark: NSColor(red: 0.25, green: 0.25, blue: 0.26, alpha: 1))
            }
        }

        public static func text(_ kind: Kind) -> Color {
            switch kind {
            case .success:
                dynamicColor(light: NSColor(red: 0.11, green: 0.38, blue: 0.14, alpha: 1), dark: NSColor(red: 0.69, green: 0.88, blue: 0.70, alpha: 1))
            case .warning:
                dynamicColor(light: NSColor(red: 0.58, green: 0.33, blue: 0.04, alpha: 1), dark: NSColor(red: 0.96, green: 0.78, blue: 0.39, alpha: 1))
            case .danger:
                dynamicColor(light: NSColor(red: 0.63, green: 0.13, blue: 0.15, alpha: 1), dark: NSColor(red: 0.98, green: 0.72, blue: 0.73, alpha: 1))
            case .info:
                dynamicColor(light: NSColor(red: 0.12, green: 0.30, blue: 0.67, alpha: 1), dark: NSColor(red: 0.72, green: 0.82, blue: 1.0, alpha: 1))
            case .neutral:
                dynamicColor(light: NSColor(red: 0.34, green: 0.34, blue: 0.36, alpha: 1), dark: NSColor(red: 0.78, green: 0.78, blue: 0.80, alpha: 1))
            }
        }

        private static func dynamicColor(light: NSColor, dark: NSColor) -> Color {
            Color(
                nsColor: NSColor(name: nil) { appearance in
                    switch appearance.bestMatch(from: [.darkAqua, .aqua]) {
                    case .darkAqua:
                        dark
                    default:
                        light
                    }
                })
        }
    }
}
