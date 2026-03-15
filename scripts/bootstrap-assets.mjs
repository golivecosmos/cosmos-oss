#!/usr/bin/env node

import fs from "node:fs";
import fsp from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");
const tauriBinDir = path.join(repoRoot, "src-tauri", "bin");

const args = new Set(process.argv.slice(2));
const isOptionalRun = args.has("--optional");
const force = args.has("--force");
const skipFfmpeg = args.has("--skip-ffmpeg");
const skipModels = args.has("--skip-models");

const DEFAULT_MODEL_BASE_URL = "https://huggingface.co";
const DEFAULT_MODEL_NAMESPACE = "nomic-ai";
const DEFAULT_TEXT_MODEL_SLUG = "nomic-embed-text-v1.5/resolve/main";
const DEFAULT_VISION_MODEL_SLUG = "nomic-embed-vision-v1.5/resolve/main";

function log(message) {
  console.log(`[bootstrap-assets] ${message}`);
}

function isTruthy(value) {
  if (!value) return false;
  const normalized = String(value).trim().toLowerCase();
  return ["1", "true", "yes", "on"].includes(normalized);
}

function runOrThrow(command, commandArgs, options = {}) {
  const result = spawnSync(command, commandArgs, {
    stdio: "pipe",
    encoding: "utf8",
    ...options,
  });

  if (result.status !== 0) {
    const stdout = (result.stdout || "").trim();
    const stderr = (result.stderr || "").trim();
    const details = [stdout, stderr].filter(Boolean).join("\n");
    throw new Error(`Command failed: ${command} ${commandArgs.join(" ")}\n${details}`);
  }
}

async function downloadToFile(url, destinationPath) {
  const response = await fetch(url, { redirect: "follow" });
  if (!response.ok) {
    throw new Error(`Download failed (${response.status}) for ${url}`);
  }

  const bytes = Buffer.from(await response.arrayBuffer());
  await fsp.mkdir(path.dirname(destinationPath), { recursive: true });
  await fsp.writeFile(destinationPath, bytes);
}

async function copyExecutable(sourcePath, destinationPath) {
  await fsp.mkdir(path.dirname(destinationPath), { recursive: true });
  await fsp.copyFile(sourcePath, destinationPath);
  if (process.platform !== "win32") {
    await fsp.chmod(destinationPath, 0o755);
  }
}

function findFirstFileRecursive(rootDir, candidateNames) {
  const wanted = new Set(candidateNames.map((name) => name.toLowerCase()));
  const stack = [rootDir];

  while (stack.length > 0) {
    const current = stack.pop();
    const entries = fs.readdirSync(current, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(fullPath);
        continue;
      }

      if (entry.isFile() && wanted.has(entry.name.toLowerCase())) {
        return fullPath;
      }
    }
  }

  return null;
}

function extractArchive(archivePath, destinationDir) {
  fs.mkdirSync(destinationDir, { recursive: true });

  if (archivePath.endsWith(".zip")) {
    if (process.platform === "win32") {
      runOrThrow("powershell", [
        "-NoProfile",
        "-Command",
        `Expand-Archive -Path '${archivePath.replace(/'/g, "''")}' -DestinationPath '${destinationDir.replace(/'/g, "''")}' -Force`,
      ]);
    } else {
      runOrThrow("unzip", ["-q", archivePath, "-d", destinationDir]);
    }
    return;
  }

  if (archivePath.endsWith(".tar.xz")) {
    runOrThrow("tar", ["-xJf", archivePath, "-C", destinationDir]);
    return;
  }

  throw new Error(`Unsupported archive format: ${archivePath}`);
}

function buildModelUrl(baseUrl, namespace, slug, filePath) {
  return `${baseUrl.replace(/\/+$/, "")}/${namespace.replace(/^\/+|\/+$/g, "")}/${slug.replace(
    /^\/+|\/+$/g,
    "",
  )}/${filePath.replace(/^\/+/, "")}`;
}

