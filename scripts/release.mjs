#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");
const bundleRoot = path.join(repoRoot, "src-tauri", "target", "release", "bundle");
const packageJsonPath = path.join(repoRoot, "package.json");

const args = process.argv.slice(2);
const pkg = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));

const options = {
  upload: false,
  skipBuild: false,
  skipAudit: false,
  skipNotarize: false,
  allowUnsigned: false,
  allowDirty: false,
  allowNonMain: false,
  repo: process.env.COSMOS_GH_REPO || "golivecosmos/cosmos-oss",
  tag: `v${pkg.version}`,
  config: "src-tauri/tauri.conf.production.json",
  notaryProfile: process.env.COSMOS_NOTARY_PROFILE || "notarytool-profile",
};

const envSigningIdentity =
  process.env.COSMOS_APPLE_SIGNING_IDENTITY || process.env.APPLE_SIGNING_IDENTITY || "";
let generatedConfigDir = null;

for (let i = 0; i < args.length; i += 1) {
  const arg = args[i];
  if (arg === "--upload") {
    options.upload = true;
    continue;
  }
  if (arg === "--skip-build") {
    options.skipBuild = true;
    continue;
  }
  if (arg === "--skip-audit") {
    options.skipAudit = true;
    continue;
  }
  if (arg === "--skip-notarize") {
    options.skipNotarize = true;
    continue;
  }
  if (arg === "--allow-unsigned") {
    options.allowUnsigned = true;
    continue;
  }
  if (arg === "--allow-dirty") {
    options.allowDirty = true;
    continue;
  }
  if (arg === "--allow-non-main") {
    options.allowNonMain = true;
    continue;
  }
  if (arg === "--repo") {
    options.repo = args[i + 1];
    i += 1;
    continue;
  }
  if (arg === "--tag") {
    options.tag = args[i + 1];
    i += 1;
    continue;
  }
  if (arg === "--config") {
    options.config = args[i + 1];
    i += 1;
    continue;
  }
  if (arg === "--notary-profile") {
    options.notaryProfile = args[i + 1];
    i += 1;
    continue;
  }
  if (arg === "--help" || arg === "-h") {
    console.log(`
Usage: node scripts/release.mjs [options]

Options:
  --upload                 Upload built artifacts to a GitHub release
  --tag <vX.Y.Z>           Release tag (default: package.json version prefixed with v)
  --repo <owner/repo>      GitHub repository (default: golivecosmos/cosmos-oss)
  --config <path>          Tauri build config (default: src-tauri/tauri.conf.production.json)
  --notary-profile <name>  Keychain profile for notarytool (default: notarytool-profile)
  --skip-build             Skip build step
  --skip-audit             Skip security audit step
  --skip-notarize          Skip notarization/stapling step
  --allow-unsigned         Allow ad-hoc/unsigned build output
  --allow-dirty            Allow uncommitted changes
  --allow-non-main         Allow running outside main/master

Environment:
  APPLE_SIGNING_IDENTITY or COSMOS_APPLE_SIGNING_IDENTITY
                           Developer ID Application identity used only at release time
`);
    process.exit(0);
  }
}

function log(message) {
  console.log(`[release] ${message}`);
}

function cleanupGeneratedConfig() {
  if (!generatedConfigDir) return;
  fs.rmSync(generatedConfigDir, { recursive: true, force: true });
  generatedConfigDir = null;
}

function run(command, commandArgs, opts = {}) {
  const result = spawnSync(command, commandArgs, {
    cwd: repoRoot,
    stdio: opts.capture ? "pipe" : "inherit",
    encoding: "utf8",
    env: process.env,
  });

  if (opts.allowFailure) return result;

  if (result.status !== 0) {
    const extra = opts.capture ? `\n${(result.stdout || "")}${(result.stderr || "")}` : "";
    throw new Error(`Command failed: ${command} ${commandArgs.join(" ")}${extra}`);
  }
  return result;
}

