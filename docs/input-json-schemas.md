# `--input-json` Schemas

Every mutation command accepts `--input-json <path>` (or `-` for stdin) to provide
arguments as JSON instead of CLI flags.

## `agrelay codex add --input-json`

```json
{
  "nickname": "string (required)",
  "priority": 100,
  "config_path": "/path/to/config.toml (optional)",
  "agent_home": "/path/to/agent/home (optional)",
  "auth_mode": "ConfigFilesystem | EnvReference | KeychainReference (default: ConfigFilesystem)"
}
```

## `agrelay codex login --input-json`

```json
{
  "nickname": "string (optional)",
  "priority": 100,
  "device_auth": false
}
```

## `agrelay codex import --input-json`

```json
{
  "nickname": "string (optional)",
  "priority": 100
}
```

## `agrelay edit --input-json`

```json
{
  "id": "string (required)",
  "nickname": "string (optional)",
  "priority": 0,
  "config_path": "/path (optional, null to clear)",
  "agent_home": "/path (optional, null to clear)",
  "auth_mode": "ConfigFilesystem | EnvReference | KeychainReference (optional)"
}
```

## `agrelay enable/disable/remove --input-json`

```json
{
  "id": "string (required)"
}
```

## `agrelay settings set --input-json`

```json
{
  "auto_switch_enabled": true,
  "cooldown_seconds": 300,
  "refresh_interval_seconds": 60,
  "network_query_concurrency": 4
}
```

All fields are optional; only provided fields are updated.

## `agrelay autoswitch enable/disable --input-json`

```json
{
  "enabled": true
}
```

## `agrelay codex settings set --input-json`

```json
{
  "source_mode": "Auto | Local | WebEnhanced"
}
```

## `agrelay refresh --input-json`

```json
{
  "id": "string (optional, specific profile)",
  "all": false
}
```

## `agrelay activity events list --input-json`

```json
{
  "limit": 50,
  "profile_id": "string (optional)",
  "reason": "session-exhausted | weekly-exhausted | account-unavailable | auth-invalid | quota-exhausted | rate-limited | command-failed | validation-failed | unknown (optional)"
}
```

## `agrelay activity logs tail --input-json`

```json
{
  "lines": 50
}
```
