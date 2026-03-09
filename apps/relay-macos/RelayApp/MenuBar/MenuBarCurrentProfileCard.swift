import SwiftUI

struct MenuBarCurrentProfileCard: View {
    let model: MenuBarCurrentCardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            MenuBarUsageCardHeaderView(model: model)

            if hasDetails {
                Divider()
            }

            if model.metrics.isEmpty {
                if let placeholder = model.placeholder {
                    Text(placeholder)
                        .font(.subheadline)
                        .foregroundStyle(Color(nsColor: .secondaryLabelColor))
                }
            } else {
                MenuBarUsageCardSectionView(model: model)
            }
        }
        .padding(.horizontal, 16)
        .padding(.top, 2)
        .padding(.bottom, 2)
        .frame(width: 310, alignment: .leading)
    }

    private var hasDetails: Bool {
        !model.metrics.isEmpty || !model.usageNotes.isEmpty || model.placeholder != nil
    }
}
