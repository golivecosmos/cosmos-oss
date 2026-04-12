import React, { useCallback } from "react";
import { FolderOpen, Shield, Brain } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "../ui/button";

interface DashboardEmptyProps {
  onSelectFolder: (path: string) => void;
}

export const DashboardEmpty: React.FC<DashboardEmptyProps> = ({
  onSelectFolder,
}) => {
  const handleChooseFolder = useCallback(async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose a folder to index",
    });
    if (selected && typeof selected === "string") {
      onSelectFolder(selected);
    }
  }, [onSelectFolder]);

  return (
    <div className="flex items-center justify-center h-full">
      <div className="text-center max-w-md space-y-6">
        <div className="mx-auto w-20 h-20 rounded-2xl bg-primary/10 flex items-center justify-center">
          <FolderOpen className="w-10 h-10 text-primary" />
        </div>

        <div className="space-y-2">
          <h1 className="text-2xl font-semibold">Drop a folder to get started</h1>
          <p className="text-muted-foreground text-sm leading-relaxed">
            Cosmos indexes your files locally and shows you what's inside...
            topics, patterns, and connections you didn't know existed.
          </p>
        </div>

        <Button size="lg" onClick={handleChooseFolder}>
          Choose Folder
        </Button>
        <p className="text-xs text-muted-foreground">or drag & drop a folder anywhere</p>

        <div className="flex items-center justify-center gap-6 text-xs text-muted-foreground pt-4">
          <span className="flex items-center gap-1.5">
            <Shield className="w-3.5 h-3.5" />
            Everything stays on your machine
          </span>
          <span className="flex items-center gap-1.5">
            <Brain className="w-3.5 h-3.5" />
            No cloud, no API keys
          </span>
        </div>
      </div>
    </div>
  );
};
