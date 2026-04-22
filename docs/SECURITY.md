# Security Policy

## Supported versions
Cosmos OSS currently ships builds directly from `main`. Once we cut official releases we will publish a support matrix here. Until then, please assume only the latest commit is supported.

## Reporting a vulnerability
1. **Do not open a public issue.**
2. Go to the GitHub repository and click **Security → Report a vulnerability**, or email `security@cosmos-oss.org` (placeholder inbox).
3. Include:
   - A clear description of the issue
   - Steps to reproduce
   - The commit hash / version you tested
   - Any proof-of-concept exploit or screenshots

We will acknowledge reports within 3 business days and provide status updates at least weekly until the issue is resolved.

## Scope
- The desktop application code in this repository (`src/`, `src-tauri/`).
- The build scripts and docs.

Out of scope: proprietary Cosmos services, self-hosted forks, or any infrastructure we don’t manage.

## What is and isn't protected today

Cosmos stores indexed metadata, text chunks, transcripts, and vector embeddings in a local SQLite database with SQLCipher encryption. Some honest notes on where that protection stops:

- **Database key storage.** The per-user database key is generated on first launch with `rand::rngs::OsRng` and stored under the app's data directory with light obfuscation (base64 + a fixed XOR byte), not the system keychain. This is a first-launch ergonomics choice that avoids a keychain prompt before the user has opted in. An attacker with local read access to the app data folder can recover the key. Tracked as a v0.2 item: migration to `Keychain` on macOS, `Credential Manager` on Windows, and `secret-service` on Linux. For at-rest protection in the meantime, Cosmos relies on FileVault (macOS, enabled by default on modern hardware) or the equivalent full-disk encryption on your platform.
- **In-memory access.** While Cosmos is running, the decrypted key lives in process memory. An attacker with code execution on your machine can read it. This is true of every local desktop application.
- **No network egress by default.** Indexing, embeddings, transcription, and search all run locally. No telemetry, no analytics beacons, no crash reports leave the machine unless you opt into something like the Gemini/Veo integration via the in-app App Store. External API calls, when enabled, go directly from your machine to the third-party endpoint you configured.
- **Updater trust.** Cosmos uses Tauri's updater with a signed manifest. Public signing keys are pinned at build time; the updater refuses unsigned or mis-signed artifacts.
- **Content Security Policy.** Production builds set a restrictive CSP (`default-src 'self'`, etc.). Dev builds are permissive to accommodate the Vite HMR channel.

If any of the above would change your threat model, please weigh that before adopting Cosmos for sensitive workflows.

## Disclosure process
1. Triage and reproduce the issue.
2. Prepare a patch + regression tests.
3. Coordinate a release date with the reporter.
4. Publish an advisory and tag a release.
5. Credit the reporter if they consent.

Thanks for helping make Cosmos OSS safer.
