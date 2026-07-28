# Bot Destination Filesize Limits

## Context

Bot-created URL requests currently copy each platform's configured `max_payload_size` directly into `FileUrl.max_filesize`. The worker propagates that value into downloader options, and yt-dlp can reject oversized source files. Final bot delivery, however, independently reads the bot process's current platform config. This produces several correctness gaps:

- Discord's actual upload limit depends on whether the destination is a DM or a guild and, for guilds, the server premium tier.
- Telegram's official Bot API and a local Bot API server have different upload limits.
- The configured Telegram and Discord payload sizes are operator ceilings, not necessarily the destination's actual API limit.
- A request can outlive a process restart or configuration change, so delivery must reuse the limit selected when the request was created.
- Generic downloads return an unstructured string when the limit is exceeded, so worker terminal max-size handling does not recognize them.
- Music providers create nested generic download requests without forwarding downloader options.
- Fixers may replace or enlarge a downloaded file after downloader enforcement; the worker currently stages that output without a final size check.

Feature 1 makes the effective request limit destination-aware end to end while retaining all existing operator configuration. It deliberately does not change logging infrastructure, feature 2, unrelated request types, or platform batching semantics beyond selecting the correct per-request byte ceiling.

## Current Architecture And Data Flow

### Request creation

Telegram messages enter `cmd::telegram::bot::handlers::message::handle_message`. Discord messages enter `cmd::discord::bot::handlers::message::handle_download_request`. Both handlers:

1. Build a platform-specific `StatusMessage` from the source message.
2. Resolve and upsert account/place metadata.
3. Create one Convex work request per URL through `RpcClient::work_request_create`.
4. Serialize `StatusMessage` into request metadata under `status_message`.
5. Start a supervised per-request watcher.

Each URL is converted to `FileUrl` and currently receives the platform config's `max_payload_size` as `max_filesize`. This field is serialized inside request `info`; no Convex schema change is needed.

### Persistence and recovery

Convex stores request `info` as serialized JSON and stores string metadata in `downloader_hub_requests.metadata`. Bot startup scans and reconnect recovery reconstruct the platform `StatusMessage` from `metadata.status_message`, then resume `watch_and_process`. The status-message JSON is therefore the existing per-request, bot-owned persistence mechanism for delivery context.

### Worker processing

`downloader-worker::process::download_and_fix` converts a URL's extracted entries to `DownloadRequest`s and inserts `url.max_filesize` as downloader option `max-filesize`. The selected downloader receives that option:

- yt-dlp maps it to `--max-filesize` and emits `DownloaderError::ExceedsMaxFilesize` for the recognized yt-dlp result.
- Generic reads the option and stops after streamed bytes exceed it, but currently emits an ordinary string wrapped in `DownloaderError::Error` or `FallibleFailed`.
- Music providers create fresh nested `DownloadRequest`s for their resolved media URLs and currently discard the parent options.

Downloaded paths pass through `app_actions::fix_file`, then every successful fixed path is imported into iroh-blobs. There is currently no size validation between fixing and staging.

### Delivery

After the bot claims the database delivery lease, central returns authoritative blob tickets. `download_and_deliver` downloads all tickets and calls `PlatformDelivery::send_batches`. Telegram and Discord implementations currently group files using their current global `max_payload_size`, not request-persisted state.

## Decisions

### Effective limit formula

For every bot-created URL request:

```text
effective_limit = min(detected_destination_or_platform_limit, configured_platform_ceiling)
```

The existing `telegram-max-payload-size` and `discord-max-payload-size` values remain unchanged and remain operator-controlled hard ceilings. Detection never raises a request above its configured ceiling.

All calculations normalize to non-negative `u64` bytes. Configuration parsing already supplies `Size`; conversion to unsigned bytes is centralized per platform.

### Telegram limits

- Exact official Bot API endpoint: 50,000,000 bytes.
- Configured local Bot API endpoint: 2,000,000,000 bytes.
- Classification uses the existing configured API URL and `TelegramBotConfig::is_api_url_local`, which already defines non-official configured endpoints as local Bot API servers.
- If endpoint classification cannot be represented in a future configuration state, use the official 50,000,000-byte limit as the safe fallback.

The config default remains `50MB`, so local Bot API installations only gain a larger effective limit when the operator explicitly raises the existing ceiling.

### Discord limits

- DMs: 10 MiB.
- Guild premium tier 0 or tier 1: 10 MiB.
- Guild premium tier 2: 50 MiB.
- Guild premium tier 3: 100 MiB.
- Unknown/non-exhaustive premium tiers: 10 MiB.

For guild messages, detection reads Serenity's guild cache first. On a cache miss it calls Discord's guild REST API (`GuildId::to_partial_guild`) and reads `premium_tier`. A failed REST lookup logs a warning and falls back to 10 MiB. DMs do not require an API call. This is conservative and cannot accidentally enqueue a payload larger than a destination known only incompletely.

### Persistence and backward compatibility

Add optional/defaulted `max_filesize: Option<u64>` state to each platform `StatusMessage`. New requests set it before serializing metadata. URL `FileUrl.max_filesize` and status metadata receive the same effective byte value. Final batching reads this persisted status-message value.

