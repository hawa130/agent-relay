import SwiftUI

struct ProfileDetailUsageCard: View {
    let usage: UsageSnapshot?
    let isFetchingUsage: Bool
    let note: UsageCardNote?
    let onRefresh: () -> Void

    var body: some View {
        SectionSurfaceCard(
            "Usage",
            headerAccessory: {
                UsageRefreshButton(isRefreshing: isFetchingUsage, variant: .card, action: onRefresh)
            },
            content: {
                Group {
                    if let usage {
                        VStack(alignment: .leading, spacing: 10) {
                            if usage.session.status != .unknown {
                                UsageMetricRow(title: "Session", window: usage.session, stale: usage.stale)
                            }
                            if usage.weekly.status != .unknown {
                                UsageMetricRow(title: "Weekly", window: usage.weekly, stale: usage.stale)
                            }

                            if let note {
                                Text(note.text)
                                    .font(NativePreferencesTheme.Typography.detail)
                                    .foregroundStyle(UsageCardNoteResolver.color(for: note))
                                    .frame(maxWidth: .infinity, alignment: .leading)
                            }

                            HStack {
                                Spacer(minLength: 0)

                                VStack(alignment: .trailing, spacing: 2) {
                                    NativeMetaText(text: "Source: \(usage.source.displayName)")
                                    NativeMetaText(text: "Updated: \(usage.lastRefreshedAt.formatted(date: .abbreviated, time: .standard))")
                                }
                            }
                        }
                    } else {
                        VStack(alignment: .leading, spacing: 8) {
                            if isFetchingUsage {
                                HStack(spacing: 8) {
                                    ProgressView()
                                        .controlSize(.small)
                                    Text("Refreshing usage…")
                                        .foregroundStyle(.secondary)
                                }
                            } else if let note {
                                Text(note.text)
                                    .font(NativePreferencesTheme.Typography.detail)
                                    .foregroundStyle(UsageCardNoteResolver.color(for: note))
                            } else {
                                Text("Usage data unavailable.")
                                    .foregroundStyle(.secondary)
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
            })
    }
}
