# Proxy Config Cache

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Cache `MitmConfig` in `get_proxy_url()` using `OnceLock` |

## ADDED Requirements

### Requirement: Cache proxy config to avoid repeated logging (T1.1)

T1.1 SHALL complete ALWAYS.

#### Scenario: First call to get_proxy_url

- **WHEN** `get_proxy_url` is called for the first time
- **THEN** it SHALL create a `MitmConfig` via `from_env()`
- **AND** it SHALL cache the config for subsequent calls

#### Scenario: Subsequent calls to get_proxy_url

- **WHEN** `get_proxy_url` is called again
- **THEN** it SHALL return the cached config
- **AND** it SHALL NOT call `from_env()` again
- **AND** it SHALL NOT log proxy environment variables again