function resolveCosmosDataDir() {
  const customDir = process.env.COSMOS_APP_DATA_DIR;
  if (customDir) return customDir;

  if (process.platform === "darwin") {
    return path.join(os.homedir(), "Library", "Application Support", "cosmos");
  }

  if (process.platform === "win32") {
    const base = process.env.LOCALAPPDATA || process.env.APPDATA;
    if (!base) throw new Error("Unable to resolve LOCALAPPDATA/APPDATA on Windows");
    return path.join(base, "cosmos");
  }

  const xdgDataHome = process.env.XDG_DATA_HOME;
  if (xdgDataHome) return path.join(xdgDataHome, "cosmos");
  return path.join(os.homedir(), ".local", "share", "cosmos");
}

function resolveFfmpegSource() {
  if (process.platform === "darwin") {
    return {
      ffmpegArchiveUrl: "https://evermeet.cx/ffmpeg/getrelease/zip",
      ffprobeArchiveUrl: "https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip",
      ffmpegCandidates: ["ffmpeg"],
      ffprobeCandidates: ["ffprobe"],
    };
  }

  if (process.platform === "linux") {
    const linuxUrl =
      process.arch === "arm64"
        ? "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linuxarm64-lgpl-shared.tar.xz"
        : "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-lgpl-shared.tar.xz";

    return {
      ffmpegArchiveUrl: linuxUrl,
      ffprobeArchiveUrl: null,
      ffmpegCandidates: ["ffmpeg"],
      ffprobeCandidates: ["ffprobe"],
    };
  }

  if (process.platform === "win32") {
    return {
      ffmpegArchiveUrl: "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
      ffprobeArchiveUrl: null,
      ffmpegCandidates: ["ffmpeg.exe"],
      ffprobeCandidates: ["ffprobe.exe"],
    };
  }

  return null;
}

async function bootstrapFfmpeg(tempDir) {
  const source = resolveFfmpegSource();
  if (!source) {
    log(`Skipping FFmpeg bootstrap: unsupported platform "${process.platform}"`);
    return;
  }

  log("Bootstrapping FFmpeg binaries...");
  await fsp.mkdir(tauriBinDir, { recursive: true });

  const ffmpegArchivePath = path.join(tempDir, path.basename(new URL(source.ffmpegArchiveUrl).pathname) || "ffmpeg.bin");
  log(`Downloading FFmpeg from ${source.ffmpegArchiveUrl}`);
  await downloadToFile(source.ffmpegArchiveUrl, ffmpegArchivePath);

  let ffprobeArchivePath = null;
  if (source.ffprobeArchiveUrl) {
    ffprobeArchivePath = path.join(
      tempDir,
      path.basename(new URL(source.ffprobeArchiveUrl).pathname) || "ffprobe.bin",
    );
    log(`Downloading FFprobe from ${source.ffprobeArchiveUrl}`);
    await downloadToFile(source.ffprobeArchiveUrl, ffprobeArchivePath);
  }

  const ffmpegExtractDir = path.join(tempDir, "ffmpeg-extract");
  extractArchive(ffmpegArchivePath, ffmpegExtractDir);

  const ffmpegSourcePath = findFirstFileRecursive(ffmpegExtractDir, source.ffmpegCandidates);
  if (!ffmpegSourcePath) {
    throw new Error("Could not locate FFmpeg binary after extraction");
  }

  let ffprobeSourcePath = null;
  if (ffprobeArchivePath) {
    const ffprobeExtractDir = path.join(tempDir, "ffprobe-extract");
    extractArchive(ffprobeArchivePath, ffprobeExtractDir);
    ffprobeSourcePath = findFirstFileRecursive(ffprobeExtractDir, source.ffprobeCandidates);
  } else {
    ffprobeSourcePath = findFirstFileRecursive(ffmpegExtractDir, source.ffprobeCandidates);
  }

  if (!ffprobeSourcePath) {
    throw new Error("Could not locate FFprobe binary after extraction");
  }

  if (process.platform === "win32") {
    // Keep both names to support current lookup logic and normal Windows conventions.
    await copyExecutable(ffmpegSourcePath, path.join(tauriBinDir, "ffmpeg"));
    await copyExecutable(ffprobeSourcePath, path.join(tauriBinDir, "ffprobe"));
    await copyExecutable(ffmpegSourcePath, path.join(tauriBinDir, "ffmpeg.exe"));
    await copyExecutable(ffprobeSourcePath, path.join(tauriBinDir, "ffprobe.exe"));
  } else {
    await copyExecutable(ffmpegSourcePath, path.join(tauriBinDir, "ffmpeg"));
    await copyExecutable(ffprobeSourcePath, path.join(tauriBinDir, "ffprobe"));
  }

  log(`FFmpeg bootstrap complete: ${tauriBinDir}`);
}

