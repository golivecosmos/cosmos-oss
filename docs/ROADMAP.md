# Cosmos OSS Roadmap

_Last updated: February 2026_

## Near-term (Q1 2026)
1. **Finish repo sanitization**
   - ✅ Remove telemetry, proprietary scripts, Golive endpoints
   - ✅ Rewrite documentation + OSS governance
   - ☐ Wire up CI (lint + cargo test + tauri build) via GitHub Actions
2. **Model UX polish**
   - Auto-detect missing models on first launch and prompt to download
   - Add CLI command to prefetch models (`pnpm tauri run download-models`)
3. **Docs & samples**
   - Add “sample media pack” to help testers verify indexing
   - Record quickstart screencast showing model download + search

## Mid-term (Q2 2026)
1. **Tauri 2 migration**
   - ✅ Port config + runtime wiring to Tauri 2 (`tauri.conf*.json`, capabilities, plugin init).
   - ✅ Replace deprecated `@tauri-apps/api` v1 imports with v2 modules/plugins.
   - ☐ Clean up remaining Rust warnings and finalize DMG packaging workflow in CI.
2. **Plugin infrastructure**
   - Turn the “App Store” into a manifest-driven registry (JSON descriptors + signatures) so the community can publish connectors.
   - Provide an SDK for writing new plugins (commands + UI stubs).
3. **Indexer improvements**
   - Parallel thumbnail generation with cancellation support.
   - Pluggable vector providers (OpenCLIP, local Mistral, custom ONNX).

## Long-term (2H 2026)
1. **Workspace sync**
   - Optional background service that syncs embeddings between machines via user-provided storage (S3, R2, MinIO).
   - End-to-end encryption using the existing SQLCipher utilities.
2. **RAG-friendly API**
   - Expose a local gRPC/WebSocket API for sending search queries from other apps.
   - Document schema + authentication (local tokens).
3. **Ecosystem**
   - Publish verified community plugins each release.
   - Host monthly contributor calls to plan features.

Want to propose something new? Open a GitHub Discussion with the `[proposal]` tag.
