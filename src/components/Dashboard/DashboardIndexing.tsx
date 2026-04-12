import React, { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Loader2 } from "lucide-react";

type IndexingPhase = "scanning" | "embedding" | "clustering" | "understanding";

interface IndexingProgress {
  filesFound: number;
  filesProcessed: number;
  totalFiles: number;
  phase: IndexingPhase;
}

export const DashboardIndexing: React.FC = () => {
  const [progress, setProgress] = useState<IndexingProgress>({
    filesFound: 0,
    filesProcessed: 0,
    totalFiles: 0,
    phase: "scanning",
  });

  useEffect(() => {
    let unlistenScan: (() => void) | null = null;
    let unlistenBulk: (() => void) | null = null;

    const setup = async () => {
      unlistenScan = await listen("scan_progress", (event: any) => {
        const payload = event?.payload;
        if (payload) {
          setProgress((p) => ({
            ...p,
            filesFound: payload.files_found || p.filesFound,
            phase: "scanning",
          }));
        }
      });

      unlistenBulk = await listen("bulk_index_progress", (event: any) => {
        const payload = event?.payload;
        if (payload) {
          setProgress((p) => ({
            ...p,
            filesProcessed: payload.processed || p.filesProcessed,
            totalFiles: payload.total || p.totalFiles,
            phase: payload.status === "completed" ? "clustering" : "embedding",
          }));
        }
      });
    };

    setup();

    return () => {
      if (unlistenScan) unlistenScan();
      if (unlistenBulk) unlistenBulk();
    };
  }, []);

  const phaseName: Record<IndexingPhase, string> = {
    scanning: "Scanning files",
    embedding: "Embedding files",
    clustering: "Clustering",
    understanding: "Understanding (Gemma 4)",
  };

  const percent =
    progress.totalFiles > 0
      ? Math.round((progress.filesProcessed / progress.totalFiles) * 100)
      : 0;

  return (
    <div className="flex items-center justify-center h-full">
      <div className="text-center max-w-sm space-y-6">
        <Loader2 className="w-12 h-12 text-primary mx-auto animate-spin" />

        <div className="space-y-1">
          <h2 className="text-lg font-medium">
            {phaseName[progress.phase]}
          </h2>
          <p className="text-sm text-muted-foreground">
            {progress.phase === "scanning" && progress.filesFound > 0
              ? `Found ${progress.filesFound.toLocaleString()} files...`
              : progress.totalFiles > 0
              ? `${progress.filesProcessed.toLocaleString()} of ${progress.totalFiles.toLocaleString()} files`
              : "Preparing..."}
          </p>
        </div>

        {progress.totalFiles > 0 && (
          <div className="w-full space-y-1">
            <div className="h-2 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-primary rounded-full transition-all duration-500"
                style={{ width: `${percent}%` }}
              />
            </div>
            <p className="text-xs text-muted-foreground">{percent}%</p>
          </div>
        )}
      </div>
    </div>
  );
};
