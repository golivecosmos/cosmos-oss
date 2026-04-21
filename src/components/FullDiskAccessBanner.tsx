import { useEffect, useState } from "react";
import { AlertCircle, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button } from "./ui/button";

const FDA_DEEP_LINK =
  "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";

export function FullDiskAccessBanner() {
  const [needsFda, setNeedsFda] = useState<boolean>(false);
  const [dismissed, setDismissed] = useState<boolean>(false);

  useEffect(() => {
    let cancelled = false;
    invoke<boolean | null>("check_full_disk_access")
      .then((granted) => {
        if (cancelled) return;
        setNeedsFda(granted === false);
      })
      .catch(() => {
        // Probe failed; don't show the banner (avoids false positives on
        // unusual systems that lack the probe path).
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!needsFda || dismissed) return null;

  return (
    <div className="border-b border-amber-300 bg-amber-50 dark:border-amber-700 dark:bg-amber-950 px-4 py-2 flex items-center gap-3 text-sm">
      <AlertCircle className="h-4 w-4 text-amber-700 dark:text-amber-300 flex-shrink-0" />
      <div className="flex-1 text-amber-900 dark:text-amber-100">
        <span className="font-medium">Full Disk Access needed.</span>{" "}
        Cosmos can't read your Library or Documents folders until you grant it
        in System Settings.
      </div>
      <Button
        size="sm"
        variant="outline"
        className="border-amber-400 text-amber-900 dark:border-amber-600 dark:text-amber-100"
        onClick={() => {
          openUrl(FDA_DEEP_LINK).catch((err) => {
            console.error("Failed to open System Settings:", err);
          });
        }}
      >
        Open System Settings
      </Button>
      <button
        onClick={() => setDismissed(true)}
        aria-label="Dismiss Full Disk Access banner"
        className="text-amber-700 dark:text-amber-300 hover:text-amber-900 dark:hover:text-amber-100"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}
