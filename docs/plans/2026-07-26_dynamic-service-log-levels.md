# Dynamic Service Log Levels

Date: 2026-07-26
Status: implementation plan for feature 2

## Goal

Add durable, runtime-changeable tracing filters for the deployed `downloader-central`, `downloader-worker`, `downloader-bot`, and `downloader-admin` service types. Operators can independently control console and file `tracing_subscriber::EnvFilter` directives globally and per service type from the admin UI. Settings are persisted in Convex and apply without restarting a process. `downloader-cli` is explicitly excluded.

The feature is service-type scoped, not instance scoped. Every running process of a given type receives the same effective settings.

## User Decisions And Constraints

- Persist one global scope plus one scope for each of `central`, `worker`, `bot`, and `admin`.
- Do not add per-instance settings.
- Console and file filters are independent. Either can inherit while the other overrides.
- Worker and bot propagation may converge through their existing 30 second plus 0-5 second jitter heartbeat cycle, so worst-case normal convergence is approximately 35 seconds.
- Central and admin subscribe directly to Convex and apply changes as soon as the live query emits.
- Central distributes effective settings only to authenticated peers.
- `downloader-cli` remains unchanged.
- Missing or reset dynamic settings restore the filters established by startup CLI/environment configuration, not a hard-coded level.
- Reject invalid directives atomically. Never partially apply a comma-separated filter.
- Append new `CentralProtocol` variants. Postcard encodes enum variants positionally, so inserting variants would renumber all following RPC requests.
- Never edit `crates/app-database/convex/_generated/`.
- Do not modify feature 1/filesize plan files or bot download-limit behavior. Bot changes are restricted to peer RPC/heartbeat logging propagation.

## Current Architecture

### Logger

`crates/app-logger/src/lib.rs` initializes all non-CLI services. It builds separate console and file formatting layers and wraps each layer in a separate `tracing_subscriber::reload::Layer<EnvFilter, _>`. Global `OnceLock<ReloadBridge>` values expose runtime mutation of each filter.

Startup filters are assembled by `build_env_filter` from `LogOptions`, component defaults, `DOWNLOADER_HUB_LOG_LEVEL`, and `DOWNLOADER_HUB_LOG_FILE_LEVEL`. The current `set_log_level` and `set_file_log_level` APIs parse individual directives lossily and rebuild on an `info` baseline. The current `update_*` APIs merge with current/default directives. Neither API can restore the exact startup filter after a dynamic override, and lossy parsing violates strict validation.

### Persistence

Convex schema and functions live under `crates/app-database/convex/`; Rust mirrors and request wrappers live under `crates/app-database/src/`. `Database::watch_query` exposes Convex subscriptions. There is no generic settings table today.

### Central And Peer RPC

`CentralProtocol` and request/response wire types live in `crates/app-peer-comms/src/rpc/`. The protocol uses postcard, so enum order and struct field count are wire contracts. Central authenticates each QUIC connection once, retaining the `AuthedForRole` in its dispatch context. Worker and bot each send a `Heartbeat` every 30 seconds plus 0-5 seconds of jitter.

Central already runs supervised Convex watchers in `bins/downloader-central/src/cmd/central/components/database/mod.rs`. Worker and bot do not connect to Convex and must receive settings through central.

### Admin

`downloader-admin` connects directly to Convex and exposes authenticated axum routes under `/api/admin`. Mutations require `WriteSession`, which rejects read-only operators. Its React SPA uses TanStack Query and Router. There is no settings page or log-level API today.

## Design Decisions

### Stored Model

Create `downloader_hub_log_settings` with at most one row for each scope:

```text
scope: "global" | "central" | "worker" | "bot" | "admin"
console: optional string
file: optional string
```

Index `by_scope` makes each row a singleton. The mutation updates or creates a scope when either value exists and deletes its row when both values are absent. Thus resetting a complete scope leaves no empty row. Each output can still be independently reset by sending `null` for that field while retaining the other field.

No timestamps, actor IDs, instance IDs, or history are required for this feature. Convex is the durable source of truth.

### Resolution

For service `S`, each output is resolved independently:

```text
effective.console = S.console ?? global.console ?? null
effective.file    = S.file    ?? global.file    ?? null
```

`null` means there is no dynamic override and the process restores the exact startup filter captured when `app_logger` initialized. An empty string is not a reset value and is rejected as an invalid filter. This avoids conflating an accidental blank input with inheritance.

The list query returns all persisted scopes in a stable shape suitable for the UI. The Rust client performs the same deterministic resolution so central RPC and direct watchers share one contract.

