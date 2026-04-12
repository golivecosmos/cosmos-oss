import React, { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { desktopDir, documentDir, pictureDir } from "@tauri-apps/api/path";
import {
  FolderOpen,
  Shield,
  Brain,
  Loader2,
  Check,
  Download,
  Sparkles,
} from "lucide-react";
import { Button } from "../ui/button";

type OnboardingStep = "pick_folder" | "setup_models" | "downloading" | "indexing";

interface ScanResult {
  file_count: number;
  dir_count: number;
  total_size_bytes: number;
}

interface DownloadProgress {
  file_name: string;
  percentage: number;
  status: string;
}

interface DashboardEmptyProps {
  onStartIndexing: (path: string) => void;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

export const DashboardEmpty: React.FC<DashboardEmptyProps> = ({
  onStartIndexing,
}) => {
  const [step, setStep] = useState<OnboardingStep>("pick_folder");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [includeGemma, setIncludeGemma] = useState(true);
  const [gemmaAlreadyDownloaded, setGemmaAlreadyDownloaded] = useState(false);
  const [coreModelsReady, setCoreModelsReady] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [downloadPhase, setDownloadPhase] = useState<string>("Preparing...");
  const [suggestedPaths, setSuggestedPaths] = useState<{ name: string; path: string }[]>([]);

  // Detect suggested folders on mount
  useEffect(() => {
    const detectPaths = async () => {
      const paths: { name: string; path: string }[] = [];
      try {
        const desktop = await desktopDir();
        if (desktop) paths.push({ name: "Desktop", path: desktop });
      } catch {}
      try {
        const docs = await documentDir();
        if (docs) paths.push({ name: "Documents", path: docs });
      } catch {}
      try {
        const pics = await pictureDir();
        if (pics) paths.push({ name: "Pictures", path: pics });
      } catch {}
      setSuggestedPaths(paths);
    };
    detectPaths();
  }, []);

  // Check Gemma 4 status on mount
  useEffect(() => {
    invoke<boolean>("is_gemma4_downloaded").then((downloaded) => {
      setGemmaAlreadyDownloaded(downloaded);
      if (downloaded) setIncludeGemma(false); // Already have it
    }).catch(() => {});

    // Check if core models are ready
    invoke<any>("check_models_status").then((status: any) => {
      setCoreModelsReady(status?.models_available === true);
    }).catch(() => {});
  }, []);

  const handleChooseFolder = useCallback(async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose a folder to understand",
    });
    if (selected && typeof selected === "string") {
      setSelectedPath(selected);
      // Quick scan for file count
      invoke<ScanResult>("scan_directory", { path: selected })
        .then(setScanResult)
        .catch(() => {});
      setStep("setup_models");
    }
  }, []);

  const handleSuggestedPath = useCallback((path: string) => {
    setSelectedPath(path);
    invoke<ScanResult>("scan_directory", { path })
      .then(setScanResult)
      .catch(() => {});
    setStep("setup_models");
  }, []);

  const handleStartDownloadAndIndex = useCallback(async () => {
    if (!selectedPath) return;
    setStep("downloading");

    // Download core models if needed
    if (!coreModelsReady) {
      setDownloadPhase("Downloading AI models...");
      try {
        await invoke("download_models");
      } catch (e) {
        console.error("Core model download failed:", e);
      }
    }

    // Download Gemma 4 if selected and not already present
    if (includeGemma && !gemmaAlreadyDownloaded) {
      setDownloadPhase("Downloading understanding model...");
      try {
        await invoke("download_gemma4_model");
      } catch (e) {
        console.error("Gemma 4 download failed:", e);
        // Non-fatal: falls back to TF-IDF
      }
    }

    // All downloads done, start indexing
    setStep("indexing");
    setDownloadPhase("Indexing your files...");
    onStartIndexing(selectedPath);
  }, [selectedPath, includeGemma, gemmaAlreadyDownloaded, coreModelsReady, onStartIndexing]);

  // Listen for download progress events
  useEffect(() => {
    let unlistenCore: (() => void) | null = null;
    let unlistenGemma: (() => void) | null = null;

    const setup = async () => {
      unlistenCore = await listen("download_progress", (event: any) => {
        const p = event?.payload;
        if (p?.file_name && p?.percentage != null) {
          setDownloadProgress((prev) => ({ ...prev, [p.file_name]: p.percentage }));
        }
      });
      unlistenGemma = await listen("gemma4_download_progress", (event: any) => {
        const p = event?.payload;
        if (p?.percentage != null) {
          setDownloadPhase(`Downloading understanding model... ${Math.round(p.percentage)}%`);
          setDownloadProgress((prev) => ({ ...prev, "gemma-4-e2b.gguf": p.percentage }));
        }
      });
    };
    setup();
    return () => {
      if (unlistenCore) unlistenCore();
      if (unlistenGemma) unlistenGemma();
    };
  }, []);

  const folderName = selectedPath?.split("/").pop() || selectedPath || "";

  // ===== STEP 1: PICK FOLDER =====
  if (step === "pick_folder") {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center max-w-lg space-y-6">
          <div className="mx-auto w-20 h-20 rounded-2xl bg-primary/10 flex items-center justify-center">
            <FolderOpen className="w-10 h-10 text-primary" />
          </div>

          <div className="space-y-2">
            <h1 className="text-2xl font-semibold">See what's in your files</h1>
            <p className="text-muted-foreground text-sm leading-relaxed max-w-sm mx-auto">
              Topics, patterns, and connections, discovered by AI running entirely on your machine.
            </p>
          </div>

          <Button size="lg" onClick={handleChooseFolder}>
            Choose a Folder
          </Button>

          {suggestedPaths.length > 0 && (
            <div className="space-y-1.5">
              <p className="text-xs text-muted-foreground">or try</p>
              <div className="flex items-center justify-center gap-2">
                {suggestedPaths.map((sp) => (
                  <button
                    key={sp.path}
                    onClick={() => handleSuggestedPath(sp.path)}
                    className="px-3 py-1 rounded-full border text-xs text-muted-foreground hover:text-foreground hover:border-ring transition-colors"
                  >
                    ~/{sp.name}
                  </button>
                ))}
              </div>
            </div>
          )}

          <div className="flex items-center justify-center gap-6 text-xs text-muted-foreground pt-2">
            <span className="flex items-center gap-1.5">
              <Shield className="w-3.5 h-3.5" />
              100% local
            </span>
            <span className="flex items-center gap-1.5">
              <Brain className="w-3.5 h-3.5" />
              No cloud, no API keys
            </span>
          </div>
        </div>
      </div>
    );
  }

  // ===== STEP 2: MODEL SELECTION =====
  if (step === "setup_models") {
    const totalDownload = (coreModelsReady ? 0 : 500) + (includeGemma && !gemmaAlreadyDownloaded ? 1500 : 0);

    return (
      <div className="flex items-center justify-center h-full">
        <div className="max-w-md space-y-6">
          <div>
            <h2 className="text-lg font-semibold">
              {folderName}
            </h2>
            <p className="text-sm text-muted-foreground">
              {scanResult
                ? `${scanResult.file_count.toLocaleString()} files found · ${formatBytes(scanResult.total_size_bytes)}`
                : "Scanning..."}
            </p>
          </div>

          <p className="text-sm text-muted-foreground">
            To understand these files, Cosmos needs AI models running on your machine.
          </p>

          <div className="space-y-3 rounded-lg border p-4">
            {/* Core models */}
            <div className="flex items-start gap-3">
              <div className="mt-0.5 w-5 h-5 rounded border flex items-center justify-center bg-primary border-primary">
                <Check className="w-3 h-3 text-primary-foreground" />
              </div>
              <div className="flex-1">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">Embedding Model</span>
                  <span className="text-xs text-muted-foreground">
                    {coreModelsReady ? (
                      <span className="text-emerald-500">Installed</span>
                    ) : (
                      "~500 MB"
                    )}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground">
                  Reads and understands your files. Required.
                </p>
              </div>
            </div>

            {/* Gemma 4 */}
            {!gemmaAlreadyDownloaded && (
              <div className="flex items-start gap-3">
                <button
                  onClick={() => setIncludeGemma(!includeGemma)}
                  className={`mt-0.5 w-5 h-5 rounded border flex items-center justify-center transition-colors ${
                    includeGemma
                      ? "bg-primary border-primary"
                      : "border-input hover:border-ring"
                  }`}
                >
                  {includeGemma && <Check className="w-3 h-3 text-primary-foreground" />}
                </button>
                <div className="flex-1">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium flex items-center gap-1.5">
                      Understanding Model
                      <span className="text-[10px] px-1.5 py-0 rounded-full bg-amber-500/10 text-amber-600 border border-amber-500/20">
                        Recommended
                      </span>
                    </span>
                    <span className="text-xs text-muted-foreground">~1.5 GB</span>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Names topics, generates insights, finds patterns. Without it, topic names are generic keywords.
                  </p>
                </div>
              </div>
            )}
          </div>

          {totalDownload > 0 && (
            <p className="text-xs text-muted-foreground">
              Total download: ~{(totalDownload / 1000).toFixed(1)} GB · One-time setup, runs offline after.
            </p>
          )}

          <Button className="w-full" size="lg" onClick={handleStartDownloadAndIndex}>
            <Download className="w-4 h-4 mr-2" />
            {totalDownload > 0 ? `Download & Index ${folderName}` : `Index ${folderName}`}
          </Button>

          {!gemmaAlreadyDownloaded && includeGemma && (
            <button
              onClick={() => { setIncludeGemma(false); }}
              className="block w-full text-center text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              Skip understanding model
            </button>
          )}
        </div>
      </div>
    );
  }

  // ===== STEP 3: DOWNLOADING =====
  if (step === "downloading") {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center max-w-sm space-y-6">
          <Loader2 className="w-12 h-12 text-primary mx-auto animate-spin" />
          <div className="space-y-1">
            <h2 className="text-lg font-medium">{downloadPhase}</h2>
            <p className="text-sm text-muted-foreground">
              {scanResult ? `${scanResult.file_count.toLocaleString()} files ready to index` : ""}
            </p>
          </div>
        </div>
      </div>
    );
  }

  // ===== STEP 4: INDEXING (handled by DashboardIndexing) =====
  return (
    <div className="flex items-center justify-center h-full">
      <div className="text-center max-w-sm space-y-6">
        <Sparkles className="w-12 h-12 text-primary mx-auto animate-pulse" />
        <div className="space-y-1">
          <h2 className="text-lg font-medium">Understanding your files...</h2>
          <p className="text-sm text-muted-foreground">
            This takes a few minutes. Cosmos is reading, embedding, and clustering {folderName}.
          </p>
        </div>
      </div>
    </div>
  );
};
