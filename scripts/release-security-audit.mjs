#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");
const defaultBundleRoot = path.join(repoRoot, "src-tauri", "target", "release", "bundle");
const allowlistPath = path.join(repoRoot, ".release-audit-allowlist");

const args = process.argv.slice(2);

const options = {
  artifacts: [],
  bundleRoot: defaultBundleRoot,
  skipSourceScan: false,
  skipArtifactScan: false,
  allowMissingArtifacts: false,
  json: false,
};

for (let i = 0; i < args.length; i += 1) {
  const arg = args[i];
  if (arg === "--artifact") {
    options.artifacts.push(args[i + 1]);
    i += 1;
    continue;
  }
  if (arg === "--bundle-root") {
    options.bundleRoot = args[i + 1];
    i += 1;
    continue;
  }
  if (arg === "--skip-source-scan") {
    options.skipSourceScan = true;
    continue;
  }
  if (arg === "--skip-artifact-scan") {
    options.skipArtifactScan = true;
    continue;
  }
  if (arg === "--allow-missing-artifacts") {
    options.allowMissingArtifacts = true;
    continue;
  }
  if (arg === "--json") {
    options.json = true;
    continue;
  }
  if (arg === "--help" || arg === "-h") {
    console.log(`
Usage: node scripts/release-security-audit.mjs [options]

Options:
  --artifact <path>           Artifact path to scan (repeatable, .app dir or .dmg file)
  --bundle-root <path>        Bundle root to auto-discover artifacts
  --skip-source-scan          Skip tracked-source secret scan
  --skip-artifact-scan        Skip release artifact scan
  --allow-missing-artifacts   Do not fail when no artifacts are discovered
  --json                      Emit machine-readable JSON output
`);
    process.exit(0);
  }
}

const findings = [];
const mountsToDetach = [];
const allowlistMatchers = loadAllowlist();

const fileNameBlocklist = [
  /(^|\/)\.env(\.|$)/i,
  /(^|\/)\.npmrc$/i,
  /(^|\/)\.netrc$/i,
  /(^|\/)(id_rsa|id_ed25519)(\.pub)?$/i,
  /(^|\/)credentials(\.json)?$/i,
  /(^|\/)secrets?(\.|$)/i,
  /\.p12$/i,
  /\.pfx$/i,
  /\.pem$/i,
  /\.key$/i,
  /\.der$/i,
  /\.crt$/i,
];

const secretPatterns = [
  { id: "aws_access_key", regex: /\b(AKIA|ASIA)[0-9A-Z]{16}\b/g },
  { id: "google_api_key", regex: /\bAIza[0-9A-Za-z\-_]{35}\b/g },
  { id: "github_pat", regex: /\bgh[pousr]_[A-Za-z0-9_]{30,}\b/g },
  { id: "openai_key", regex: /\bsk-[A-Za-z0-9_\-]{20,}\b/g },
  { id: "private_key_block", regex: /-----BEGIN [A-Z ]*PRIVATE KEY-----/g },
  { id: "slack_token", regex: /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/g },
  { id: "generic_bearer", regex: /\bBearer\s+[A-Za-z0-9._\-]{25,}\b/g },
];

const textExtensions = new Set([
  ".txt",
  ".md",
  ".json",
  ".js",
  ".mjs",
  ".cjs",
  ".ts",
  ".tsx",
  ".html",
  ".css",
  ".xml",
  ".yml",
  ".yaml",
  ".toml",
  ".ini",
  ".conf",
  ".plist",
  ".sql",
  ".csv",
  ".log",
  ".rtf",
  ".env",
  ".sh",
]);