### Strict EnvFilter Validation And Application

`app_logger` will capture clones of the initial console and file `EnvFilter` values alongside each reload handle. Add a public `LogFilterSettings { console: Option<String>, file: Option<String> }` and an atomic-facing `apply_log_filter_settings` function.

For every present string, parse the entire string with strict `EnvFilter::try_new`; whitespace-only/empty values are rejected. Parse both output filters before mutating either reload handle. If validation fails, neither output changes. Missing values clone the corresponding startup filter. If the first reload succeeds and the second reload unexpectedly fails, restore both prior filters on a best-effort basis and return an error. Normal reload failures are only expected when subscriber state has gone away.

Expose `validate_filter` for the admin HTTP boundary. Existing public setters will also use strict parsing rather than silently dropping malformed directives. Dynamic application replaces the complete filter, which gives operators standard EnvFilter semantics such as `info,app_actions=debug`.

### Convex API

Add `convex/logSettings.ts`:

- `list` query: no args; returns stored rows as `{scope, console?, file?}[]`.
- `set` mutation: args `{scope, console?, file?}`; upserts the singleton row or deletes it if both controls are absent; returns `null`.

Convex validators constrain scope and value types. EnvFilter grammar is Rust-specific, so strict grammar validation happens in the admin Rust backend before mutation and again in every process before application. If invalid data is inserted by an out-of-band Convex caller, watchers/peers retain their last valid filter and log a warning rather than crashing or applying a partial filter.

Add Rust API types:

- `LogSettingsScope`: serde camel-case enum with `Global`, `Central`, `Worker`, `Bot`, `Admin` and string conversion.
- `LogSettings`: `{scope, console: Option<String>, file: Option<String>}`.
- `EffectiveLogSettings`: `{console: Option<String>, file: Option<String>}`.
- `Database::log_settings_list()`.
- `Database::log_settings_watch()`.
- `Database::log_settings_set(scope, console, file)`.
- `resolve_log_settings(rows, service_scope)` to apply per-output fallback.

### RPC Contract

Append, after every existing `CentralProtocol` variant:

```text
GetLogSettings(request::GetLogSettings)
  -> request::LogSettingsResult
```

Wire payloads:

```text
GetLogSettings                     // unit request
LogSettings { console: Option<String>, file: Option<String> }
LogSettingsResult::Ok(LogSettings)
LogSettingsResult::BackendError
```

All fields are always serialized positionally; no skip attributes are allowed. Central derives the requested service scope from the authenticated session role (`worker` or `bot`). Admin peers are rejected/represented as backend/unauthorized only if needed, because admin uses its direct watcher. Central never accepts a caller-provided scope, preventing an authenticated worker from requesting another role's configuration.

Central keeps an in-memory snapshot of the latest list emission in an `ArcSwap`/lock-backed global initialized before accepting RPC. `GetLogSettings` resolves the caller's role from that snapshot without querying Convex per heartbeat. The settings watcher updates the snapshot independently from applying central's own effective settings, so an invalid central-only out-of-band value cannot block otherwise valid worker or bot settings. Every receiving service validates its resolved settings before applying them. If Convex has not emitted yet or its watcher is unavailable, the empty snapshot resolves to no override, preserving peer and central startup filters.

Worker and bot heartbeat loops retain the existing inventory `Heartbeat`, then call `GetLogSettings` and pass successful settings to `app_logger::apply_log_filter_settings`. A failed settings call leaves the current filter untouched. A successful response containing two `None` values restores startup filters. Initial convergence is intentionally allowed to wait for the first heartbeat; reconnect does not require new state.

### Admin HTTP API

Add routes:

```text
GET /api/admin/log-settings
PUT /api/admin/log-settings/:scope
```

GET requires `AdminSession` and returns all five scopes in UI-ready form, including absent fields as `null`. PUT requires `WriteSession`, accepts `{console: string | null, file: string | null}`, normalizes surrounding whitespace, treats `null` as inheritance/reset, rejects blank strings, strictly validates each present EnvFilter, and then calls the Convex mutation. Invalid input returns HTTP 400 with the parser error; database failures return HTTP 500. Read-only admins can view but cannot save.

The admin process separately runs a supervised direct `log_settings_watch`, resolves `admin`, and applies settings. This watcher is not tied to browser sessions or the optional central connection. Invalid out-of-band values are warned and ignored, preserving the last valid runtime filters.

### React UI