Old in-progress rows do not contain the new status field. `#[serde(default)]` keeps those rows deserializable, and delivery falls back to the current configured ceiling capped by the safe platform fallback (Telegram's configured API type or Discord's 10 MiB baseline). This keeps legacy work deliverable without allowing an unknown destination to exceed a conservative platform limit.

No wire protocol struct or Convex schema changes are required. In particular, no positional postcard type is extended, avoiding a rolling-deployment compatibility break.

### Enforcement boundaries

- Source/downloader enforcement remains driven by `FileUrl.max_filesize`.
- Generic converts both known `Content-Length` overflow and streamed overflow into `DownloaderError::ExceedsMaxFilesize`. It checks before writing an overflowing chunk and removes any partial output on overflow where practical.
- Music handler interfaces accept and propagate the parent downloader options into nested generic requests. The archive provider applies the limit to the downloaded archive and the direct provider applies it to media. The worker's final check remains authoritative for extracted/fixed outputs.
- Worker final validation checks each fixed output's metadata before importing it into iroh-blobs. Files larger than the URL request's effective limit are excluded and reported with the same structured max-size wording. If every output is excluded, the request fails rather than being refused for another worker.
- Blob-ticket input requests are unchanged because the requirement is every bot-created URL request and those inputs do not carry a URL max-size field.

## Exact File And Symbol Changes

### Documentation

- `docs/plans/2026-07-26_bot-destination-filesize-limits.md`
  - Standalone architecture, decisions, compatibility, implementation, and verification plan.

### Telegram bot

