import SwiftUI

struct ProfileListRow: View {
    let profile: Profile
    let usage: UsageSnapshot?
    let isActive: Bool
    let isFetchingUsage: Bool
    let usageRefreshError: String?
    var contextMenuModel: ProfileRowContextMenuModel?
    var onMakeCurrent: (() -> Void)?
    var onEdit: (() -> Void)?
    var onToggleEnabled: (() -> Void)?
    var onDelete: (() -> Void)?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            ProfileListAgentLabel(
                agent: profile.agent,
                accountState: profile.accountState)

            HStack(alignment: .top, spacing: 8) {
                if let usage {
                    MultiRingProgressView(
                        items: usage.ringProgressItems,
                        size: .mini)
                    { _ in
                        EmptyView()
                    }
                    .frame(width: 26, height: 26)
                    .padding(.vertical, 6)
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text(profile.nickname)
                        .font(.system(size: 13, weight: .semibold, design: .rounded))
                        .foregroundStyle(profile.enabled ? .primary : .secondary)

                    if let usage {
                        ProfileListUsageLine(
                            title: "Session",
                            value: usage.session.menuBarDisplayValue,
                            resetDate: usage.session.resetAt)

                        ProfileListUsageLine(
                            title: "Weekly",
                            value: usage.weekly.menuBarDisplayValue,
                            resetDate: usage.weekly.resetAt)

                        HStack {
                            Spacer(minLength: 0)

                            HStack(spacing: 4) {
                                if let indicator = statusIndicator {
                                    ProfileListRowStatusIndicator(indicator: indicator)
                                }

                                updatedText(for: usage)
                                    .font(.system(size: 10))
                                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                            }
                        }
                    } else {
                        HStack(spacing: 6) {
                            Text("Waiting for refresh")
                                .font(NativePreferencesTheme.Typography.detail)
                                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)

                            Spacer(minLength: 0)

                            if let indicator = statusIndicator {
                                ProfileListRowStatusIndicator(indicator: indicator)
                            }
                        }
                    }
                }
            }
        }
        .padding(4)
        .frame(maxWidth: .infinity, alignment: .leading)
        .overlay(alignment: .topTrailing) {
            if isActive {
                ProfileStateBadge(title: "Current", kind: .info)
                    .padding(.top, 4)
                    .padding(.trailing, 4)
            }
        }
        .contextMenu {
            if let contextMenuModel {
                Button("Set as Current", action: run(onMakeCurrent))
                    .disabled(!contextMenuModel.canMakeCurrent)

                Button("Edit", action: run(onEdit))
                    .disabled(!contextMenuModel.canEdit)

                Button(contextMenuModel.toggleEnabledTitle, action: run(onToggleEnabled))
                    .disabled(!contextMenuModel.canToggleEnabled)

                Divider()

                Button("Delete", role: .destructive, action: run(onDelete))
                    .disabled(!contextMenuModel.canDelete)
            }
        }
    }

    var statusIndicator: ProfileListRowStatusIndicator.Kind? {
        ProfileListRowStatusIndicator.Kind(
            profile: profile,
            isFetchingUsage: isFetchingUsage,
            usage: usage,
            usageRefreshError: usageRefreshError,
            isStale: usage?.stale == true)
    }

    private func updatedText(for usage: UsageSnapshot) -> some View {
        AdaptiveRelativeDateText(
            prefix: "Updated ",
            date: usage.lastRefreshedAt,
            style: .named)
    }

    private func run(_ action: (() -> Void)?) -> () -> Void {
        { action?() }
    }
}