Add a `LoggingPage` at `/logging` and a navigation item. The page displays five cards/rows in order: Global, Central, Worker, Bot, Admin. Each has independent console and file directive text inputs.

UI contract:

- Empty input is submitted as `null` and means inherit/reset.
- Global empty means use each process's startup filter.
- Service empty means inherit its corresponding global output, then startup if global is also empty.
- Placeholder/help text shows common EnvFilter examples and explains independent inheritance.
- Each scope saves independently via TanStack Query mutation.
- Successful save invalidates/refetches the settings query.
- Backend validation errors render beside the affected scope without optimistic application.
- Save controls are disabled for read-only admins.
- Layout stacks on mobile and uses the existing card/input/button visual system on desktop.

The TypeScript API adds `LogSettingsScope`, `LogSettings`, `listLogSettings`, and `setLogSettings` contracts.

## Exact File And Symbol Changes

### Documentation

- Add this file: `docs/plans/2026-07-26_dynamic-service-log-levels.md`.

### `app-logger`

- Edit `crates/app-logger/src/lib.rs`.
- Extend `ReloadBridge` to store the startup `EnvFilter`.
- Change `store_reload_handle`/`finish_init` to retain startup console/file filters.
- Add public `LogFilterSettings`.
- Add strict parser/validator and `apply_log_filter_settings`.
- Make existing `set_filter` reject malformed/empty directives rather than use `filter_map` and `parse_lossy`.

### Convex And Rust Database Client

- Edit `crates/app-database/convex/schema.ts` to define the scope validator, settings fields, table, and `by_scope` index.
- Add `crates/app-database/convex/logSettings.ts` with `list` and `set`.
- Add `crates/app-database/src/api/log_settings.rs` with wire/domain types, resolver, query/watch/mutation methods.
- Edit `crates/app-database/src/api/mod.rs` to export `log_settings`.
- Do not edit any `_generated` file.

### Peer Protocol

- Edit `crates/app-peer-comms/src/rpc/request.rs` to add `GetLogSettings`, positional `LogSettings`, and `LogSettingsResult`.
- Edit `crates/app-peer-comms/src/rpc/mod.rs` to append `GetLogSettings` after the current final variant.

### Central

- Edit `bins/downloader-central/src/cmd/central/components/rpc/mod.rs` to initialize/store the settings snapshot, resolve by authenticated role, and dispatch appended `GetLogSettings`.
- Edit `bins/downloader-central/src/cmd/central/components/mod.rs` to initialize the snapshot before peering starts.
- Edit `bins/downloader-central/src/cmd/central/components/database/mod.rs` to spawn the supervised settings watcher, update the RPC snapshot, and apply central's filters.

### Worker

- Edit `bins/downloader-worker/src/cmd/work/rpc.rs` to add `get_log_settings`.
- Edit `bins/downloader-worker/src/cmd/work/app/mod.rs` heartbeat loop to fetch and apply settings after a successful heartbeat.

### Bot

- Edit only peer propagation files: `bins/downloader-bot/src/peering/rpc/mod.rs` and `bins/downloader-bot/src/peering/mod.rs`.
- Add `get_log_settings` and apply successful responses from the heartbeat loop.
- Do not touch download-limit code.

### Admin Backend

- Add `bins/downloader-admin/src/cmd/run/components/log_settings.rs` for the direct watcher.
- Edit `bins/downloader-admin/src/cmd/run/components/mod.rs` to export the component if required.
- Edit `bins/downloader-admin/src/cmd/run/mod.rs` to spawn the supervised watcher independently of HTTP and central connectivity.
- Edit `bins/downloader-admin/src/cmd/run/components/http_api/mod.rs` to register GET/PUT routes.
- Edit `bins/downloader-admin/src/cmd/run/components/http_api/routes.rs` to add list/set handlers and strict validation.

### Admin Frontend

- Edit `bins/downloader-admin/frontend/src/lib/api.ts` with settings types and methods.
- Add `bins/downloader-admin/frontend/src/pages/LoggingPage.tsx`.
- Edit `bins/downloader-admin/frontend/src/router.tsx` and `bins/downloader-admin/frontend/src/pages/Shell.tsx` to register and navigate to `/logging`.
- Rebuild generated `bins/downloader-admin/frontend/dist/` through `bun run build`; do not hand-edit generated assets.

## Rollback, Failure, And Fallback Semantics

