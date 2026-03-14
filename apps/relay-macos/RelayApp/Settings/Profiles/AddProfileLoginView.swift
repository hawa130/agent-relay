import SwiftUI

struct AddProfileLoginView: View {
    let loginTitle: String
    let flowState: AddProfileSheetFlowState
    let isBusy: Bool
    let onSecondaryAction: () -> Void
    let onPrimaryAction: (() -> Void)?

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(loginTitle)
                .font(.title3.weight(.semibold))
                .padding(.horizontal, 16)
                .padding(.top, 16)

            AddProfileLoginStatusCard(flowState: flowState)
                .padding(.horizontal, 16)

            Text(flowState.bodyText)
                .font(.body)
                .foregroundStyle(.secondary)
                .padding(.horizontal, 16)
                .frame(maxWidth: .infinity, alignment: .leading)

            HStack {
                Spacer()

                Button(flowState.secondaryActionTitle) {
                    onSecondaryAction()
                }

                if let primaryActionTitle = flowState.primaryActionTitle,
                   let onPrimaryAction
                {
                    Button(primaryActionTitle) {
                        onPrimaryAction()
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(isBusy || flowState.isRequesting)
                }
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 14)
        }
    }
}
