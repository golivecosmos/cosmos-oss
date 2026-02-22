import { invoke } from "@tauri-apps/api/core";
import { mapBackendJobToJob } from "./mapper";
import { BackendJob, Job } from "./types";

export async function loadPersistentJobs(limit = 2000): Promise<Job[]> {
  const persistentJobs = await invoke<BackendJob[]>("get_jobs", { limit });
  return persistentJobs.map(mapBackendJobToJob);
}

export async function recoverInterruptedJobs(): Promise<boolean> {
  const runningJobs = await invoke<BackendJob[]>("get_jobs", { status: "running" });
  if (runningJobs.length === 0) {
    return false;
  }

  console.log(
    `\ud83d\udd04 Found ${runningJobs.length} interrupted jobs, marking as failed...`
  );

  for (const job of runningJobs) {
    try {
      await invoke("manage_job_queue", {
        action: "cancel",
        job_id: job.id,
      });
      console.log(`\u274c Marked interrupted job as cancelled: ${job.id}`);
    } catch (error) {
      console.error("Failed to cancel interrupted job:", job.id, error);
    }
  }

  return true;
}

export async function loadIndexedFileCount(): Promise<number> {
  return invoke<number>("get_indexed_count");
}

