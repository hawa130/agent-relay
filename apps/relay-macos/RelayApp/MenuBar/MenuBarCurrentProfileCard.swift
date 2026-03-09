import SwiftUI

struct MenuBarCurrentProfileCard: View {
    let model: MenuBarCurrentCardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
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
        .padding(.horizontal, 13)
        .padding(.top, 1)
        .padding(.bottom, 1)
        .frame(width: 300, alignment: .leading)
    }

    private var hasDetails: Bool {
        !model.metrics.isEmpty || !model.usageNotes.isEmpty || model.placeholder != nil
    }
}
