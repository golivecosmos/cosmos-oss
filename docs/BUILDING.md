# Building Cosmos OSS

This guide explains how to reproduce the development environment, download the offline models, and ship production binaries for macOS, Windows, and Linux.

> **TL;DR** – install Node 20 + pnpm, Rust stable, the Tauri prerequisites for your OS, then run `pnpm dev` for development or `pnpm tauri build` for releases.

---

## 0. Choose your setup path

### Option 1: GitHub DMG download
1. Open [GitHub Releases](https://github.com/cosmos-oss/cosmos-oss/releases).
2. Download the latest macOS `.dmg`.
3. Install and launch Cosmos.

### Option 2: Packaged download from the Cosmos website
1. Open the Cosmos website download page: [app.meetcosmos.com/download](https://app.meetcosmos.com/download).
2. Download the package for your OS.
3. Install and launch Cosmos.

### Option 3: Build from source (this document)
Continue with the steps below.

## 1. Common prerequisites

| Tool | Version | Notes |
| ---- | ------- | ----- |
| Node.js | 20.x LTS | `corepack enable` then `corepack prepare pnpm@9.15.3 --activate` |
| pnpm | 9.x | Needed for workspaces + lockfile |
| Rust | Stable (1.78+) | Install via `rustup` |
| Tauri toolchain | per [guide](https://tauri.app/v1/guides/getting-started/prerequisites) | Xcode CLT (mac), MSVC Build Tools (Win), `libgtk-3-dev` & friends (Linux) |
| FFmpeg binaries | latest | Place in `src-tauri/bin` or run `pnpm bootstrap:assets:ffmpeg` |
| Git LFS (optional) | latest | Required if you plan to commit sample assets |

Install JS deps once:
```bash
pnpm install
```

Optional: prefetch FFmpeg + model assets immediately:
```bash
pnpm bootstrap:assets
```

`postinstall` can do this automatically when explicitly enabled:
```bash
COSMOS_BOOTSTRAP_ASSETS=1 pnpm install
```

## 2. OS-specific setup

### macOS (Apple Silicon or Intel)
```bash
# Install the Xcode Command Line Tools (if not already)
xcode-select --install

# Ensure the macOS target is available for Rust
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

### Windows 11 / 10
1. Install the “Desktop development with C++” workload from Visual Studio Build Tools (MSVC, Windows 11 SDK).
2. Use the “x64 Native Tools Command Prompt for VS” when running `pnpm tauri build`.
3. Install [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) if it isn’t already present.

### Linux (Ubuntu/Fedora/Arch)
Install GTK and mold dependencies (example for Ubuntu/Debian):
```bash
sudo apt update && sudo apt install \
  libgtk-3-dev libwebkit2gtk-4.1-dev \
  build-essential libssl-dev libayatana-appindicator3-dev \
  librsvg2-dev ffmpeg
```

## 3. Running the desktop app locally
```bash
# Launch Vite + Tauri dev shell
pnpm dev
```
This command rebuilds Rust on file changes, launches the Tauri window, and attaches React Fast Refresh.

### Downloading the models
1. Open the running desktop app.
2. Click the lightning-bolt menu in the top bar → **Manage AI Models**.
3. Press **Download models**. Progress events are streamed from the Rust backend.
4. Once complete, the files live at:
   - macOS: `~/Library/Application Support/cosmos/models`
   - Windows: `%APPDATA%/cosmos/models`
   - Linux: `$XDG_DATA_HOME/cosmos/models` (defaults to `~/.local/share/cosmos/models`)

#### Manual mirrors
Set these environment variables before launching if you need to point at an internal registry:
```
COSMOS_MODEL_BASE_URL=https://my-hf-mirror.example \
COSMOS_MODEL_NAMESPACE=nomic-ai \
COSMOS_TEXT_MODEL_SLUG=nomic-embed-text-v1.5/resolve/main \
COSMOS_VISION_MODEL_SLUG=nomic-embed-vision-v1.5/resolve/main \
pnpm dev
```

## 4. Building production binaries
### macOS universal DMG
```bash
pnpm build:desktop
```
Outputs a universal app bundle and DMG under `src-tauri/target/release/bundle/macos`.

If your release includes bundled third-party binaries, include the notices in [`docs/THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) with your packaged artifacts.

### Windows MSI / NSIS
```bash
pnpm tauri build --target x86_64-pc-windows-msvc
```
Artifacts appear under `src-tauri/target/x86_64-pc-windows-msvc/release/bundle`.
> Code-signing is optional. Leave `signingIdentity` blank for unsigned builds.

### Linux AppImage / deb / rpm
```bash
# AppImage + deb for x86_64
pnpm tauri build --target x86_64-unknown-linux-gnu
```
Install platform packages such as `appimagetool` if Tauri requests them.

## 5. Secure macOS release (signed + notarized + audited)
For public macOS releases, use the secure release pipeline instead of uploading ad-hoc DMGs.

Current distribution status:
- Public packaged distribution is GitHub Releases only.
- The only published package today is the macOS DMG.
- There is no separate download website or hosted updater feed yet.
- Windows and Linux users should build from source until public packages exist for those platforms.

Prerequisites:
- Developer ID Application certificate installed in Keychain.
- Signing identity exported for the release command:
  ```bash
  export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
  ```
- `xcrun notarytool` keychain profile configured:
  ```bash
  xcrun notarytool store-credentials "notarytool-profile" \
    --apple-id <apple-id> \
    --team-id <team-id> \
    --password <app-specific-password>
  ```
- GitHub CLI authenticated (`gh auth login`) for release asset upload.

Commands:
```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"

# Build + sign + notarize + security audit (no upload)
pnpm release:production

# Build + sign + notarize + security audit + upload artifacts to an existing/new GitHub release tag
pnpm release:production:upload -- --tag v0.1.1 --repo golivecosmos/cosmos-oss
```

Security guarantees in this pipeline:
- Refuses unsigned/ad-hoc app bundles by default.
- Runs notarization + stapling for DMGs.
- Runs `scripts/release-security-audit.mjs` before upload:
  - scans tracked source for high-risk secret patterns,
  - scans built `.app` / mounted `.dmg` contents,
  - blocks known sensitive file types (`.env`, private keys, credential files).

Allowlist file: `.release-audit-allowlist` (keep minimal; every exception weakens the gate).

## 6. Optional integrations
- **Google Gemini / Veo**: open **Settings → App Store**, install “Google Gemini,” and provide the API key. Keys are stored locally via SQLCipher.
- **Whisper transcription**: automatically enabled after the Whisper model downloads (FastEmbed uses Candle).

## 7. Testing & QA
| Command | Purpose |
| ------- | ------- |
| `pnpm lint` | ESLint over all TypeScript/TSX code |
| `pnpm tauri build --debug` | Rust + bundle smoke-test |
| `pnpm tauri dev --features dev-tools` | Launch devtools-enabled window |
| `cd src-tauri && cargo test` | Backend unit tests (services + commands) |

Whenever you change database schemas or long-running commands, please add or update tests in `src-tauri/src/services/tests/` and describe manual QA steps in your PR.

## 8. Updater & signing
The updater plugin is enabled, but OSS configs ship with empty updater settings (`"plugins.updater.endpoints": []`, `"plugins.updater.pubkey": ""`), so update checks are effectively disabled until you provide your own endpoint(s) and minisign key.

In other words: this repo does not currently ship a hosted updater service. Public distribution is the notarized DMG uploaded to GitHub Releases.

## 9. Troubleshooting
| Symptom | Fix |
| ------- | --- |
| `Download timed out` when fetching models | Check connectivity or host your own mirror via the env vars above |
| `FastEmbed text model not found` | Ensure `models/nomic-embed-text-v1.5/onnx/model.onnx` exists under the app data dir |
| `npm` instead of `pnpm` used by IDE | Run `corepack enable` and restart the IDE |
| Windows build fails with linker errors | Re-open the “x64 Native Tools” prompt after installing Build Tools |

Need more help? Open an issue or start a Discussion on GitHub.
