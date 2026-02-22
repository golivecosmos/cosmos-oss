import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  ReactNode,
} from "react";
import {
  loadIndexedFileCount,
  loadPersistentJobs,
  recoverInterruptedJobs as recoverInterruptedJobsAction,
} from "./indexingJobs/actions";
import { useJobEventListeners } from "./indexingJobs/useJobEventListeners";
import { usePeriodicJobSync } from "./indexingJobs/usePeriodicJobSync";
import { type Job } from "./indexingJobs/types";

export type { Job } from "./indexingJobs/types";

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

  const loadJobs = useCallback(async () => {
    try {
      const jobs = await loadPersistentJobs(2000);
      setIndexingJobs(jobs);
    } catch (error) {
      console.error("Failed to load persistent jobs:", error);
      setIndexingJobs([]);
    }
  }, []);

  const recoverInterruptedJobs = useCallback(async () => {
    try {
      const recoveredAny = await recoverInterruptedJobsAction();
      if (recoveredAny) {
        await loadJobs();
      }
    } catch (error) {
      console.error("❌ Failed to recover interrupted jobs:", error);
    }
  }, [loadJobs]);

  const loadIndexedCount = useCallback(async () => {
    try {
      const count = await loadIndexedFileCount();
      setIndexedCount(count);
    } catch (error) {
      console.error("Failed to load indexed file count:", error);
      setIndexedCount(0);
    }
  }, []);

  useEffect(() => {
    const startup = async () => {
      await recoverInterruptedJobs();
      await loadJobs();
    };

    startup();
  }, [loadJobs, recoverInterruptedJobs]);

  useJobEventListeners({
    setIndexingJobs,
    loadJobs,
    loadIndexedCount,
    setIndexedCount,
  });

  usePeriodicJobSync({ indexingJobs, loadJobs });

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
