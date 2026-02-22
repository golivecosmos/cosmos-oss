export type JobStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export interface JobProgress {
  current_file: string;
  processed: number;
  total: number;
  status: string;
  errors: string[];
  directory_path: string;
}

export interface Job {
  id: string;
  path: string;
  progress: JobProgress;
  status: JobStatus;
  startTime: Date;
}

export interface BackendJob {
  id: string;
  target_path: string;
  status: string;
  current_file?: string;
  processed?: number;
  total?: number;
  errors?: string[];
  started_at?: string;
  created_at?: string;
}

