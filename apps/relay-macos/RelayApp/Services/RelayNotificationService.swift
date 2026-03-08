import Foundation
import UserNotifications

actor RelayNotificationService {
    private var requestedAuthorization = false

    func requestAuthorizationIfNeeded() async {
        guard !requestedAuthorization else {
            return
        }

        requestedAuthorization = true
        _ = try? await UNUserNotificationCenter.current().requestAuthorization(
            options: [.alert, .sound]
        )
    }

    func post(title: String, body: String) async {
        await requestAuthorizationIfNeeded()

        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body

        let request = UNNotificationRequest(
            identifier: UUID().uuidString,
            content: content,
            trigger: nil
        )

        try? await UNUserNotificationCenter.current().add(request)
    }
}
