import SwiftUI

struct NativeStepperRow: View {
    let title: String
    let valueText: String
    @Binding var value: Int
    let range: ClosedRange<Int>
    var step: Int = 1

    var body: some View {
        LabeledContent {
            HStack(spacing: 10) {
                Text(valueText)
                    .monospacedDigit()
                Stepper("", value: $value, in: range, step: step)
                    .labelsHidden()
                    .accessibilityLabel(title)
                    .accessibilityValue(Self.accessibilityValueText(valueText))
            }
        } label: {
            Text(title)
        }
    }

    nonisolated static func accessibilityValueText(_ valueText: String) -> String {
        valueText
    }
}