function assertBranchAndCleanState() {
  const branch = run("git", ["rev-parse", "--abbrev-ref", "HEAD"], { capture: true }).stdout.trim();
  if (!options.allowNonMain && branch !== "main" && branch !== "master") {
    throw new Error(`Production release must run from main/master. Current branch: ${branch}`);
  }

  if (!options.allowDirty) {
    const status = run("git", ["status", "--porcelain"], { capture: true }).stdout.trim();
    if (status.length > 0) {
      throw new Error("Working tree is not clean. Commit or stash changes, or use --allow-dirty.");
    }
  }
}

function resolveBuildConfig() {
  const configPath = path.resolve(repoRoot, options.config);
  const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
  const configuredIdentity = config?.bundle?.macOS?.signingIdentity || "";

  if (!envSigningIdentity) {
    if (!configuredIdentity && !options.allowUnsigned) {
      throw new Error(
        "No macOS signing identity configured. Set APPLE_SIGNING_IDENTITY or COSMOS_APPLE_SIGNING_IDENTITY for release builds, or use --allow-unsigned for internal testing.",
      );
    }
    return options.config;
  }

  config.bundle ??= {};
  config.bundle.macOS ??= {};
  config.bundle.macOS.signingIdentity = envSigningIdentity;

  generatedConfigDir = fs.mkdtempSync(path.join(os.tmpdir(), "cosmos-release-config-"));
  const generatedConfigPath = path.join(generatedConfigDir, path.basename(configPath));
  fs.writeFileSync(generatedConfigPath, `${JSON.stringify(config, null, 2)}\n`);

  log(`Using signing identity from environment for ${options.config}`);
  return generatedConfigPath;
}

function buildRelease() {
  log("Building frontend and Tauri production bundle...");
  const buildConfig = resolveBuildConfig();
  run("pnpm", ["run", "build"]);
  run("pnpm", ["run", "tauri", "build", "--config", buildConfig]);
}

function listApps() {
  const macosDir = path.join(bundleRoot, "macos");
  if (!fs.existsSync(macosDir)) return [];
  return fs
    .readdirSync(macosDir)
    .filter((name) => name.endsWith(".app"))
    .map((name) => path.join(macosDir, name));
}

function listDmgs() {
  const dmgDir = path.join(bundleRoot, "dmg");
  if (!fs.existsSync(dmgDir)) return [];
  return fs
    .readdirSync(dmgDir)
    .filter((name) => name.endsWith(".dmg"))
    .map((name) => path.join(dmgDir, name));
}

function createDmgFromApp(appPath) {
  const dmgDir = path.join(bundleRoot, "dmg");
  fs.mkdirSync(dmgDir, { recursive: true });
  const archSuffix = process.arch === "arm64" ? "aarch64" : process.arch === "x64" ? "x64" : process.arch;
  const fileName = `Cosmos.OSS_${pkg.version}_${archSuffix}.dmg`;
  const outputPath = path.join(dmgDir, fileName);
  log(`Creating DMG fallback at ${outputPath}`);
  run("hdiutil", [
    "create",
    "-volname",
    "Cosmos OSS",
    "-srcfolder",
    appPath,
    "-ov",
    "-format",
    "UDZO",
    outputPath,
  ]);
  return outputPath;
}

function verifySignedApp(appPath) {
  const result = run("codesign", ["-dv", "--verbose=4", appPath], { capture: true, allowFailure: true });
  const output = `${result.stdout || ""}\n${result.stderr || ""}`;
  if (result.status !== 0) {
    throw new Error(`codesign failed for ${appPath}\n${output}`);
  }

  const adHoc = /Signature=adhoc/i.test(output) || /TeamIdentifier=not set/i.test(output);
  if (adHoc && !options.allowUnsigned) {
    throw new Error(
      `App is not Developer ID signed (${appPath}). Refusing release. Use --allow-unsigned only for internal testing.`,
    );
  }
}

