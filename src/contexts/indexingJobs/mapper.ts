import { BackendJob, Job, JobStatus } from "./types";

export function mapBackendJobToJob(backendJob: BackendJob): Job {
  return {
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
    status: backendJob.status as JobStatus,
    startTime: backendJob.started_at
      ? new Date(backendJob.started_at)
      : new Date(backendJob.created_at || Date.now()),
  };
}

