# Session Handoff: Understanding Layer + HN Launch

## What happened last session

Phase 1 (Clustering Backend) is COMPLETE. All Rust code is written, compiled, and tested (151 tests pass). Changes are uncommitted on main.

**What was built:**
- `clustering_service.rs` — k-means clustering with k-means++ init, random projection (768d->50d) for fast clustering, PCA to 2D for map visualization, TF-IDF cluster naming from text content + directory patterns
- `commands/clustering.rs` — 4 Tauri commands: `compute_clusters`, `get_clusters`, `get_file_positions`, `get_cluster_files`
- Schema tables: `clusters` (id, name, centroid, position, dominant_type, auto_tags, file_count) + `cluster_members` (cluster_id, file_id, file_path, position_x/y, source_type)
- All wired into AppState, main.rs (both debug + release), mod files

**Stitch MCP is configured** with API key, ready to use for UI design.

## What to do next

Read the full plan at `~/.claude/plans/graceful-imagining-rabbit.md` and the CEO plan at `~/.gstack/projects/golivecosmos-cosmos-oss/ceo-plans/2026-04-05-understanding-layer.md`.

### Immediate next steps:

1. **Commit the Phase 1 work** (all uncommitted changes on main)

2. **Phase 0B: Design with Stitch MCP** — Use Stitch MCP tools to design screens:
   - Visual Map view (2D scatter of file clusters, zoom/pan, cluster labels)
   - Cluster card component (mosaic thumbnail, name, count, type badge)
   - Updated AILibrary with grid/map view toggle
   - Sidebar cluster list replacing SmartCollections shell

3. **Phase 2: Visual Map UI** — Build the hero feature:
   - `src/components/VisualMap/VisualMap.tsx` — Canvas-based 2D scatter with zoom/pan
   - `src/components/VisualMap/ClusterCard.tsx` — Cluster card with thumbnail mosaic
   - `src/components/VisualMap/MapControls.tsx` — Zoom, reset, toggle labels
   - `src/hooks/useClusters.ts` — Hook that calls Tauri commands, listens for indexing events to auto-recompute
   - Update `AILibrary.tsx` with grid/map toggle
   - Update `AppLayoutContext.tsx` with cluster state

4. **Phase 3: Progressive Features** (all independent, parallelize):
   - 3A: Search within clusters (ClusterSearchBar)
   - 3B: Smart collections rewrite (cluster-backed)
   - 3C: What's New digest
   - 3D: Settings breakup (1315-line file -> 5 sub-components)
   - 3E: Dark mode polish (OS detection, theme-aware status colors)

5. **Phase 5: Nomic v2 + Knowledge Graph + Wiki Export**

6. **Phase 6: Gemma 4 E2B + RAG Interface**

7. **Phase 4: HN Polish** (README, demo GIF, onboarding, CLAUDE.md)

### Key architecture context:
- Frontend: React 18 + React Router 6 + Tailwind + Radix UI + Framer Motion
- State: React Context (`AppLayoutContext`) + custom hooks, no Redux
- IPC: `invoke()` for commands, `listen()` for events from Rust backend
- Embeddings: 768-dim Nomic v1.5 stored as BLOBs, cosine similarity via sqlite-vec
- The clustering service reads raw embedding BLOBs, averages text chunk embeddings per file, and runs k-means on the result

### No scope cuts — everything ships before HN launch.
