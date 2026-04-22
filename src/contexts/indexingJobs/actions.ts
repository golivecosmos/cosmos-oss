import { invoke } from "@tauri-apps/api/core";
import { mapBackendJobToJob } from "./mapper";
import { BackendJob, Job } from "./types";

export async function loadPersistentJobs(limit = 2000): Promise<Job[]> {
  const persistentJobs = await invoke<BackendJob[]>("get_jobs", { limit });
  return persistentJobs.map(mapBackendJobToJob);
}

export async function recoverInterruptedJobs(): Promise<boolean> {
  const result = await invoke<{ recovered_count?: number }>("recover_interrupted_jobs");
  const recoveredCount = result?.recovered_count ?? 0;

  if (recoveredCount === 0) {
    return false;
  }

  console.log(`Recovered ${recoveredCount} interrupted jobs back to pending`);

  return true;
}

export async function loadIndexedFileCount(): Promise<number> {
  return invoke<number>("get_indexed_count");
}