- `crates/app-config/src/conditional/telegram_bot.rs`
  - Compare parsed URLs in `TelegramBotConfig::is_api_url_local` so URL normalization (the official root URL's trailing slash) does not misclassify the official API as local.
- `bins/downloader-bot/src/cmd/telegram/bot/mod.rs`
  - Add official/local platform upload byte constants.
  - Add `TelegramBot::effective_max_filesize()` returning the minimum of platform API capability and config ceiling.
- `bins/downloader-bot/src/cmd/telegram/bot/helpers/status_message.rs`
  - Add defaulted persisted `max_filesize` state.
  - Add setter/builder and `max_filesize()` accessor with config fallback for legacy metadata.
  - Ensure derived sub-messages retain the field.
- `bins/downloader-bot/src/cmd/telegram/bot/handlers/message/mod.rs`
  - Compute and attach the effective limit before per-URL request creation.
  - Put the exact same limit on every URL's `FileUrl.max_filesize`.
- `bins/downloader-bot/src/cmd/telegram/bot/handlers/delivery.rs`
  - Group final media using `StatusMessage::max_filesize()` instead of the global config.
- `bins/downloader-bot/src/cmd/telegram/bot/helpers/file_group.rs`
  - Use the request-supplied limit for both individual-file and cumulative media-group boundaries instead of consulting global config during grouping.

### Discord bot

- `bins/downloader-bot/src/cmd/discord/bot/discord_bot.rs`
  - Add conservative DM/tier byte constants.
  - Add config ceiling byte conversion and premium-tier-to-limit mapping.
- `bins/downloader-bot/src/cmd/discord/bot/helpers/status_message.rs`
  - Add defaulted persisted `max_filesize` state.
  - Add setter/builder and accessor with config fallback.
  - Preserve it in sub-messages.
- `bins/downloader-bot/src/cmd/discord/bot/handlers/message.rs`
  - Add cache-first/API-second destination limit detection from `Message.guild_id` and `Guild.premium_tier`/`PartialGuild.premium_tier`.
  - Fall back conservatively on cache/API failure.
  - Apply `min(destination, config)` to status metadata and every URL request.
- `bins/downloader-bot/src/cmd/discord/bot/handlers/delivery.rs`
  - Batch against the status message's persisted limit.

### Downloader enforcement

- `crates/app-actions/src/downloaders/handlers/generic.rs`
  - Return structured `DownloaderError` directly from the internal download path.
  - Parse the option once, reject oversized `Content-Length` before file creation, check streamed size before writing, and remove partial files on overflow.
  - Preserve fallible behavior only for non-max-size errors; max-size is always hard/structured.
- `crates/app-actions/src/downloaders/handlers/music/mod.rs`
  - Pass the parent `DownloadRequest` through provider handlers rather than only directory/URL.
- `crates/app-actions/src/downloaders/handlers/music/yams.rs`
  - Build nested generic requests with cloned parent downloader options.
- `crates/app-actions/src/downloaders/handlers/music/spotifydown.rs`
  - Build nested generic requests with cloned parent downloader options.

### Worker final output validation

- `bins/downloader-worker/src/cmd/work/app/process.rs`
  - Retain the URL request's optional max size through `fix_stage_and_deliver`.
  - Validate fixed output metadata before blob import.
  - Record structured max-size errors and fail when no compliant output remains.
  - Keep blob-ticket processing behavior unchanged by passing no limit.

## Edge Cases

- Empty URL lists create no request, unchanged.
- Multiple URLs in one message all receive the same detected destination limit, computed once per incoming message.
- Duplicate URL sorting/deduplication behavior is unchanged.
- Discord guild leaves cache between message receipt and detection: the REST fallback is used.
- Discord API unavailable or forbidden: use 10 MiB and still create requests.
- Discord introduces an unknown premium tier: use 10 MiB until explicitly supported.
- Discord tier changes while work is processing: retain the request-time value for deterministic worker and delivery behavior. If the destination is downgraded later, Discord can still reject delivery; retry behavior remains existing behavior.
- Telegram custom/local endpoint with default config: effective limit remains 50 MB because config is the ceiling.
- Telegram local endpoint with raised config: effective limit can rise to at most 2 GB.
- Configuration changes or bot restart after creation: new requests use new config; existing new-format requests reuse persisted values; legacy rows use current config fallback.
- Negative configured `Size`: existing code expects payload size to be non-negative. Conversion continues to enforce that invariant with a clear expectation.
- Missing/malformed status metadata: existing recovery skips that request; unchanged.
- Missing/invalid downloader max option: no max-size restriction is applied, preserving non-bot and legacy request behavior.
- `Content-Length` absent or dishonest: streaming enforcement remains authoritative.
- Chunk crosses the limit: overflowing bytes are not written.
- Generic partial file cleanup fails: original max-size error remains primary; the worker temp directory lifecycle still cleans artifacts.
- Yams archive is larger than limit while extracted song might be smaller: reject conservatively because the nested source payload itself exceeds the request's allowed download bound.
- A fixer enlarges an otherwise valid input: final worker check excludes it before blob staging.
- Some fixed outputs fit and some do not: deliver compliant files and attach errors for oversized outputs.
- All fixed outputs exceed the limit: terminal request failure with max-size reason; do not cycle through workers.

## Phased Implementation

### Phase 1: Persisted platform limit state

1. Extend Telegram and Discord `StatusMessage` JSON with defaulted optional byte limits.
2. Preserve that state when creating URL sub-status messages.
3. Add accessors that use persisted state and fall back to config for legacy rows.
4. Switch final platform batching to these accessors.

### Phase 2: Destination detection and request creation

1. Implement Telegram official/local capability selection.
2. Implement Discord DM and guild premium-tier mapping.
3. Add Discord cache-first/API-second lookup and conservative fallback.
4. Compute each message's effective limit once.
5. Persist it in status metadata and apply it to every created URL `FileUrl`.

### Phase 3: Downloader closure

1. Make generic overflow errors structured and hard.
2. Add header preflight and pre-write streaming checks.
3. Forward parent options through music provider nested downloads.

### Phase 4: Post-fix validation

1. Carry URL max size into worker fix/stage.
2. stat each fixed output before blob import.
3. exclude/report oversized outputs and fail terminally if no compliant files remain.

### Phase 5: Formatting and verification

1. Run `just fmt-dev` as required by workspace instructions.
2. Run `just dev-build downloader-bot`.
3. Run `just dev-build downloader-worker`.
4. If Convex source is not modified, no `bun run check` is required; if implementation unexpectedly requires Convex edits, run it in `crates/app-database`.
5. Inspect `git diff` and `git status` to ensure no feature-2/logger files or unrelated changes were touched.

## Verification Matrix

### Static/build verification

- Rust formatting and clippy/fix workflow succeeds via `just fmt-dev`.
- Bot builds with Serenity cache and REST premium-tier access.
- Worker and app-actions build with structured generic errors and propagated options.
- Existing status metadata without `max_filesize` still deserializes.

### Request creation checks

- Telegram official API, config 100 MB: request/status limit is 50,000,000 bytes.
- Telegram local API, config 100 MB: request/status limit is 100 MB.
- Telegram local API, config 3 GB: request/status limit is 2,000,000,000 bytes.
- Discord DM, config 100 MiB: request/status limit is 10 MiB.
- Discord tier 0/1, config 100 MiB: 10 MiB.
- Discord tier 2, config 100 MiB: 50 MiB.
- Discord tier 3, config 100 MiB: 100 MiB.
- Discord tier 3, config 25 MiB: 25 MiB.
- Discord unknown/cache miss/API failure: 10 MiB, further capped by config.
- Every URL generated from the same message has `max_filesize: Some(effective_limit)`.

### Worker checks

- Generic `Content-Length` over limit immediately returns `ExceedsMaxFilesize`.
- Generic chunked body crossing limit returns `ExceedsMaxFilesize` without writing the overflowing chunk.
- Worker recognizes generic max-size-only failure as terminal.
- Music nested generic request contains `max-filesize`.
- Fixed output at exactly the limit stages successfully.
- Fixed output one byte over limit is not staged.
- Mixed compliant/oversized fixed outputs stage only compliant files and preserve an error.
- All post-fix outputs oversized cause terminal max-size failure.

### Recovery/delivery checks

- New request recovered after restart batches with its persisted effective limit even if config changed.
- Legacy request metadata without the field batches with the current config capped by the safe platform fallback.
- No request-time destination API lookup is repeated during final delivery.
