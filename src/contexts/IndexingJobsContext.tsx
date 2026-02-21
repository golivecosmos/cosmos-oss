import {
  createContext,
  useContext,
  useState,
  useEffect,
  ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

export interface Job {
  id: string;
  path: string;
  progress: any;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  startTime: Date;
}

interface IndexingJobsContextType {
  indexingJobs: Job[];
  indexedCount: number;
  hasActiveJobs: boolean;
  hasFailedJobs: boolean;
  loadJobs: () => Promise<void>;
  loadIndexedCount: () => Promise<void>;
  recoverInterruptedJobs: () => Promise<void>;
}

const IndexingJobsContext = createContext<IndexingJobsContextType | undefined>(
  undefined
);

export function useIndexingJobs() {
  const context = useContext(IndexingJobsContext);
  if (context === undefined) {
    throw new Error(
      "useIndexingJobs must be used within an IndexingJobsProvider"
    );
  }
  return context;
}

interface IndexingJobsProviderProps {
  children: ReactNode;
}

export function IndexingJobsProvider({ children }: IndexingJobsProviderProps) {
  const [indexingJobs, setIndexingJobs] = useState<Job[]>([]);
  const [indexedCount, setIndexedCount] = useState<number>(0);

  const loadJobs = async () => {
    try {
      const persistentJobs = await invoke<any[]>("get_jobs", { limit: 2000 });

      const convertedJobs: Job[] = persistentJobs.map((job) => ({
        id: job.id,
        path: job.target_path,
        progress: {
          current_file: job.current_file || "",
          processed: job.processed || 0,
          total: job.total || 0,
          status: job.status,
          errors: job.errors || [],
          directory_path: job.target_path,
        },
        status: job.status as
          | "pending"
          | "running"
          | "completed"
          | "failed"
          | "cancelled",
        startTime: job.started_at
          ? new Date(job.started_at)
          : new Date(job.created_at),
      }));

      setIndexingJobs(convertedJobs);
    } catch (error) {
      console.error("Failed to load persistent jobs:", error);
      setIndexingJobs([]);
    }
  };

  const recoverInterruptedJobs = async () => {
    try {
      const runningJobs = await invoke<any[]>("get_jobs", {
        status: "running",
      });

      if (runningJobs.length > 0) {
        console.log(
          `🔄 Found ${runningJobs.length} interrupted jobs, marking as failed...`
        );

        for (const job of runningJobs) {
          try {
            await invoke("manage_job_queue", {
              action: "cancel",
              job_id: job.id,
            });
            console.log(`❌ Marked interrupted job as cancelled: ${job.id}`);
          } catch (error) {
            console.error("Failed to cancel interrupted job:", job.id, error);
          }
        }

        await loadJobs();
      }
    } catch (error) {
      console.error("❌ Failed to recover interrupted jobs:", error);
    }
  };

  const loadIndexedCount = async () => {
    try {
      const count = await invoke<number>("get_indexed_count");
      setIndexedCount(count);
    } catch (error) {
      console.error("Failed to load indexed file count:", error);
      setIndexedCount(0);
    }
  };

  useEffect(() => {
    const startup = async () => {
      await recoverInterruptedJobs();
      await loadJobs();
    };

    startup();
  }, []);

  useEffect(() => {
    let unlistenCreated: any = null;
    let unlistenUpdated: any = null;
    let unlistenCompleted: any = null;
    let unlistenBatchCreated: any = null;
    let unlistenJobsCleared: any = null;
    let unlistenIndexCleared: any = null;
    let isListenersSetup = false;

    const setupJobEventListeners = async () => {
      if (isListenersSetup) return;

      try {
        console.log("🔔 Setting up job event listeners...");

        const convertJob = (backendJob: any): Job => ({
          id: backendJob.id,
          path: backendJob.target_path,
          progress: {
            current_file: backendJob.current_file || "",
            processed: backendJob.processed || 0,
            total: backendJob.total || 0,
            status: backendJob.status,
            errors: backendJob.errors || [],
            directory_path: backendJob.target_path,
          },
          status: backendJob.status as
            | "pending"
            | "running"
            | "completed"
            | "failed"
            | "cancelled",
          startTime: backendJob.started_at
            ? new Date(backendJob.started_at)
            : new Date(backendJob.created_at),
        });

        unlistenCreated = await listen("job_created", (event: any) => {
          console.log("🔔 Job created event:", event.payload.id);
          const newJob = convertJob(event.payload);
          setIndexingJobs((prev) => [
            newJob,
            ...prev.filter((j) => j.id !== newJob.id),
          ]);
        });

        unlistenBatchCreated = await listen(
          "jobs_batch_created",
          async (event: any) => {
            console.log(
              "🔔 Batch jobs created event:",
              event.payload.total_jobs,
              "jobs for",
              event.payload.directory_path
            );

            await loadJobs();
            console.log("✅ Refreshed job list after batch creation");
          }
        );

        unlistenUpdated = await listen("job_updated", (event: any) => {
          const updatedJob = convertJob(event.payload);
          setIndexingJobs((prev) =>
            prev.map((job) => (job.id === updatedJob.id ? updatedJob : job))
          );
        });

        unlistenCompleted = await listen("job_completed", (event: any) => {
          const completedJob = convertJob(event.payload);
          setIndexingJobs((prev) =>
            prev.map((job) => (job.id === completedJob.id ? completedJob : job))
          );

          loadIndexedCount();

          localStorage.setItem("desktopDocsHasIndexedFiles", "true");
        });

        unlistenJobsCleared = await listen("jobs_cleared", (event: any) => {
          console.log(
            "🔔 Jobs cleared event:",
            event.payload.deleted_count,
            "jobs cleared"
          );
          setIndexingJobs([]);
        });

        isListenersSetup = true;
        console.log("✅ Job event listeners set up successfully");
      } catch (error) {
        console.error("❌ Failed to setup job event listeners:", error);
      }
    };

    const setupIndexClearedEventListener = async () => {
      try {
        unlistenIndexCleared = await listen("index_cleared", () => {
          console.log("📱 Index cleared, updating count...");
          setIndexedCount(0);
        });
      } catch (error) {
        console.error("Failed to setup event listener:", error);
      }
    };

    setupJobEventListeners();
    setupIndexClearedEventListener();

    return () => {
      if (unlistenCreated) {
        unlistenCreated();
        unlistenCreated = null;
      }
      if (unlistenUpdated) {
        unlistenUpdated();
        unlistenUpdated = null;
      }
      if (unlistenCompleted) {
        unlistenCompleted();
        unlistenCompleted = null;
      }
      if (unlistenBatchCreated) {
        unlistenBatchCreated();
        unlistenBatchCreated = null;
      }
      if (unlistenJobsCleared) {
        unlistenJobsCleared();
        unlistenJobsCleared = null;
      }
      if (unlistenIndexCleared) {
        unlistenIndexCleared();
        unlistenIndexCleared = null;
      }
      isListenersSetup = false;
      console.log("🧹 Job event listeners cleaned up");
    };
  }, []);

  useEffect(() => {
    let syncInterval: NodeJS.Timeout | null = null;

    const startPeriodicSync = () => {
      syncInterval = setInterval(async () => {
        try {
          const hasActiveJobs = indexingJobs.some(
            (job) => job.status === "pending" || job.status === "running"
          );

          if (hasActiveJobs) {
            await loadJobs();
          }
        } catch (error) {
          console.error("Periodic sync failed:", error);
        }
      }, 10000);
    };

    if (indexingJobs.length > 0) {
      startPeriodicSync();
    }

    return () => {
      if (syncInterval) {
        clearInterval(syncInterval);
        syncInterval = null;
      }
    };
  }, [indexingJobs.length]);

  const hasActiveJobs = indexingJobs.some((job) => job.status === "running");
  const hasFailedJobs = indexingJobs.some((job) => job.status === "failed");

  return (
    <IndexingJobsContext.Provider
      value={{
        indexingJobs,
        indexedCount,
        hasActiveJobs,
        hasFailedJobs,
        loadJobs,
        loadIndexedCount,
        recoverInterruptedJobs,
      }}
    >
      {children}
    </IndexingJobsContext.Provider>
  );
}
