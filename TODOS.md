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

## P3 — Future

### Model hot-swap for custom GGUF models
- **What:** Let users/contributors swap the Gemma 4 GGUF for any compatible model (Llama, Phi, Mistral) via config.
- **Why:** OSS extensibility. Contributors can experiment with different models.
- **Depends on:** llama-cpp-rs integration being stable.
- **Effort:** S (human) / S (CC)
- **Added:** 2026-04-05 via /plan-ceo-review