function buildModelManifest() {
  const baseUrl = process.env.COSMOS_MODEL_BASE_URL || DEFAULT_MODEL_BASE_URL;
  const namespace = process.env.COSMOS_MODEL_NAMESPACE || DEFAULT_MODEL_NAMESPACE;
  const textSlug = process.env.COSMOS_TEXT_MODEL_SLUG || DEFAULT_TEXT_MODEL_SLUG;
  const visionSlug = process.env.COSMOS_VISION_MODEL_SLUG || DEFAULT_VISION_MODEL_SLUG;

  return [
    {
      destination: "nomic-embed-text-v1.5/onnx/model.onnx",
      url: buildModelUrl(baseUrl, namespace, textSlug, "model.onnx"),
    },
    {
      destination: "nomic-embed-text-v1.5/config.json",
      url: buildModelUrl(baseUrl, namespace, textSlug, "config.json"),
    },
    {
      destination: "nomic-embed-text-v1.5/tokenizer.json",
      url: buildModelUrl(baseUrl, namespace, textSlug, "tokenizer.json"),
    },
    {
      destination: "nomic-embed-text-v1.5/tokenizer_config.json",
      url: buildModelUrl(baseUrl, namespace, textSlug, "tokenizer_config.json"),
    },
    {
      destination: "nomic-embed-text-v1.5/special_tokens_map.json",
      url: buildModelUrl(baseUrl, namespace, textSlug, "special_tokens_map.json"),
    },
    {
      destination: "nomic-embed-vision-v1.5/onnx/model.onnx",
      url: buildModelUrl(baseUrl, namespace, visionSlug, "model.onnx"),
    },
    {
      destination: "nomic-embed-vision-v1.5/preprocessor_config.json",
      url: buildModelUrl(baseUrl, namespace, visionSlug, "preprocessor_config.json"),
    },
    {
      destination: "whisper-base/config.json",
      url: "https://huggingface.co/openai/whisper-base/resolve/main/config.json",
    },
    {
      destination: "whisper-base/tokenizer.json",
      url: "https://huggingface.co/openai/whisper-base/resolve/main/tokenizer.json",
    },
    {
      destination: "whisper-base/model.safetensors",
      url: "https://huggingface.co/openai/whisper-base/resolve/main/model.safetensors",
    },
  ];
}

async function bootstrapModels() {
  const cosmosDataDir = resolveCosmosDataDir();
  const modelsRootDir = path.join(cosmosDataDir, "models");
  const manifest = buildModelManifest();

  log(`Bootstrapping model files into ${modelsRootDir}`);
  await fsp.mkdir(modelsRootDir, { recursive: true });

  for (const model of manifest) {
    const destinationPath = path.join(modelsRootDir, model.destination);
    const exists = fs.existsSync(destinationPath);
    if (exists && !force) {
      log(`Skipping existing model file: ${model.destination}`);
      continue;
    }

    const tempPath = `${destinationPath}.part`;
    log(`Downloading model: ${model.url}`);
    await downloadToFile(model.url, tempPath);
    await fsp.mkdir(path.dirname(destinationPath), { recursive: true });
    await fsp.rename(tempPath, destinationPath);
  }

  log("Model bootstrap complete");
}

async function main() {
  if (isOptionalRun && !isTruthy(process.env.COSMOS_BOOTSTRAP_ASSETS)) {
    log("Skipping optional bootstrap. Set COSMOS_BOOTSTRAP_ASSETS=1 to enable.");
    return;
  }

  const tempDir = await fsp.mkdtemp(path.join(os.tmpdir(), "cosmos-bootstrap-"));

  try {
    if (!skipFfmpeg) {
      await bootstrapFfmpeg(tempDir);
    }

    if (!skipModels) {
      await bootstrapModels();
    }

    log("Done");
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(`[bootstrap-assets] ERROR: ${error.message}`);
  process.exitCode = 1;
});
