import React, { createContext, useContext, useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { resolve } from "@tauri-apps/api/path";
import { FileItem } from "../components/FileTree";
import { ReferenceImageData } from "../components/SearchBar";
import { useSearch, SearchOptions, SearchType } from "../hooks/useSearch";
import { getErrorMessage } from "../utils/errorMessage";
import {
  useIndexingJobs,
} from "./IndexingJobsContext";

// Model download states - clean state machine
type ModelDownloadState =
  | "checking" // Initial check
  | "ready" // Models available and loaded
  | "downloading" // Download in progress
  | "failed" // Download failed
  | "installing"; // Installing/loading models

interface ModelDownloadProgress {
  state: ModelDownloadState;
  progress: number; // 0-100
  currentFile?: string;
  error?: string;
  filesCompleted: number;
  totalFiles: number;
}

// Drive management types
interface DriveMetadata {
  uuid: string;
  name: string;
  custom_name?: string | null;
  physical_location?: string | null;
  last_mount_path?: string | null;
  total_space: number;
  free_space: number;
  is_removable: boolean;
  first_seen: string;
  last_seen: string;
  status: 'connected' | 'disconnected' | 'indexing' | 'error';
  indexed_files_count: number;
  total_size_indexed: number;
}

interface DriveInfo {
  uuid: string;
  name: string;
  custom_name?: string | null;
  physical_location?: string | null;
  mount_path: string;
  status: 'connected' | 'disconnected' | 'indexing' | 'error';
  indexed_files_count: number;
  is_removable: boolean;
}

interface AppLayoutContextType {
  // File and collection state
  selectedCollection: string;
  setSelectedCollection: (collection: string) => void;
  indexingPaths: Set<string>;
  setIndexingPaths: (paths: Set<string>) => void;
  transcribingPaths: Set<string>;
  setTranscribingPaths: (paths: Set<string>) => void;
  handleTranscribeFile: (path: string) => Promise<void>;
  isWatchedDialogOpen: boolean;
  setIsWatchedDialogOpen: (open: boolean) => void;

  // Drive management state
  drives: DriveInfo[];
  isDrivesLoading: boolean;
  loadDrives: () => Promise<void>;
  updateDrive: (uuid: string, customName: string | null, physicalLocation: string | null) => Promise<void>;
  deleteDrive: (uuid: string) => Promise<void>;

  // Model download state
  modelDownload: ModelDownloadProgress;
  retryModelDownload: () => void;

  // UI state for modals and overlays
  showOnboarding: boolean;
  setShowOnboarding: (show: boolean) => void;
  hasCompletedOnboarding: boolean;
  setHasCompletedOnboarding: (completed: boolean) => void;
  showInteractiveTour: boolean;
  setShowInteractiveTour: (show: boolean) => void;
  showSettings: boolean;
  setShowSettings: (show: boolean) => void;
  showBugReport: boolean;
  setShowBugReport: (show: boolean) => void;
  showBenchmark: boolean;
  setShowBenchmark: (show: boolean) => void;
  showIndexingInfo: boolean;
  setShowIndexingInfo: (show: boolean) => void;
  showAppStore: boolean;
  setShowAppStore: (show: boolean) => void;

  // Search state
  searchState: any;
  handleSearch: (query: string, type: SearchType, options?: SearchOptions) => Promise<void>;
  clearSearch: () => void;
  refreshCurrentSearch: () => void;

  // Reference image state
  referenceImage: ReferenceImageData | null;
  setReferenceImage: (image: ReferenceImageData | null) => void;
  showReferenceImage: boolean;
  setShowReferenceImage: (show: boolean) => void;

  // Indexing jobs state
  indexedCount: number;
  hasActiveJobs: boolean;
  hasFailedJobs: boolean;
  loadIndexedCount: () => Promise<void>;
  recoverInterruptedJobs: () => Promise<void>;
  loadJobs: () => Promise<void>;

  // Handler functions
  handleFileUpload: (file: File) => void;
  handleAddToFavorites: (file: FileItem) => Promise<void>;
  handleShare: (file: FileItem) => Promise<void>;
  handleDelete: (file: FileItem) => Promise<void>;
  handleCreateFolder: (parent: FileItem) => Promise<void>;
  handleCollectionSelect: (collection: any) => void;
  handleOnboardingComplete: () => void;
  handleRestartOnboarding: () => void;
  handleOnboardingDismiss: () => void;
  handleTourComplete: () => void;
  handleTourDismiss: () => void;
  handleStartTour: () => void;
  handleOpenSettings: () => void;
  handleCloseSettings: () => void;
  handleRestartTourFromSettings: () => void;
  handleOpenBugReport: () => void;
  handleCloseBugReport: () => void;
  handleOpenBenchmark: () => void;
  handleOpenAppStore: () => void;
  handleCloseAppStore: () => void;
  isIndexingAllowed: () => boolean;
  showIndexingWarning: () => void;
  handleBulkIndex: (item: FileItem) => Promise<void>;
  handleReferenceImageClose: () => void;
  handleAddToIndex: (path: string) => Promise<void>;
}

const AppLayoutContext = createContext<AppLayoutContextType | undefined>(undefined);

export const useAppLayout = () => {
  const context = useContext(AppLayoutContext);
  if (!context) {
    throw new Error("useAppLayout must be used within an AppLayoutProvider");
  }
  return context;
};

interface AppLayoutProviderProps {
  children: React.ReactNode;
}

export const AppLayoutProvider: React.FC<AppLayoutProviderProps> = ({ children }) => {
  const [selectedCollection, setSelectedCollection] = useState<string>("indexed");
  const [indexingPaths, setIndexingPaths] = useState<Set<string>>(new Set());
  const [transcribingPaths, setTranscribingPaths] = useState<Set<string>>(new Set());
  const [isWatchedDialogOpen, setIsWatchedDialogOpen] = useState(false);

  // Drive management state
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [isDrivesLoading, setIsDrivesLoading] = useState(true);

  // Simplified model state - single source of truth
  const [modelDownload, setModelDownload] = useState<ModelDownloadProgress>({
    state: "checking",
    progress: 0,
    filesCompleted: 0,
    totalFiles: 0,
  });

  const [showOnboarding, setShowOnboarding] = useState(false);
  const [hasCompletedOnboarding, setHasCompletedOnboarding] = useState(false);
  const [showInteractiveTour, setShowInteractiveTour] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showBugReport, setShowBugReport] = useState(false);
  const [showBenchmark, setShowBenchmark] = useState(false);
  const [showIndexingInfo, setShowIndexingInfo] = useState(false);
  const [showAppStore, setShowAppStore] = useState(false);

  const { searchState, handleSearch, clearSearch, refreshCurrentSearch } = useSearch();
  const downloadInProgressRef = useRef(false);
  const modelReloadInProgressRef = useRef(false);
  const progressUpdateTimeout = useRef<NodeJS.Timeout | null>(null);
  const fileProgressRef = useRef<Record<string, number>>({});

  const [referenceImage, setReferenceImage] = useState<ReferenceImageData | null>(null);
  const [showReferenceImage, setShowReferenceImage] = useState(false);

  const {
    indexedCount,
    hasActiveJobs,
    hasFailedJobs,
    loadIndexedCount,
    recoverInterruptedJobs,
    loadJobs,
  } = useIndexingJobs();

  // Drive management functions
  const loadDrives = useCallback(async () => {
    try {
      setIsDrivesLoading(true);
      await invoke('sync_drives_to_database');
      const allDrives = await invoke<DriveMetadata[]>('get_all_drives_with_metadata');
      const mappedDrives = allDrives.map(drive => ({
        uuid: drive.uuid,
        name: drive.name,
        custom_name: drive.custom_name,
        physical_location: drive.physical_location,
        mount_path: drive.last_mount_path || '',
        status: drive.status,
        indexed_files_count: drive.indexed_files_count,
        is_removable: drive.is_removable
      }));
      setDrives(mappedDrives);
    } catch (err) {
      console.error('Failed to load drives:', err);
    } finally {
      setIsDrivesLoading(false);
    }
  }, []);

  const updateDrive = useCallback(async (uuid: string, customName: string | null, physicalLocation: string | null) => {
    try {
      await invoke('update_drive_metadata', {
        driveUuid: uuid,
        customName,
        physicalLocation
      });

      // Update local state
      setDrives(prev => prev.map(drive =>
        drive.uuid === uuid
          ? { ...drive, custom_name: customName, physical_location: physicalLocation }
          : drive
      ));
    } catch (error) {
      console.error('Failed to update drive metadata:', error);
      throw error;
    }
  }, []);

  const deleteDrive = useCallback(async (uuid: string) => {
    try {
      await invoke('delete_drive_from_database', { driveUuid: uuid });

      // Remove from local state
      setDrives(prev => prev.filter(drive => drive.uuid !== uuid));
    } catch (error) {
      console.error('Failed to delete drive:', error);
      throw error;
    }
  }, []);

  // Check for first-time user and model status on app startup
  useEffect(() => {
    const startup = async () => {
      checkFirstTimeUser();
      initializeModels();
      await recoverInterruptedJobs();
      await loadJobs();
      await loadDrives();
    };

    startup();
  }, [loadDrives]);

  // Handles theming
  useEffect(() => {
    const savedTheme = localStorage.getItem('theme');
    if (savedTheme === 'dark') {
      document.documentElement.classList.add('dark');
    } else if (savedTheme === 'light') {
      document.documentElement.classList.remove('dark');
    } else {
      // fallback to light
      document.documentElement.classList.toggle('dark', false);
      localStorage.setItem('theme', 'light')
    }
  }, [])

  // Handle app closing during downloads
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (modelDownload.state === "downloading") {
        e.preventDefault();
        e.returnValue = "Models are still downloading. Are you sure you want to close?";
        return "Models are still downloading. Are you sure you want to close?";
      }
    };

    window.addEventListener("beforeunload", handleBeforeUnload);

    return () => {
      window.removeEventListener("beforeunload", handleBeforeUnload);
    };
  }, [modelDownload.state]);

  // Listen for download progress events
  useEffect(() => {
    let unlisten: any = null;

    const setupDownloadListener = async () => {
      try {
        unlisten = await listen("download_progress", (event: any) => {
          const progress = event.payload;

          // Clear any pending update to debounce rapid updates
          if (progressUpdateTimeout.current) {
            clearTimeout(progressUpdateTimeout.current);
          }

          // Track individual file progress
          if (progress.status === "Downloading") {
            fileProgressRef.current[progress.file_name] = progress.percentage || 0;
          } else if (progress.status === "Completed") {
            fileProgressRef.current[progress.file_name] = 100;
          }

          // Debounce progress updates to prevent flickering (50ms delay)
          progressUpdateTimeout.current = setTimeout(() => {
            setModelDownload((prev) => {
              // Calculate smooth overall progress based on all files
              const fileNames = Object.keys(fileProgressRef.current);
              const actualTotalFiles = Math.max(prev.totalFiles, fileNames.length);

              if (actualTotalFiles === 0) {
                return prev;
              }

              // Calculate total progress across all files
              let totalProgress = 0;
              for (const fileName of fileNames) {
                totalProgress += fileProgressRef.current[fileName] || 0;
              }

              // Average progress across all files
              const averageProgress = totalProgress / actualTotalFiles;

              // Smooth progress transitions - only update if there's a meaningful change
              const progressDiff = Math.abs(averageProgress - prev.progress);
              const finalProgress = progressDiff > 0.5 ? averageProgress : prev.progress;

              const newFilesCompleted = progress.status === "Completed" ? prev.filesCompleted + 1 : prev.filesCompleted;

              return {
                ...prev,
                state: progress.status === "Completed" ? "installing" : "downloading",
                progress: Math.round(finalProgress * 10) / 10, // Round to 1 decimal place for smoothness
                filesCompleted: newFilesCompleted,
                totalFiles: actualTotalFiles,
              };
            });
          }, 50); // 50ms debounce

          // Improved completion detection
          if (progress.status === "Completed") {
            if (progressUpdateTimeout.current) {
              clearTimeout(progressUpdateTimeout.current);
            }

            // Check if this was the last file to complete
            setModelDownload((prev) => {
              const fileNames = Object.keys(fileProgressRef.current);
              const completedFiles = fileNames.filter(
                (name) => fileProgressRef.current[name] === 100
              ).length;
              const totalFiles = fileNames.length;

              const allFilesCompleted = completedFiles >= totalFiles && totalFiles > 0;

              if (allFilesCompleted) {
                // All files completed - finalize after a short delay
                setTimeout(() => {
                  finalizeModelSetup();
                }, 1000);
              }

              return prev; // Return unchanged state since we already updated it above
            });
          }
        });
      } catch (error) {
        console.error("Failed to setup download progress listener:", error);
      }
    };

    setupDownloadListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
      if (progressUpdateTimeout.current) {
        clearTimeout(progressUpdateTimeout.current);
      }
    };
  }, []);

  // Listen for drive events
  useEffect(() => {
    const setupDriveEventListeners = async () => {
      const unlistenConnected = await listen<DriveInfo>('drive_connected', (event) => {
        console.log('Drive connected:', event.payload);
        setDrives(prev => {
          const existing = prev.find(d => d.uuid === event.payload.uuid);
          if (existing) {
            return prev.map(d => d.uuid === event.payload.uuid ? event.payload : d);
          } else {
            return [...prev, event.payload];
          }
        });
      });

      const unlistenDisconnected = await listen<{ uuid: string, name: string }>('drive_disconnected', (event) => {
        console.log('Drive disconnected:', event.payload);
        setDrives(prev => prev.map(drive =>
          drive.uuid === event.payload.uuid
            ? { ...drive, status: 'disconnected' as const }
            : drive
        ));
      });

      return () => {
        unlistenConnected();
        unlistenDisconnected();
      };
    };

    const cleanup = setupDriveEventListeners();

    return () => {
      cleanup.then(fn => fn());
    };
  }, []);

  // Listen for transcription events
  useEffect(() => {
    const setupTranscriptionEventListeners = async () => {
      const unlistenStarted = await listen<{ file_path: string, status: string }>('transcription_started', (event) => {
        setTranscribingPaths(prev => {
          const newSet = new Set([...prev, event.payload.file_path]);
          return newSet;
        });
      });

      const unlistenCompleted = await listen<{ file_path: string, status: string, transcription: any }>('transcription_completed', (event) => {
        setTranscribingPaths(prev => {
          const newSet = new Set(prev);
          newSet.delete(event.payload.file_path);
          return newSet;
        });
        toast.success("Audio transcription completed!");
      });

      const unlistenFailed = await listen<{ file_path: string, status: string, error: string }>('transcription_failed', (event) => {
        setTranscribingPaths(prev => {
          const newSet = new Set(prev);
          newSet.delete(event.payload.file_path);
          return newSet;
        });
        toast.error(`Failed to transcribe audio: ${event.payload.error}`);
      });

      return () => {
        unlistenStarted();
        unlistenCompleted();
        unlistenFailed();
      };
    };

    const cleanup = setupTranscriptionEventListeners();

    return () => {
      cleanup.then(fn => fn());
    };
  }, [refreshCurrentSearch]);

  // Check if this is a first-time user
  const checkFirstTimeUser = async () => {
    try {
      const hasOnboarded = localStorage.getItem("desktopDocsOnboardingCompleted");
      const hasEverIndexedFiles = localStorage.getItem("desktopDocsHasIndexedFiles");

      if (!hasOnboarded && !hasEverIndexedFiles) {
        setShowOnboarding(true);
      } else if (!hasOnboarded && hasEverIndexedFiles) {
        localStorage.setItem("desktopDocsOnboardingCompleted", "true");
        setHasCompletedOnboarding(true);
      } else {
        setHasCompletedOnboarding(true);
      }
    } catch (error) {
      console.error("Failed to check onboarding status:", error);
      setShowOnboarding(true);
    }
  };

  // Initialize models - single entry point
  const initializeModels = async () => {
    if (downloadInProgressRef.current || modelReloadInProgressRef.current) return;

    try {
      setModelDownload((prev) => ({ ...prev, state: "checking" }));

      const status = await invoke<{
        models_available: boolean;
        missing_models: string[];
        total_missing: number;
      }>("check_models_status");

      if (status.models_available) {
        // Try to load models
        try {
          modelReloadInProgressRef.current = true;
          await invoke("reload_models");
          setModelDownload({
            state: "ready",
            progress: 100,
            filesCompleted: 0,
            totalFiles: 0,
          });
        } catch (error) {
          console.error("Failed to load models:", error);
          setModelDownload({
            state: "failed",
            progress: 0,
            error: "Failed to load models",
            filesCompleted: 0,
            totalFiles: 0,
          });
        } finally {
          modelReloadInProgressRef.current = false;
        }
      } else {
        // Start download
        await startModelDownload(status.total_missing);
      }
    } catch (error) {
      console.error("Failed to check model status:", error);
      setModelDownload({
        state: "failed",
        progress: 0,
        error: "Failed to check model status",
        filesCompleted: 0,
        totalFiles: 0,
      });
    }
  };

  // Start model download - transactional
  const startModelDownload = async (totalFiles: number) => {
    if (downloadInProgressRef.current) return;

    downloadInProgressRef.current = true;

    // Reset file progress tracking for new download
    fileProgressRef.current = {};

    try {
      setModelDownload({
        state: "downloading",
        progress: 0,
        filesCompleted: 0,
        totalFiles,
      });

      await invoke("download_models");
      // Success handled by event listener
    } catch (error) {
      console.error("Model download failed:", error);
      setModelDownload({
        state: "failed",
        progress: 0,
        error: error as string,
        filesCompleted: 0,
        totalFiles,
      });
      downloadInProgressRef.current = false;
    }
  };

  // Finalize model setup after download
  const finalizeModelSetup = async () => {
    // Prevent multiple simultaneous finalization attempts
    if (modelReloadInProgressRef.current) {
      console.log("🔄 Model reload already in progress, skipping finalization");
      return;
    }

    try {
      modelReloadInProgressRef.current = true;
      setModelDownload((prev) => ({
        ...prev,
        state: "installing",
        progress: 95,
      }));

      // Reload models
      await invoke("reload_models");

      setModelDownload({
        state: "ready",
        progress: 100,
        filesCompleted: 0,
        totalFiles: 0,
      });
      downloadInProgressRef.current = false;
    } catch (error) {
      console.error("Failed to finalize model setup:", error);
      setModelDownload((prev) => ({
        ...prev,
        state: "failed",
        error: "Failed to load models after download",
      }));
      downloadInProgressRef.current = false;
    } finally {
      modelReloadInProgressRef.current = false;
    }
  };

  // Retry model download
  const retryModelDownload = () => {
    downloadInProgressRef.current = false;
    initializeModels();
  };

  // Load indexed count on mount for collection display
  useEffect(() => {
    loadIndexedCount();
  }, []);

  // Handler functions
  const handleFileUpload = (file: File) => {
    // Implement visual search
  };

  const handleAddToFavorites = async (file: FileItem) => {
    // In a real app, you'd store this in a database or config file
  };

  const handleShare = async (file: FileItem) => {
    // Implement sharing logic
  };

  const handleDelete = async (file: FileItem) => {
    try {
      await invoke("delete_file", { path: file.path });
      // Refresh the file list
    } catch (error) {
      console.error("Failed to delete file:", error);
    }
  };

  const handleCreateFolder = async (parent: FileItem) => {
    try {
      const newFolderPath = `${parent.path}/New Folder`;
      await invoke("create_directory", { path: newFolderPath });
      // Refresh the file list
    } catch (error) {
      console.error("Failed to create folder:", error);
    }
  };

  // Handle collection selection
  const handleCollectionSelect = (collection: any) => {
    setSelectedCollection(collection.id);
    clearSearch();
    loadIndexedCount();
  };

  // Handle onboarding completion
  const handleOnboardingComplete = () => {
    setShowOnboarding(false);
    setHasCompletedOnboarding(true);
    localStorage.setItem("desktopDocsOnboardingCompleted", "true");

    // Store selected paths for future reference, but don't auto-index
    // Users can manually choose what to index later
    const selectedPaths = localStorage.getItem("desktopDocsSelectedPaths");
    if (selectedPaths) {
      try {
        const paths = JSON.parse(selectedPaths) as string[];
        console.log("📁 Permission granted for folders:", paths);
        // Note: We intentionally don't auto-index here to give users control
      } catch (error) {
        console.error("Failed to parse selected paths:", error);
      }
    }

    // Start interactive tour when models are ready
    setTimeout(() => {
      if (modelDownload.state === "ready") {
        setShowInteractiveTour(true);
      }
    }, 1000);
  };

  // Handle onboarding restart (for testing)
  const handleRestartOnboarding = () => {
    localStorage.removeItem("desktopDocsOnboardingCompleted");
    localStorage.removeItem("desktopDocsHasIndexedFiles");
    setHasCompletedOnboarding(false);
    setShowOnboarding(true);
    setShowInteractiveTour(false);
  };

  // Handle onboarding dismissal
  const handleOnboardingDismiss = () => {
    setShowOnboarding(false);
    setHasCompletedOnboarding(true);
    localStorage.setItem("desktopDocsOnboardingCompleted", "true");

    setTimeout(() => {
      if (modelDownload.state === "ready") {
        setShowInteractiveTour(true);
      }
    }, 1000);
  };

  // Handle interactive tour completion
  const handleTourComplete = () => {
    setShowInteractiveTour(false);
    localStorage.setItem("desktopDocsTourCompleted", "true");
  };

  // Handle interactive tour dismissal
  const handleTourDismiss = () => {
    setShowInteractiveTour(false);
    localStorage.setItem("desktopDocsTourCompleted", "true");
  };

  // Handle starting the tour manually
  const handleStartTour = () => {
    setShowInteractiveTour(true);
  };

  // Handle opening settings
  const handleOpenSettings = () => {
    setShowSettings(true);
  };

  // Handle closing settings
  const handleCloseSettings = () => {
    setShowSettings(false);
  };

  // Handle restarting tour from settings
  const handleRestartTourFromSettings = () => {
    setShowSettings(false);
    setShowInteractiveTour(true);
  };

  // Handle opening bug report
  const handleOpenBugReport = () => {
    setShowBugReport(true);
  };

  // Handle closing bug report
  const handleCloseBugReport = () => {
    setShowBugReport(false);
  };

  // Handle opening benchmark dashboard
  const handleOpenBenchmark = () => {
    setShowBenchmark(true);
  };

  const handleOpenAppStore = () => {
    setShowAppStore(true);
  };

  const handleCloseAppStore = () => {
    setShowAppStore(false);
  };

  // Helper function to check if indexing is allowed
  const isIndexingAllowed = () => {
    return modelDownload.state === "ready";
  };

  // Show warning when trying to index without models
  const showIndexingWarning = () => {
    if (modelDownload.state !== "ready") {
      alert("AI models are not ready yet. Please wait for the setup to complete before indexing files.");
    }
  };

  // Handle bulk indexing with model check
  const handleBulkIndex = async (item: FileItem) => {
    if (!item.is_dir) return;

    if (!isIndexingAllowed()) {
      showIndexingWarning();
      return;
    }

    console.log("🗂️ handleBulkIndex called for directory:", item.path);

    try {
      console.log("🚀 Starting bulk index for:", item.path);

      toast.success("Added directory to search index queue");
      await invoke("index_directory", { path: item.path });
      console.log("✅ Bulk index command completed for:", item.path);
    } catch (error) {
      toast.error("Failed to add directory to search index queue");
      console.error("❌ Failed to start bulk indexing:", error);
    }
  };

  const handleReferenceImageClose = () => {
    if (referenceImage?.url) {
      URL.revokeObjectURL(referenceImage.url);
    }
    setReferenceImage(null);
    setShowReferenceImage(false);
  };

  const handleAddToIndex = async (path: string) => {
    if (!isIndexingAllowed()) {
      showIndexingWarning();
      return;
    }

    try {
      const cleanPath = path.replace("file://", "");
      const absolutePath = await resolve(cleanPath);
      const isDirectory = await invoke<boolean>("is_directory", {
        path: absolutePath,
      });
      window.dispatchEvent(
        new CustomEvent("indexing-started", {
          detail: {
            path: absolutePath,
            type: isDirectory ? "directory" : "file",
          },
        })
      );
      if (isDirectory) {
        await invoke("index_directory", { path: absolutePath });
        toast.success("Added directory to search index queue");
      } else {
        await invoke("index_file", {
          path: absolutePath,
          name: absolutePath.split("/").pop() || "",
          isDirectory: false,
        });
        toast.success("Added file to search index queue");
      }
      await loadIndexedCount();
    } catch (error) {
      console.error("Failed to index file/directory:", error);
      toast.error(`Failed to index file/directory: ${getErrorMessage(error)}`);
    }
  };

  const handleTranscribeFile = async (path: string) => {
    try {
      const cleanPath = path.replace("file://", "");
      const absolutePath = await resolve(cleanPath);

      toast.success("Starting audio transcription...");

      await invoke("transcribe_audio_file", {
        filePath: absolutePath
      });
    } catch (error) {
      console.error("Failed to transcribe audio file:", error);
      toast.error(`Failed to transcribe audio: ${error}`);
    }
  };


  const value: AppLayoutContextType = {
    // File and collection state
    selectedCollection,
    setSelectedCollection,
    indexingPaths,
    setIndexingPaths,
    transcribingPaths,
    setTranscribingPaths,
    handleTranscribeFile,
    isWatchedDialogOpen,
    setIsWatchedDialogOpen,

    // Drive management state
    drives,
    isDrivesLoading,
    loadDrives,
    updateDrive,
    deleteDrive,

    // Model download state
    modelDownload,
    retryModelDownload,

    // UI state for modals and overlays
    showOnboarding,
    setShowOnboarding,
    hasCompletedOnboarding,
    setHasCompletedOnboarding,
    showInteractiveTour,
    setShowInteractiveTour,
    showSettings,
    setShowSettings,
    showBugReport,
    setShowBugReport,
    showBenchmark,
    setShowBenchmark,
    showIndexingInfo,
    setShowIndexingInfo,
    showAppStore,
    setShowAppStore,

    // Search state
    searchState,
    handleSearch,
    clearSearch,
    refreshCurrentSearch,

    // Reference image state
    referenceImage,
    setReferenceImage,
    showReferenceImage,
    setShowReferenceImage,

    // Indexing jobs state
    indexedCount,
    hasActiveJobs,
    hasFailedJobs,
    loadIndexedCount,
    recoverInterruptedJobs,
    loadJobs,

    // Handler functions
    handleFileUpload,
    handleAddToFavorites,
    handleShare,
    handleDelete,
    handleCreateFolder,
    handleCollectionSelect,
    handleOnboardingComplete,
    handleRestartOnboarding,
    handleOnboardingDismiss,
    handleTourComplete,
    handleTourDismiss,
    handleStartTour,
    handleOpenSettings,
    handleCloseSettings,
    handleRestartTourFromSettings,
    handleOpenBugReport,
    handleCloseBugReport,
    handleOpenBenchmark,
    handleOpenAppStore,
    handleCloseAppStore,
    isIndexingAllowed,
    showIndexingWarning,
    handleBulkIndex,
    handleReferenceImageClose,
    handleAddToIndex,
  };

  return (
    <AppLayoutContext.Provider value={value}>
      {children}
    </AppLayoutContext.Provider>
  );
};
