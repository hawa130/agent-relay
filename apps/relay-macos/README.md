# Relay for macOS

This directory is reserved for the native macOS menu bar app.

V1 rule:

- The app is a control plane only.
- All real profile, switch, validation, and diagnostics operations must go through `relay` CLI JSON commands.
- No UI code should directly mutate Codex configuration files.

Planned structure:

```text
RelayApp/
  App/
  MenuBar/
  Settings/
  Services/
  Models/
  Resources/
```

