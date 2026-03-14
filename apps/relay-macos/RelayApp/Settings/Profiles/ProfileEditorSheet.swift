import SwiftUI

struct ProfileEditorSheet: View {
    let title: String
    let mode: ProfileEditorMode
    let onSave: @MainActor (ProfileDraft) async -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var draft: ProfileDraft

    init(
        title: String,
        initialDraft: ProfileDraft,
        mode: ProfileEditorMode,
        onSave: @escaping @MainActor (ProfileDraft) async -> Void)
    {
        self.title = title
        self.mode = mode
        self.onSave = onSave
        _draft = State(initialValue: initialDraft)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(title)
                .font(.title3.weight(.semibold))
                .padding(.horizontal, 16)
                .padding(.top, 16)

            Form {
                Section {
                    TextField("Nickname", text: $draft.nickname)
                    VStack(alignment: .leading, spacing: 0) {
                        NativeStepperRow(
                            title: "Priority",
                            valueText: "\(draft.priority)",
                            value: $draft.priority,
                            range: 0 ... 10000)

                        Text("Lower numbers are preferred first during switching.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                } header: {
                    Text("Identity")
                }
            }
            .formStyle(.grouped)
            .frame(maxWidth: .infinity)

            HStack {
                Spacer()

                Button("Cancel") {
                    dismiss()
                }

                Button("Save") {
                    Task {
                        await onSave(normalizedDraft)
                        dismiss()
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(draft.nickname.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            .padding(.horizontal, 16)
            .padding(.bottom, 14)
        }
        .frame(width: 500)
    }

    private var normalizedDraft: ProfileDraft {
        var copy = draft
        copy.nickname = copy.nickname.trimmingCharacters(in: .whitespacesAndNewlines)
        copy.agentHome = copy.agentHome.trimmingCharacters(in: .whitespacesAndNewlines)
        copy.configPath = copy.configPath.trimmingCharacters(in: .whitespacesAndNewlines)

        switch mode {
        case let .edit(profile):
            if copy.agentHome.isEmpty, profile.agentHome != nil {
                copy.clearAgentHome = true
            }
            if copy.configPath.isEmpty, profile.configPath != nil {
                copy.clearConfigPath = true
            }
        }

        return copy
    }
}
