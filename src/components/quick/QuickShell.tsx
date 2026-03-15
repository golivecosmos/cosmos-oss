import { FormEvent, KeyboardEvent, MouseEvent, startTransition, useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { ArrowUpRight, Clock3, Search } from "lucide-react";

import { useSearch, SemanticFileTypeFilter } from "../../hooks/useSearch";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../ui/select";
import { cn } from "../../lib/utils";

const QUICK_PANEL_FOCUS_EVENT = "quick_panel:focus_search";
const QUICK_SHORTCUT = "Cmd/Ctrl+Shift+Space";
const RECENT_QUERIES_KEY = "recent_queries";
const RECENT_FILES_KEY = "recent_files";
const PINNED_ACTIONS_KEY = "pinned_actions";
const DEFAULT_PINNED_ACTIONS = ["open_full_app"];
const MAX_RECENT_QUERIES = 10;
const MAX_RECENT_FILES = 20;
const RESULT_LIMIT = 20;
const COLLAPSED_WINDOW_SIZE = { width: 920, height: 128 };
const EXPANDED_WINDOW_WIDTH = 1120;
const EXPANDED_MIN_HEIGHT = 420;
const EXPANDED_MAX_HEIGHT = 760;
const EXPANDED_BASE_HEIGHT = 360;
const EXPANDED_PER_RESULT_HEIGHT = 34;
const EXPANDED_RESULT_COUNT_CAP = 10;
const quickPanelWindow = getCurrentWindow();

interface QuickSearchResult {
  file_path: string;
  score: number;
  source_type?: string | null;
  snippet?: string | null;
  timestamp?: number | null;
  mime_type?: string | null;
  metadata?: string;
}

interface FilePreviewPayload {
  content: string;
}

function loadPersistedArray(key: string, fallback: string[] = []): string[] {
  try {
    const value = localStorage.getItem(key);
    if (!value) return fallback;
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) ? parsed.filter((entry) => typeof entry === "string") : fallback;
  } catch {
    return fallback;
  }
}

function savePersistedArray(key: string, values: string[]): void {
  localStorage.setItem(key, JSON.stringify(values));
}

function normalizePath(path: string): string {
  if (path.startsWith("asset://localhost/")) {
    try {
      return decodeURIComponent(path.replace("asset://localhost", ""));
    } catch {
      return path.replace("asset://localhost", "");
    }
  }
  return path;
}

function getFileExtension(path: string): string {
  return path.split(".").pop()?.toLowerCase() ?? "";
}

function inferPreviewKind(result: QuickSearchResult): "image" | "video" | "audio" | "document" {
  const metadata = parseResultMetadata(result);
  const metadataSourceType = typeof metadata.source_type === "string" ? metadata.source_type.toLowerCase() : "";
  const mime = result.mime_type?.toLowerCase() ?? "";
  const ext = getFileExtension(result.file_path);
  if (metadataSourceType === "video_frame" || metadataSourceType === "transcript_chunk") {
    return "video";
  }
  if (mime.startsWith("image/") || ["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff"].includes(ext)) {
    return "image";
  }
  if (mime.startsWith("video/") || ["mp4", "mov", "mkv", "avi", "webm", "m4v", "wmv", "flv"].includes(ext)) {
    return "video";
  }
  if (mime.startsWith("audio/") || ["mp3", "wav", "ogg", "m4a", "aac", "flac"].includes(ext)) {
    return "audio";
  }
  return "document";
}

function displayName(path: string): string {
  const normalized = normalizePath(path).replace(/\\/g, "/");
  return normalized.split("/").pop() ?? normalized;
}

function displaySource(sourceType?: string | null): string {
  if (!sourceType) return "indexed";
  if (sourceType === "text_chunk") return "text";
  if (sourceType === "transcript_chunk") return "transcript";
  if (sourceType === "video_frame") return "video frame";
  return sourceType.replace(/_/g, " ");
}

function parseResultMetadata(result: QuickSearchResult): Record<string, any> {
  if (!result.metadata || typeof result.metadata !== "string") {
    return {};
  }

  try {
    return JSON.parse(result.metadata);
  } catch {
    return {};
  }
}

