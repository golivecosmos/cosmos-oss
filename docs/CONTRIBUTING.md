# Contributing Guide

Thanks for helping Cosmos OSS! This document describes expectations for issues, pull requests, style, and testing.

If you only need to run the app (not contribute code), use the 3 setup options in [`README.md`](../README.md): GitHub DMG, website package, or build from source.

## 1. Ground rules
- Respect the [Code of Conduct](CODE_OF_CONDUCT.md).
- Discuss large/controversial features in GitHub Discussions before opening a PR.
- Keep the codebase telemetry-free; new network calls must be opt-in and documented.

## 2. Development workflow
```bash
git clone https://github.com/cosmos-oss/cosmos-oss.git
cd cosmos-oss
pnpm install
pnpm dev  # launches Vite + Tauri dev shell
```
Use the Quick Menu → Manage Models to fetch models the first time.

### Recommended tools
- Node 20 + pnpm 9 (managed by Corepack)
- Rust stable (1.78+) via rustup
- VS Code + Rust Analyzer / TypeScript language service

## 3. Coding standards
| Area | Guidelines |
| ---- | ---------- |
| TypeScript/React | Prefer functional components + hooks, ES modules, `@/` aliases. Keep components small and colocate feature hooks under `src/hooks/`. |
| Styling | Tailwind CSS + `tailwind-merge`. Avoid inline styles unless dynamic. |
| Rust | Clippy clean (`cargo clippy --all-targets`). Use `anyhow` for fallible service logic and `thiserror` for typed errors. |
| Logging | Use `app_log_info!/warn!/error!` macros so logs surface in the UI log viewer. |
| Secrets | Never check in API keys, certs, or release credentials. |

## 4. Testing & linting
| Command | Purpose |
| ------- | ------- |
| `pnpm lint` | TypeScript type/lint gate (`tsc --noEmit`) |
| `pnpm tauri build --debug` | Smoke-test Rust + bundling |
| `cd src-tauri && cargo test` | Backend unit/integration tests |
| Manual | Use the Quick Menu to download models and index the sample folders referenced in the PR |

PRs must include notes on how you tested (manual steps + commands). If you add a new command/service, add a corresponding test in `src-tauri/src/services/tests/` or `src-tauri/src/commands/tests/`.

## 5. Submitting pull requests
1. Fork the repo (or create a feature branch in `cosmos-oss` if you have write access).
2. Keep commits focused; avoid mixing refactors with functional changes.
3. Update docs when behavior changes (README, BUILDING, or inline Rust doc comments).
4. Ensure `pnpm lint` and `cargo test` pass locally. We’ll add CI soon, but manual diligence matters now.
5. Fill out the PR template:
   - Summary of change
   - Testing performed
   - Screenshots (if UI)
   - Follow-up tasks / debt

## 6. Issue triage & labels
- `good first issue`: small, self-contained bugs or doc updates.
- `needs models`: tasks touching FastEmbed/Whisper downloads.
- `tauri2-blocker`: items required before the Tauri 2 upgrade.
- `help wanted`: bigger features where design is already sketched.

If you’re unsure where to start, comment on an issue and ask to be assigned.

## 7. Release cadence
We aim for tagged releases every 4–6 weeks. Each release should include:
- Updated CHANGELOG entry (coming soon) summarizing features/fixes
- Fresh binary builds for macOS, Windows, Linux
- Verification that the Quick Menu → Manage Models flow still works on a clean profile

## 8. Questions?
Open a Discussion or ping the maintainers via issues. We’d love feedback on docs, build tooling, and the path to Tauri 2.
