import { Dispatch, SetStateAction, useEffect } from "react";
import { listen, type Event } from "@tauri-apps/api/event";
import { mapBackendJobToJob } from "./mapper";
import { BackendJob, Job } from "./types";

interface JobsBatchCreatedPayload {
  total_jobs: number;
  directory_path: string;
}

interface JobsClearedPayload {
  deleted_count: number;
}

interface UseJobEventListenersArgs {
  setIndexingJobs: Dispatch<SetStateAction<Job[]>>;
  loadJobs: () => Promise<void>;
  loadIndexedCount: () => Promise<void>;
  setIndexedCount: Dispatch<SetStateAction<number>>;
}

export function useJobEventListeners({
  setIndexingJobs,
  loadJobs,
  loadIndexedCount,
  setIndexedCount,
}: UseJobEventListenersArgs) {
  useEffect(() => {
    let isMounted = true;
    const unlistenFns: Array<() => void> = [];

    const setupListeners = async () => {
      try {
        console.log("🔔 Setting up job event listeners...");

        const unlistenCreated = await listen<BackendJob>("job_created", (event) => {
          if (!isMounted) return;
          const payload = (event as Event<BackendJob>).payload;
          console.log("🔔 Job created event:", payload.id);
          const newJob = mapBackendJobToJob(payload);
          setIndexingJobs((prev) => [newJob, ...prev.filter((j) => j.id !== newJob.id)]);
        });
        unlistenFns.push(unlistenCreated);

        const unlistenBatchCreated = await listen<JobsBatchCreatedPayload>(
          "jobs_batch_created",
          async (event) => {
            if (!isMounted) return;
            const payload = (event as Event<JobsBatchCreatedPayload>).payload;
            console.log(
              "🔔 Batch jobs created event:",
              payload.total_jobs,
              "jobs for",
              payload.directory_path
            );
            await loadJobs();
            console.log("✅ Refreshed job list after batch creation");
          }
        );
        unlistenFns.push(unlistenBatchCreated);

        const unlistenUpdated = await listen<BackendJob>("job_updated", (event) => {
          if (!isMounted) return;
          const updatedJob = mapBackendJobToJob((event as Event<BackendJob>).payload);
          setIndexingJobs((prev) =>
            prev.map((job) => (job.id === updatedJob.id ? updatedJob : job))
          );
        });
        unlistenFns.push(unlistenUpdated);

        const unlistenCompleted = await listen<BackendJob>("job_completed", (event) => {
          if (!isMounted) return;
          const completedJob = mapBackendJobToJob((event as Event<BackendJob>).payload);
          setIndexingJobs((prev) =>
            prev.map((job) => (job.id === completedJob.id ? completedJob : job))
          );

          void loadIndexedCount();
          localStorage.setItem("desktopDocsHasIndexedFiles", "true");
        });
        unlistenFns.push(unlistenCompleted);

        const unlistenJobsCleared = await listen<JobsClearedPayload>("jobs_cleared", (event) => {
          if (!isMounted) return;
          const payload = (event as Event<JobsClearedPayload>).payload;
          console.log("🔔 Jobs cleared event:", payload.deleted_count, "jobs cleared");
          setIndexingJobs([]);
        });
        unlistenFns.push(unlistenJobsCleared);

        const unlistenIndexCleared = await listen("index_cleared", () => {
          if (!isMounted) return;
          console.log("📱 Index cleared, updating count...");
          setIndexedCount(0);
        });
        unlistenFns.push(unlistenIndexCleared);

        console.log("✅ Job event listeners set up successfully");
      } catch (error) {
        console.error("❌ Failed to setup job event listeners:", error);
      }
    };

    void setupListeners();

    return () => {
      isMounted = false;
      for (const unlisten of unlistenFns) {
        unlisten();
      }
      console.log("🧹 Job event listeners cleaned up");
    };
  }, [loadIndexedCount, loadJobs, setIndexedCount, setIndexingJobs]);
}

