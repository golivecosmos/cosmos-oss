import { useEffect } from "react";
import { Job } from "./types";

interface UsePeriodicJobSyncArgs {
  indexingJobs: Job[];
  loadJobs: () => Promise<void>;
}

export function usePeriodicJobSync({ indexingJobs, loadJobs }: UsePeriodicJobSyncArgs) {
  useEffect(() => {
    if (indexingJobs.length === 0) {
      return;
    }

    const syncInterval = setInterval(() => {
      const hasActiveJobs = indexingJobs.some(
        (job) => job.status === "pending" || job.status === "running"
      );
      if (hasActiveJobs) {
        void loadJobs();
      }
    }, 10000);

    return () => {
      clearInterval(syncInterval);
    };
  }, [indexingJobs, loadJobs]);
}