- Reset one output: persist that scope without the field (or delete the row when both fields reset). Service override falls back to global; global falls back to startup.
- Reset all dynamic settings: remove all rows. Every direct watcher/peer receives `{console: null, file: null}` and restores its startup filters.
- Convex unavailable at process startup: services retain startup filters. Central/admin watcher supervisors retry. Worker/bot retain startup or their last successfully applied dynamic filters until central can answer again.
- Convex subscription disconnects: no synthetic reset occurs; the last valid applied filter remains while the watcher retries. This avoids an outage unexpectedly changing observability.
- Central snapshot not initialized by a first emission: peer response resolves from an empty list, restoring startup. Snapshot initialization occurs before accepting RPC to avoid a missing-global panic.
- Invalid admin input: no Convex mutation and no runtime change.
- Invalid out-of-band persisted value: process parser rejects the whole console/file pair and retains the prior valid runtime state. The watcher continues for future corrections.
- RPC failure or backend error: heartbeat behavior remains intact and the peer retains its current filters; next heartbeat retries.
- Partial reload failure: attempt to restore both previous filters and return an error. Parsed settings are never intentionally half-applied.
- Old/new rolling deployment: the variant is appended, preserving indexes of all existing requests. New peers calling the appended variant require a new central; during a central-first rollout this is safe. If peers are deployed first against an old central, the added RPC fails and they retain startup/current filters while continuing heartbeats.
- Full feature rollback: stop exposing UI/API and stop watcher/RPC calls first; persisted table rows are inert. The table/functions can be removed in a later Convex deployment after all new binaries are rolled back.

## Phased Implementation

1. Harden `app-logger`: capture startup filters, add strict parsing and atomic pair application.
2. Add Convex schema/functions and Rust settings types/query/watch/mutation/resolution.
3. Append peer RPC wire contracts.
4. Add central settings snapshot, direct watcher/application, and authenticated role-based RPC response.
5. Add worker and bot heartbeat polling/application.
6. Add admin direct watcher, HTTP read/write API, and authorization/validation.
7. Add React settings page, routing/navigation, and rebuild embedded assets.
8. Format, type-check Convex/frontend, and run targeted Rust builds/checks.

## Verification

### Static And Build Checks

- Run `just fmt-dev` for workspace formatting/lints, per repository instructions.
- Run `bun run check` in `crates/app-database` after Convex edits.
- Run `bun run build` in `bins/downloader-admin/frontend` to type-check and regenerate embedded assets.
- Run targeted `just dev-build downloader-central`, `just dev-build downloader-worker`, `just dev-build downloader-bot`, and `just dev-build downloader-admin` if the environment and time permit.
- Confirm no `downloader-cli` file changed.
- Confirm no `convex/_generated` file changed.
- Confirm `GetLogSettings` is the final `CentralProtocol` variant.

### Behavioral Scenarios

1. With no settings rows, start each service with distinct CLI/env console and file directives. Confirm the startup directives remain active.
2. Set global console to `warn` and global file to `info,app_actions=debug`. Confirm central/admin update immediately and worker/bot update on their next heartbeat.
3. Set worker console to `trace` with worker file inherited. Confirm only worker console overrides; worker file keeps the global file filter.
4. Set bot file to `error` with bot console inherited. Confirm independent output resolution.
5. Reset global console while retaining global file. Confirm console returns to each service's startup console setting while file remains dynamic.
6. Reset a service override. Confirm it falls back to global rather than directly to startup.
7. Reset global and all service rows. Confirm exact startup filters are restored without process restart.
8. Submit malformed directives such as `not a directive` or a blank string. Confirm HTTP 400, no persistence change, and no runtime change.
9. Insert malformed data out of band. Confirm watchers log a warning and preserve the last valid filters; correcting the row applies normally.
10. Stop Convex. Confirm existing filters remain and central/admin watcher retry does not crash services.
11. Stop/restart central around a worker/bot heartbeat. Confirm RPC failures do not alter filters and later heartbeats converge.
12. Log in as a read-only admin. Confirm the page is visible and save buttons are disabled; direct mutation attempts receive 403.
13. Verify mobile layout and desktop navigation for `/logging`.

## Acceptance Criteria

- Convex durably stores global and service-type console/file controls.
- Console and file filters resolve and apply independently.
- Central/admin update through direct subscriptions; worker/bot converge through heartbeat polling in approximately 35 seconds.
- No per-instance or CLI behavior is introduced.
- Reset restores startup configuration after global/service inheritance is exhausted.
- Invalid directives cannot be partially persisted through the admin API or partially applied by a process.
- Existing irpc positional indexes remain unchanged because the new request is appended.
- Admin read/write authorization and UI behavior match existing session conventions.
