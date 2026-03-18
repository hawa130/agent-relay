import SwiftUI

struct NativeDebouncedTextField: View {
    let title: String
    let prompt: String
    @Binding var value: String
    var debounceMilliseconds: Int = 800
    var onCommit: (String) -> Void

    @FocusState private var isFocused: Bool
    @State private var debounceTask: Task<Void, Never>?

    var body: some View {
        TextField(title, text: $value, prompt: Text(prompt))
            .focused($isFocused)
            .onSubmit {
                commit()
            }
            .onChange(of: isFocused) { _, focused in
                if !focused {
                    commit()
                }
            }
            .onChange(of: value) { _, _ in
                scheduleDebounce()
            }
    }

    private func commit() {
        debounceTask?.cancel()
        debounceTask = nil
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return
        }
        onCommit(trimmed)
    }

    private func scheduleDebounce() {
        debounceTask?.cancel()
        let delay = debounceMilliseconds
        debounceTask = Task {
            try? await Task.sleep(for: .milliseconds(delay))
            guard !Task.isCancelled else {
                return
            }
            commit()
        }
    }
}