function notarizeDmgs(dmgs) {
  if (options.skipNotarize) {
    log("Skipping notarization by request (--skip-notarize).");
    return;
  }
  if (dmgs.length === 0) {
    throw new Error("No DMG files found to notarize.");
  }

  const profileCheck = run(
    "xcrun",
    ["notarytool", "history", "--keychain-profile", options.notaryProfile],
    { allowFailure: true, capture: true },
  );

  if (profileCheck.status !== 0) {
    throw new Error(
      `Notary profile "${options.notaryProfile}" not available. Configure with:\n` +
        `xcrun notarytool store-credentials "${options.notaryProfile}" --apple-id <id> --team-id <team> --password <app-specific-password>`,
    );
  }

  for (const dmg of dmgs) {
    log(`Submitting for notarization: ${dmg}`);
    run("xcrun", ["notarytool", "submit", dmg, "--keychain-profile", options.notaryProfile, "--wait"]);
    log(`Stapling notarization: ${dmg}`);
    run("xcrun", ["stapler", "staple", dmg]);
    run("xcrun", ["stapler", "validate", dmg]);
  }
}

function runSecurityAudit(artifacts) {
  if (options.skipAudit) {
    log("Skipping security audit by request (--skip-audit).");
    return;
  }

  const auditScript = path.join(repoRoot, "scripts", "release-security-audit.mjs");
  const commandArgs = [auditScript];
  artifacts.forEach((artifact) => {
    commandArgs.push("--artifact", artifact);
  });
  run("node", commandArgs);
}

function ensureGhReady() {
  run("gh", ["--version"], { allowFailure: false });
  const auth = run("gh", ["auth", "status"], { allowFailure: true, capture: true });
  if (auth.status !== 0) {
    throw new Error(`gh auth is not configured. Run: gh auth login\n${auth.stderr || auth.stdout}`);
  }
}

function ensureReleaseExists() {
  const view = run("gh", ["release", "view", options.tag, "--repo", options.repo], {
    allowFailure: true,
    capture: true,
  });
  if (view.status === 0) return;
  log(`Release ${options.tag} not found in ${options.repo}. Creating it with generated notes.`);
  run("gh", ["release", "create", options.tag, "--repo", options.repo, "--generate-notes"]);
}

function uploadArtifacts(artifacts) {
  if (!options.upload) return;
  ensureGhReady();
  ensureReleaseExists();
  const uploadableArtifacts = artifacts.filter(
    (artifact) => fs.existsSync(artifact) && fs.statSync(artifact).isFile(),
  );

  artifacts
    .filter((artifact) => !uploadableArtifacts.includes(artifact))
    .forEach((artifact) => {
      log(`Skipping non-file artifact during GitHub upload: ${artifact}`);
    });

  if (uploadableArtifacts.length === 0) {
    throw new Error("No file artifacts available to upload.");
  }

  run("gh", [
    "release",
    "upload",
    options.tag,
    ...uploadableArtifacts,
    "--repo",
    options.repo,
    "--clobber",
  ]);
}

function main() {
  assertBranchAndCleanState();

  if (!options.skipBuild) {
    buildRelease();
  }

  const apps = listApps();
  if (apps.length === 0) {
    throw new Error(`No .app found under ${path.join(bundleRoot, "macos")}`);
  }

  apps.forEach((appPath) => verifySignedApp(appPath));

  let dmgs = listDmgs();
  if (dmgs.length === 0) {
    dmgs = [createDmgFromApp(apps[0])];
  }

  notarizeDmgs(dmgs);

  const artifacts = [...apps, ...dmgs];
  runSecurityAudit(artifacts);
  uploadArtifacts(artifacts);

  log("Release pipeline completed.");
  artifacts.forEach((artifact) => log(`artifact: ${artifact}`));
}

try {
  main();
} catch (error) {
  console.error(`[release] ERROR: ${error.message}`);
  process.exit(1);
} finally {
  cleanupGeneratedConfig();
}
