import AppKit

let application = NSApplication.shared
let delegate = RelayAppDelegate()
application.delegate = delegate
_ = NSApplicationMain(CommandLine.argc, CommandLine.unsafeArgv)
