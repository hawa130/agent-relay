enum AddAccountResult: Equatable {
    case success
    case cancelled
    case notSignedIn(detail: String)
    case failed(detail: String)
}
