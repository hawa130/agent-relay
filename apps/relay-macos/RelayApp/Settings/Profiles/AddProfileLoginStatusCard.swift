import SwiftUI

struct AddProfileLoginStatusCard: View {
    let flowState: AddProfileSheetFlowState

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Group {
                if flowState.isRequesting {
                    ProgressView()
                        .controlSize(.small)
                        .tint(flowState.accentColor)
                        .frame(width: 22, height: 22)
                } else {
                    Image(systemName: flowState.symbolName)
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundStyle(flowState.accentColor)
                        .frame(width: 22, height: 22)
                }
            }

            VStack(alignment: .leading, spacing: 4) {
                Text(flowState.statusTitle)
                    .font(.headline)

                Text(flowState.statusSubtitle)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                if let detail = flowState.statusDetail {
                    Text(detail)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            }

            Spacer(minLength: 0)
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(flowState.backgroundColor))
    }
}