function loadAllowlist() {
  if (!fs.existsSync(allowlistPath)) return [];
  const lines = fs
    .readFileSync(allowlistPath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"));

  return lines.map((line) => {
    if (line.startsWith("/") && line.lastIndexOf("/") > 0) {
      const lastSlash = line.lastIndexOf("/");
      const body = line.slice(1, lastSlash);
      const flags = line.slice(lastSlash + 1);
      return { type: "regex", value: new RegExp(body, flags) };
    }
    return { type: "literal", value: line.toLowerCase() };
  });
}

function run(command, commandArgs, options = {}) {
  const result = spawnSync(command, commandArgs, {
    encoding: "utf8",
    ...options,
  });
  return result;
}

function addFinding(level, category, targetPath, detail, snippet = "") {
  const finding = { level, category, path: targetPath, detail, snippet };
  if (isAllowlisted(finding)) return;
  findings.push(finding);
}

function isAllowlisted(finding) {
  const haystack = `${finding.path}\n${finding.detail}\n${finding.snippet}`.toLowerCase();
  return allowlistMatchers.some((matcher) => {
    if (matcher.type === "literal") return haystack.includes(matcher.value);
    return matcher.value.test(`${finding.path}\n${finding.detail}\n${finding.snippet}`);
  });
}

function getAllFiles(rootDir) {
  const results = [];
  const stack = [rootDir];
  while (stack.length > 0) {
    const current = stack.pop();
    const entries = fs.readdirSync(current, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      if (entry.isSymbolicLink()) continue;
      if (entry.isDirectory()) {
        stack.push(fullPath);
      } else if (entry.isFile()) {
        results.push(fullPath);
      }
    }
  }
  return results;
}

function maybeContainsSecret(content, filePath) {
  for (const { id, regex } of secretPatterns) {
    const matches = content.match(regex);
    if (!matches || matches.length === 0) continue;
    addFinding(
      "error",
      "secret_pattern",
      filePath,
      `Matched ${id}`,
      String(matches[0]).slice(0, 160),
    );
  }
}

function scanTextFile(filePath) {
  const stat = fs.statSync(filePath);
  if (stat.size > 5 * 1024 * 1024) return;
  let content = "";
  try {
    content = fs.readFileSync(filePath, "utf8");
  } catch {
    return;
  }
  maybeContainsSecret(content, filePath);
}

function scanBinaryWithStrings(filePath) {
  const stat = fs.statSync(filePath);
  if (stat.size > 100 * 1024 * 1024) return;
  const result = run("strings", ["-a", filePath]);
  if (result.status !== 0) return;
  maybeContainsSecret(result.stdout || "", filePath);
}

function scanPathBlocklist(filePath) {
  const normalized = filePath.replaceAll("\\", "/");
  for (const pattern of fileNameBlocklist) {
    if (pattern.test(normalized)) {
      addFinding("error", "blocked_filename", filePath, `Blocked file pattern: ${pattern}`);
    }
  }
}

function shouldTreatAsText(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  if (textExtensions.has(ext)) return true;

  const stat = fs.statSync(filePath);
  if (stat.size === 0 || stat.size > 1024 * 1024) return false;

  try {
    const buffer = fs.readFileSync(filePath);
    const sample = buffer.subarray(0, Math.min(4096, buffer.length));
    const hasNullByte = sample.includes(0);
    return !hasNullByte;
  } catch {
    return false;
  }
}

function scanFiles(files) {
  for (const filePath of files) {
    scanPathBlocklist(filePath);
    if (shouldTreatAsText(filePath)) {
      scanTextFile(filePath);
    } else {
      scanBinaryWithStrings(filePath);
    }
  }
}

function discoverArtifacts() {
  const artifacts = [];
  if (options.artifacts.length > 0) {
    for (const artifact of options.artifacts) {
      const absolute = path.isAbsolute(artifact) ? artifact : path.join(repoRoot, artifact);
      artifacts.push(absolute);
    }
    return artifacts;
  }

  if (!fs.existsSync(options.bundleRoot)) return artifacts;

  const macosDir = path.join(options.bundleRoot, "macos");
  if (fs.existsSync(macosDir)) {
    const apps = fs
      .readdirSync(macosDir)
      .filter((name) => name.endsWith(".app"))
      .map((name) => path.join(macosDir, name));
    artifacts.push(...apps);
  }

  const dmgDir = path.join(options.bundleRoot, "dmg");
  if (fs.existsSync(dmgDir)) {
    const dmgs = fs
      .readdirSync(dmgDir)
      .filter((name) => name.endsWith(".dmg"))
      .map((name) => path.join(dmgDir, name));
    artifacts.push(...dmgs);
  }

  return artifacts;
}

function scanAppBundle(appPath) {
  if (!fs.existsSync(appPath)) {
    addFinding("error", "missing_artifact", appPath, "App bundle does not exist");
    return;
  }
  const files = getAllFiles(appPath);
  scanFiles(files);
}

function extractMountPoints(plistOutput) {
  const mountPoints = [];
  const regex = /<key>mount-point<\/key>\s*<string>([^<]+)<\/string>/g;
  let match = regex.exec(plistOutput);
  while (match) {
    mountPoints.push(match[1]);
    match = regex.exec(plistOutput);
  }
  return mountPoints;
}

function scanDmg(dmgPath) {
  if (!fs.existsSync(dmgPath)) {
    addFinding("error", "missing_artifact", dmgPath, "DMG does not exist");
    return;
  }

  const attach = run("hdiutil", ["attach", "-readonly", "-nobrowse", "-plist", dmgPath]);
  if (attach.status !== 0) {
    addFinding(
      "error",
      "dmg_attach_failed",
      dmgPath,
      (attach.stderr || attach.stdout || "").trim() || "Could not attach DMG",
    );
    return;
  }

  const mountPoints = extractMountPoints(attach.stdout || "");
  if (mountPoints.length === 0) {
    addFinding("error", "dmg_mountpoint_missing", dmgPath, "No mount-point found in hdiutil output");
    return;
  }

  for (const mountPoint of mountPoints) {
    mountsToDetach.push(mountPoint);
    let scanned = false;
    const entries = fs.readdirSync(mountPoint, { withFileTypes: true });
    for (const entry of entries) {
      if (!entry.isDirectory() || !entry.name.endsWith(".app")) continue;
      scanAppBundle(path.join(mountPoint, entry.name));
      scanned = true;
    }

    if (!scanned) {
      const files = getAllFiles(mountPoint);
      scanFiles(files);
    }
  }
}

function detachAllMounts() {
  for (const mountPoint of mountsToDetach.reverse()) {
    run("hdiutil", ["detach", mountPoint]);
  }
}

function scanTrackedSourceFiles() {
  const result = run("git", ["-C", repoRoot, "ls-files"]);
  if (result.status !== 0) {
    addFinding("error", "git_ls_files_failed", repoRoot, (result.stderr || "").trim());
    return;
  }

  const files = (result.stdout || "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((relative) => path.join(repoRoot, relative))
    .filter((filePath) => fs.existsSync(filePath) && fs.statSync(filePath).isFile());

  for (const filePath of files) {
    const relativePath = path.relative(repoRoot, filePath);
    if (relativePath.startsWith("src-tauri/target/")) continue;
    if (relativePath.startsWith("dist/")) continue;
    scanPathBlocklist(filePath);
    if (shouldTreatAsText(filePath)) scanTextFile(filePath);
  }
}

function printOutput() {
  if (options.json) {
    console.log(
      JSON.stringify(
        {
          ok: findings.length === 0,
          findings,
        },
        null,
        2,
      ),
    );
    return;
  }

  if (findings.length === 0) {
    console.log("✅ Release security audit passed. No blocked files or secret patterns found.");
    return;
  }

  console.error(`❌ Release security audit found ${findings.length} issue(s):`);
  findings.forEach((finding, index) => {
    console.error(
      `${index + 1}. [${finding.category}] ${finding.path}\n   ${finding.detail}${
        finding.snippet ? `\n   snippet: ${finding.snippet}` : ""
      }`,
    );
  });
}

try {
  if (!options.skipSourceScan) {
    scanTrackedSourceFiles();
  }

  if (!options.skipArtifactScan) {
    const artifacts = discoverArtifacts();
    if (artifacts.length === 0 && !options.allowMissingArtifacts) {
      addFinding("error", "no_artifacts", options.bundleRoot, "No release artifacts were found");
    }

    for (const artifact of artifacts) {
      if (artifact.endsWith(".app")) {
        scanAppBundle(artifact);
      } else if (artifact.endsWith(".dmg")) {
        scanDmg(artifact);
      } else if (fs.existsSync(artifact) && fs.statSync(artifact).isFile()) {
        scanFiles([artifact]);
      } else {
        addFinding("error", "unknown_artifact", artifact, "Unsupported or missing artifact path");
      }
    }
  }
} finally {
  detachAllMounts();
}

printOutput();
if (findings.length > 0) process.exit(1);
