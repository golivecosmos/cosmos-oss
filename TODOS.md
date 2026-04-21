# Cosmos OSS — Deferred Work

## P1 — High Priority

### SQLite connection pool (r2d2_sqlite)
- **What:** Replace single `Arc<Mutex<Connection>>` in `database_service.rs` with a proper connection pool. Each worker gets its own connection, eliminating mutex contention.
- **Why:** Root cause of DB corruption under heavy indexing load. The hotfix adds `busy_timeout` and poison recovery as a band-aid, but the real fix is eliminating the single-mutex bottleneck.
- **Blocked by:** `r2d2_sqlite` v0.33 requires `rusqlite` v0.39, but project uses v0.37. Need to upgrade `rusqlite` first, which may require `sqlite-vec` compatibility check.
- **Effort:** M (human) / S (CC). Touches `database_service.rs` + every service that calls `get_connection()`.
- **Added:** 2026-04-05 via /plan-eng-review (performance hotfix)

## P2 — Next Major Features

### Replace Whisper with Gemma 4 for transcription
- **What:** Once Gemma 4 (or future version) supports timestamped audio segments, replace Whisper entirely. Reduces model count from 3 to 2.
- **Why:** Simplifies model stack. One fewer runtime dependency (Candle Whisper pipeline).
- **Blocked by:** Gemma 4 lacking timestamp/segment output. TranscriptionDisplay.tsx depends on segmented transcript data.
- **Effort:** M (human) / S (CC)
- **Added:** 2026-04-05 via /plan-ceo-review

### RAG interface — "Ask questions about your files"
- **What:** Chat-like interface for natural language Q&A grounded in indexed files. Gemma 4 E2B + Nomic retrieval = local RAG.
- **Why:** The ultimate personal knowledge base feature.
- **Depends on:** Gemma 4 integration, clustering, and search-within-cluster all working.
- **Effort:** L (human) / M (CC)
- **Added:** 2026-04-05 via /plan-ceo-review

## P2 — HN Readiness Follow-ups

### Failed-state UI for jobs
- **What:** Failed indexing/transcription jobs are invisible outside logs. Add red badge on file cards (Grid/List views), "Retry" in context menu, failed count in sidebar.
- **Why:** Today users think the app is broken when jobs silently fail. After PR1 the backend logs and marks jobs as failed; the UI just doesn't surface it.
- **Effort:** M (human: ~3h) / S (CC)
- **Added:** 2026-04-21 via /plan-eng-review (HN readiness follow-up)

### `main.rs` command-registration dedup
- **What:** Two full `tauri::Builder::default().invoke_handler(...)` blocks — one for `#[cfg(debug_assertions)]`, one for release. Every new command must be added to both manually and they drift.
- **Why:** Every recent PR has had to add the same command twice (most recently `recover_interrupted_jobs` in this plan). A small declarative macro or shared handler list closes the class of bug.
- **Effort:** S (human: ~30min) / S (CC)
- **Added:** 2026-04-21 via /plan-eng-review

### `cluster_members` in `purge_indexed_data_for_file`
- **What:** When Phase 2 clustering wires up (`compute_clusters` / `get_clusters` commands exist in `clustering.rs` per earlier work), `sqlite_service::purge_indexed_data_for_file` must also `DELETE FROM cluster_members WHERE file_path = ?`. Currently the purge covers images, text_chunks, vec_text_chunks, transcriptions only.
- **Why:** Cancel-and-purge contract silently breaks when clustering ships. Cheaper to document now than to chase orphaned cluster rows later.
- **Blocked by:** Phase 2 clustering UI wiring. Table exists, commands exist, no UI consumer yet.
- **Effort:** XS (human: ~15min) / XS (CC)
- **Added:** 2026-04-21 via /plan-eng-review

## P3 — Future

### Model hot-swap for custom GGUF models
- **What:** Let users/contributors swap the Gemma 4 GGUF for any compatible model (Llama, Phi, Mistral) via config.
- **Why:** OSS extensibility. Contributors can experiment with different models.
- **Depends on:** llama-cpp-rs integration being stable.
- **Effort:** S (human) / S (CC)
- **Added:** 2026-04-05 via /plan-ceo-review
