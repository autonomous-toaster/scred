# Proxy Blocked Log

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Fix `proxy_resolver::connect_through_proxy` to log status line + target host instead of full response |
| T1.2 | Fix `upstream_connection::connect_through_proxy` to include target host in log message |
| T2.1 | Add access log in `H2MitmHandler::handle_stream` |
| T2.2 | Add access log in `handle_h2_upstream_request` |
| T2.3 | Add access log in `handle_single_request` for HTTP/1.1 upstream path |

## ADDED Requirements

### Requirement: Concise blocked-URL logging (T1.1, T1.2)

T1.1 SHALL complete BEFORE T1.2.

#### Scenario: Corporate proxy rejects CONNECT

- **WHEN** `proxy_resolver::connect_through_proxy` receives a non-200 CONNECT response
- **THEN** it SHALL log a `warn!` message containing the target host, target port, and HTTP status line
- **AND** it SHALL NOT log the full response body

#### Scenario: Forward proxy rejects CONNECT

- **WHEN** `upstream_connection::connect_through_proxy` receives a non-200 CONNECT response
- **THEN** it SHALL include the target host and port in the `warn!` message

### Requirement: Access log for proxied requests (T2.1, T2.2, T2.3)

T2.1 SHALL complete BEFORE T2.2. T2.2 SHALL complete BEFORE T2.3.

#### Scenario: H2 client request

- **WHEN** `H2MitmHandler::handle_stream` processes a request
- **THEN** it SHALL log at `info!` level with format `{method} {uri}`

#### Scenario: HTTP/1.1 request with H2 upstream

- **WHEN** `handle_h2_upstream_request` processes a request
- **THEN** it SHALL log at `info!` level with format `{method} https://{target_host}{path}`

#### Scenario: HTTP/1.1 request with HTTP/1.1 upstream

- **WHEN** `handle_single_request` forwards a request to an HTTP/1.1 upstream
- **THEN** it SHALL log at `info!` level with format `{method} https://{target_host}{path}`