function resolveSourceType(result: QuickSearchResult): string | undefined {
  if (result.source_type && result.source_type.trim().length > 0) {
    return result.source_type;
  }
  const metadata = parseResultMetadata(result);
  if (typeof metadata.source_type === "string" && metadata.source_type.trim().length > 0) {
    return metadata.source_type;
  }
  return undefined;
}

function resolveResultTimestamp(result: QuickSearchResult): number | undefined {
  if (typeof result.timestamp === "number" && !Number.isNaN(result.timestamp)) {
    return result.timestamp;
  }

  const metadata = parseResultMetadata(result);
  const candidateKeys = [
    "time_start_seconds",
    "timestamp",
    "best_match_timestamp",
    "start_time",
    "start_timestamp",
  ];

  for (const key of candidateKeys) {
    const value = metadata[key];
    if (typeof value === "number" && Number.isFinite(value)) {
      return value;
    }
  }

  return undefined;
}

export function QuickShell() {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const blurHideTimeoutRef = useRef<number | null>(null);
  const isHidePendingRef = useRef(false);
  const [isVisible, setIsVisible] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [semanticFilter, setSemanticFilter] = useState<SemanticFileTypeFilter>("all");
  const [previewText, setPreviewText] = useState<string | null>(null);
  const [recentQueries, setRecentQueries] = useState<string[]>(() => loadPersistedArray(RECENT_QUERIES_KEY));
  const [pinnedActions, setPinnedActions] = useState<string[]>(() =>
    loadPersistedArray(PINNED_ACTIONS_KEY, DEFAULT_PINNED_ACTIONS)
  );
  const { searchState, handleSearch, clearSearch } = useSearch();
  const results = (searchState.results as QuickSearchResult[]).slice(0, RESULT_LIMIT);
  const selectedResult = results[selectedIndex] ?? null;
  const selectedTimestamp = selectedResult ? resolveResultTimestamp(selectedResult) : undefined;
  const isExpanded = query.trim().length > 0 || searchState.isSearching || results.length > 0;
  const expandedResultCount = Math.min(results.length, EXPANDED_RESULT_COUNT_CAP);
  const expandedHeight = Math.min(
    EXPANDED_MAX_HEIGHT,
    Math.max(
      EXPANDED_MIN_HEIGHT,
      EXPANDED_BASE_HEIGHT + expandedResultCount * EXPANDED_PER_RESULT_HEIGHT + (searchState.isSearching ? 48 : 0)
    )
  );

  useEffect(() => {
    const target = isExpanded
      ? { width: EXPANDED_WINDOW_WIDTH, height: expandedHeight }
      : COLLAPSED_WINDOW_SIZE;
    void quickPanelWindow
      .setSize(new LogicalSize(target.width, target.height))
      .catch((error) => {
        console.error("Quick panel resize failed", error);
      });
  }, [expandedHeight, isExpanded]);

  const handleWindowDragStart = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    void quickPanelWindow.startDragging().catch((error) => {
      console.error("Quick panel drag failed", error);
    });
  };

  const handlePanelMouseDownCapture = (event: MouseEvent<HTMLDivElement>) => {
    const target = event.target as HTMLElement | null;
    if (!target) return;
    if (
      target.closest(
        "input, textarea, button, select, option, a, video, audio, iframe, [role='button'], [data-no-drag='true']"
      )
    ) {
      return;
    }
    handleWindowDragStart(event);
  };

  const showQuickPanelAnimated = () => {
    if (blurHideTimeoutRef.current) {
      window.clearTimeout(blurHideTimeoutRef.current);
      blurHideTimeoutRef.current = null;
    }
    isHidePendingRef.current = false;
    window.requestAnimationFrame(() => {
      setIsVisible(true);
    });
  };

  const hideQuickPanelAnimated = () => {
    if (isHidePendingRef.current) {
      return;
    }
    isHidePendingRef.current = true;
    setIsVisible(false);

    window.setTimeout(() => {
      void invoke("hide_quick_panel").finally(() => {
        isHidePendingRef.current = false;
      });
    }, 140);
  };

  useEffect(() => {
    if (pinnedActions.length === 0) {
      setPinnedActions(DEFAULT_PINNED_ACTIONS);
      savePersistedArray(PINNED_ACTIONS_KEY, DEFAULT_PINNED_ACTIONS);
      return;
    }
    savePersistedArray(PINNED_ACTIONS_KEY, pinnedActions);
  }, [pinnedActions]);

  useEffect(() => {
    const trimmed = query.trim();
    if (!trimmed) {
      clearSearch();
      return;
    }

    const timeout = window.setTimeout(() => {
      startTransition(() => {
        handleSearch(trimmed, "text", { semanticFileTypeFilter: semanticFilter });
      });
    }, 150);

    return () => window.clearTimeout(timeout);
  }, [clearSearch, handleSearch, query, semanticFilter]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [searchState.query, semanticFilter]);

  useEffect(() => {
    if (selectedIndex < results.length) return;
    setSelectedIndex(results.length === 0 ? 0 : results.length - 1);
  }, [results.length, selectedIndex]);

  useEffect(() => {
    let cancelled = false;

    const loadPreview = async () => {
      setPreviewText(null);
      if (!selectedResult) return;
      if (selectedResult.snippet && selectedResult.snippet.trim().length > 0) {
        setPreviewText(selectedResult.snippet.trim());
        return;
      }
      if (inferPreviewKind(selectedResult) !== "document") return;
      if (getFileExtension(selectedResult.file_path) === "pdf") return;

      try {
        const payload = await invoke<FilePreviewPayload>("read_file_preview", {
          path: normalizePath(selectedResult.file_path),
          maxBytes: 65536,
        });
        if (!cancelled) {
          setPreviewText(payload.content?.trim() || null);
        }
      } catch {
        if (!cancelled) {
          setPreviewText(null);
        }
      }
    };

    loadPreview();
    return () => {
      cancelled = true;
    };
  }, [selectedResult]);

  useEffect(() => {
    if (typeof selectedTimestamp !== "number" || Number.isNaN(selectedTimestamp)) {
      return;
    }

    const seekToTimestamp = (media: HTMLMediaElement) => {
      const seek = Math.max(0, selectedTimestamp);
      if (media.readyState >= 1) {
        const maxSeek = Number.isFinite(media.duration) && media.duration > 0 ? media.duration - 0.05 : seek;
        media.currentTime = Math.min(seek, Math.max(0, maxSeek));
        return;
      }

      const onLoadedMetadata = () => {
        const maxSeek = Number.isFinite(media.duration) && media.duration > 0 ? media.duration - 0.05 : seek;
        media.currentTime = Math.min(seek, Math.max(0, maxSeek));
      };

      media.addEventListener("loadedmetadata", onLoadedMetadata, { once: true });
    };

    if (videoRef.current) {
      seekToTimestamp(videoRef.current);
    }
    if (audioRef.current) {
      seekToTimestamp(audioRef.current);
    }
  }, [selectedResult?.file_path, selectedTimestamp]);

  useEffect(() => {
    showQuickPanelAnimated();

    inputRef.current?.focus();

    let unlisten: (() => void) | undefined;
    const setupFocusListener = async () => {
      unlisten = await listen(QUICK_PANEL_FOCUS_EVENT, () => {
        showQuickPanelAnimated();
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    };
    setupFocusListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    let unlistenFocus: (() => void) | undefined;

    const scheduleHide = () => {
      if (blurHideTimeoutRef.current) {
        window.clearTimeout(blurHideTimeoutRef.current);
        blurHideTimeoutRef.current = null;
      }

      // Defer slightly so transient focus churn does not flap the panel.
      blurHideTimeoutRef.current = window.setTimeout(() => {
        hideQuickPanelAnimated();
      }, 80);
    };

    const cancelHide = () => {
      if (blurHideTimeoutRef.current) {
        window.clearTimeout(blurHideTimeoutRef.current);
        blurHideTimeoutRef.current = null;
      }
      isHidePendingRef.current = false;
      setIsVisible(true);
    };

    const setupFocusChangedListener = async () => {
      unlistenFocus = await quickPanelWindow.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          cancelHide();
          return;
        }
        scheduleHide();
      });
    };

    setupFocusChangedListener();

    const handleWindowBlur = () => {
      scheduleHide();
    };

    const handleVisibilityChange = () => {
      if (document.hidden) {
        scheduleHide();
      } else {
        cancelHide();
      }
    };

    window.addEventListener("blur", handleWindowBlur);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      if (unlistenFocus) {
        unlistenFocus();
      }
      window.removeEventListener("blur", handleWindowBlur);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      if (blurHideTimeoutRef.current) {
        window.clearTimeout(blurHideTimeoutRef.current);
        blurHideTimeoutRef.current = null;
      }
      isHidePendingRef.current = false;
      setIsVisible(false);
    };
  }, []);

  const persistRecentQuery = (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) return;
    const updated = [trimmed, ...recentQueries.filter((item) => item !== trimmed)].slice(0, MAX_RECENT_QUERIES);
    setRecentQueries(updated);
    savePersistedArray(RECENT_QUERIES_KEY, updated);
  };

  const persistRecentFile = (path: string) => {
    const updated = [path, ...loadPersistedArray(RECENT_FILES_KEY).filter((item) => item !== path)].slice(
      0,
      MAX_RECENT_FILES
    );
    savePersistedArray(RECENT_FILES_KEY, updated);
  };

  const openSelectedResult = async (result?: QuickSearchResult | null) => {
    const target = result ?? selectedResult;
    if (!target) return;
    const filePath = normalizePath(target.file_path);
    await invoke("open_with_default_app", { path: filePath });
    persistRecentFile(filePath);
  };

  const openFullApp = async () => {
    const selectedSource = selectedResult ? resolveSourceType(selectedResult) : undefined;
    const payload = {
      query: query.trim() || undefined,
      selectedPath: selectedResult ? normalizePath(selectedResult.file_path) : undefined,
      timestamp: selectedTimestamp,
      source: selectedSource,
      semanticFileTypeFilter: semanticFilter,
    };
    await invoke("open_full_app", { payload });
  };

  const dismissQuickPanel = async () => {
    hideQuickPanelAnimated();
  };

  const onSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = query.trim();
    if (!trimmed) return;
    persistRecentQuery(trimmed);
    await handleSearch(trimmed, "text", { semanticFileTypeFilter: semanticFilter });
  };

  const onQueryChange = (value: string) => {
    setQuery(value);
    if (!value.trim()) {
      clearSearch();
      setSelectedIndex(0);
    }
  };

  const onInputKeyDown = async (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (results.length === 0) return;
      setSelectedIndex((prev) => (prev + 1) % results.length);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (results.length === 0) return;
      setSelectedIndex((prev) => (prev - 1 + results.length) % results.length);
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      await dismissQuickPanel();
      return;
    }

    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      await openFullApp();
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      persistRecentQuery(query);
      if (results.length > 0) {
        await openSelectedResult();
      } else if (query.trim()) {
        await handleSearch(query.trim(), "text", { semanticFileTypeFilter: semanticFilter });
      }
    }
  };

  const renderPreview = () => {
    if (!selectedResult) {
      return (
        <div className="h-full flex items-center justify-center text-sm text-gray-500 dark:text-customGray">
          Select a result to preview
        </div>
      );
    }

    const kind = inferPreviewKind(selectedResult);
    const filePath = normalizePath(selectedResult.file_path);
    const previewSrc = convertFileSrc(filePath);
    const extension = getFileExtension(filePath);

    if (kind === "image") {
      return (
        <div className="h-full w-full flex items-center justify-center">
          <img src={previewSrc} alt={displayName(filePath)} className="max-h-full max-w-full object-contain rounded-lg" />
        </div>
      );
    }

    if (kind === "video") {
      return (
        <div className="h-full w-full flex items-center justify-center">
          <video
            key={`${filePath}:${selectedTimestamp ?? "no-ts"}`}
            ref={videoRef}
            src={previewSrc}
            controls
            preload="metadata"
            className="h-full w-full rounded-lg bg-black/80 object-contain"
          >
            <track kind="captions" />
          </video>
        </div>
      );
    }

    if (kind === "audio") {
      return (
        <div className="h-full w-full flex items-center justify-center">
          <audio
            key={`${filePath}:${selectedTimestamp ?? "no-ts"}`}
            ref={audioRef}
            src={previewSrc}
            controls
            preload="metadata"
            className="w-full max-w-lg"
          >
            <track kind="captions" />
          </audio>
        </div>
      );
    }

    if (extension === "pdf") {
      return (
        <iframe
          src={`${previewSrc}#toolbar=0`}
          className="h-full w-full rounded-lg border-0 bg-white"
          title={displayName(filePath)}
        />
      );
    }

    return (
      <div className="h-full w-full overflow-auto rounded-lg border border-gray-200 dark:border-darkBgHighlight bg-white/70 dark:bg-darkBg/70 p-4">
        <pre className="whitespace-pre-wrap break-words text-xs text-gray-700 dark:text-gray-200">
          {previewText ?? "No preview available for this file."}
        </pre>
      </div>
    );
  };

  return (
    <div className="h-screen w-screen overflow-hidden rounded-[24px] bg-[transparent] p-2 text-gray-900 dark:text-white">
      <div
        onMouseDownCapture={handlePanelMouseDownCapture}
        className={cn(
          "mx-auto flex h-full w-full max-w-[1120px] flex-col gap-4 rounded-[22px] bg-transparent p-4 transition-[opacity,transform] duration-180 ease-out",
          isVisible ? "opacity-100 scale-100" : "opacity-0 scale-[0.985]",
          isExpanded ? "justify-start" : "justify-center"
        )}
      >
        <div
          data-tauri-drag-region
          onMouseDown={handleWindowDragStart}
          className="flex h-6 w-full cursor-grab items-center justify-center rounded-md active:cursor-grabbing"
          title="Drag to move quick panel"
        >
          <div className="h-1.5 w-20 rounded-full bg-slate-300/90 dark:bg-darkBgHighlight" />
        </div>
        <div className="rounded-2xl border border-gray-200 dark:border-darkBgHighlight bg-white/90 dark:bg-darkBgMid/90 shadow-lg">
          <form onSubmit={onSubmit} className="flex items-center gap-3 px-4 py-3">
            <div
              className="flex h-11 w-8 shrink-0 cursor-grab items-center justify-center rounded-md active:cursor-grabbing"
              title="Drag to move quick panel"
            >
              <img
                src="/cosmos-logo.webp"
                alt="Cosmos"
                className="h-5 w-5 pointer-events-none select-none opacity-80"
              />
            </div>
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
              <Input
                ref={inputRef}
                value={query}
                onChange={(event) => onQueryChange(event.target.value)}
                onKeyDown={onInputKeyDown}
                placeholder="Search your files semantically..."
                className="h-11 rounded-xl border-gray-200 bg-white pl-9 pr-3 dark:border-darkBgHighlight dark:bg-darkBg"
              />
            </div>
            <Select
              value={semanticFilter}
              onValueChange={(value) => setSemanticFilter(value as SemanticFileTypeFilter)}
            >
              <SelectTrigger className="h-11 w-36 rounded-xl border-gray-200 bg-white dark:border-darkBgHighlight dark:bg-white">
                <SelectValue placeholder="All Files" />
              </SelectTrigger>
              <SelectContent className="border-gray-200 bg-white text-gray-900 dark:border-darkBgHighlight dark:bg-white dark:text-gray-900">
                <SelectItem value="all">All Files</SelectItem>
                <SelectItem value="image">Images</SelectItem>
                <SelectItem value="video">Videos</SelectItem>
                <SelectItem value="audio">Audio</SelectItem>
                <SelectItem value="document">Documents</SelectItem>
              </SelectContent>
            </Select>
            <Button type="button" onClick={openFullApp} className="h-11 rounded-xl px-4">
              Open Full App
            </Button>
          </form>
        </div>

        {isExpanded && (
          <div className="grid min-h-0 flex-1 grid-cols-[minmax(320px,0.42fr)_minmax(420px,1fr)] gap-4">
          <div className="min-h-0 flex flex-col rounded-2xl border border-gray-200 dark:border-darkBgHighlight bg-white/90 dark:bg-darkBgMid/90 p-2 shadow-lg">
            <div className="mb-2 px-2 text-xs font-medium uppercase tracking-wide text-gray-500 dark:text-customGray">
              Results {searchState.isSearching ? "· searching..." : `· ${results.length}`}
            </div>
            <div className="min-h-0 flex-1 overflow-auto pb-1">
              {results.length === 0 ? (
                <div className="space-y-3 px-3 py-4">
                  <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-customGray">
                    <Clock3 className="h-4 w-4" />
                    Recent queries
                  </div>
                  {recentQueries.length === 0 ? (
                    <p className="text-xs text-gray-500 dark:text-customGray">No recent queries yet.</p>
                  ) : (
                    recentQueries.map((recentQuery) => (
                      <button
                        key={recentQuery}
                        type="button"
                        onClick={() => setQuery(recentQuery)}
                        className="block w-full rounded-lg px-2 py-2 text-left text-sm hover:bg-gray-100 dark:hover:bg-darkBgHighlight"
                      >
                        {recentQuery}
                      </button>
                    ))
                  )}
                </div>
              ) : (
                results.map((result, index) => {
                  const isActive = index === selectedIndex;
                  const resultSource = resolveSourceType(result);
                  const resultTimestamp = resolveResultTimestamp(result);
                  return (
                    <button
                      key={`${result.file_path}-${index}`}
                      type="button"
                      onMouseEnter={() => setSelectedIndex(index)}
                      onClick={() => setSelectedIndex(index)}
                      onDoubleClick={() => openSelectedResult(result)}
                      className={cn(
                        "mb-1 w-full rounded-xl border px-3 py-2 text-left transition-colors",
                        isActive
                          ? "border-blue-300 bg-blue-50 dark:border-blueShadow dark:bg-darkBgHighlight"
                          : "border-transparent hover:border-gray-200 hover:bg-gray-50 dark:hover:border-darkBgHighlight dark:hover:bg-darkBg"
                      )}
                    >
                      <p className="truncate text-sm font-medium">{displayName(result.file_path)}</p>
                      <p className="truncate text-xs text-gray-500 dark:text-customGray">{normalizePath(result.file_path)}</p>
                      <div className="mt-1 flex items-center gap-2 text-[11px] text-gray-500 dark:text-customGray">
                        <span>{displaySource(resultSource)}</span>
                        <span>score {result.score.toFixed(4)}</span>
                        {typeof resultTimestamp === "number" && <span>@ {resultTimestamp.toFixed(1)}s</span>}
                      </div>
                    </button>
                  );
                })
              )}
            </div>
          </div>

          <div className="min-h-0 flex flex-col rounded-2xl border border-gray-200 dark:border-darkBgHighlight bg-white/90 dark:bg-darkBgMid/90 p-3 shadow-lg">
            <div className="mb-2 flex items-center justify-between text-xs text-gray-500 dark:text-customGray">
              <span className="truncate">{selectedResult ? normalizePath(selectedResult.file_path) : "Preview"}</span>
              <div className="flex items-center gap-2">
                {pinnedActions.includes("open_full_app") && (
                  <Button type="button" variant="ghost" size="sm" onClick={openFullApp} className="h-7 text-xs">
                    <ArrowUpRight className="mr-1 h-3 w-3" />
                    Full App
                  </Button>
                )}
                <Button type="button" variant="ghost" size="sm" onClick={dismissQuickPanel} className="h-7 text-xs">
                  Esc
                </Button>
              </div>
            </div>
            <div className="min-h-0 flex-1 overflow-hidden rounded-xl bg-gray-100/70 p-2 dark:bg-darkBg/70">
              {renderPreview()}
            </div>
          </div>
          </div>
        )}

        {isExpanded && (
          <div className="flex items-center justify-between px-1 pb-1 text-xs text-gray-500 dark:text-customGray">
            <div className="flex items-center gap-2">
              <Search className="h-3.5 w-3.5" />
              <span>{`Shortcut: ${QUICK_SHORTCUT}`}</span>
            </div>
            <div className="flex items-center gap-3">
              <span>Up/Down navigate</span>
              <span>Enter open file</span>
              <span>Cmd/Ctrl+Enter full app</span>
              <span>Esc close</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
