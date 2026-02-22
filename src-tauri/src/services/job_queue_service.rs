use anyhow::{anyhow, Result};
use rusqlite::{OptionalExtension, ErrorCode};
use std::sync::Arc;
use crate::{app_log_info, app_log_debug, app_log_warn, app_log_error};
use crate::services::database_service::DatabaseService;
use crate::services::schema_service::SchemaService;
use uuid;

/// Service for managing job queue operations
pub struct JobQueueService {
    db_service: Arc<DatabaseService>,
    schema_service: Arc<SchemaService>,
}

impl JobQueueService {
    /// Create a new job queue service
    pub fn new(db_service: Arc<DatabaseService>, schema_service: Arc<SchemaService>) -> Self {
        Self {
            db_service,
            schema_service,
        }
    }

    /// Create a new job in the database
    pub fn create_job(&self, job_type: &str, target_path: &str, total_files: Option<usize>) -> Result<String> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // **DEFENSIVE: Ensure jobs table exists before proceeding**
        if !self.schema_service.jobs_table_exists(&db) {
            app_log_warn!("⚠️ JOBS TABLE: Jobs table missing during create_job, creating it now");
            self.schema_service.ensure_jobs_table_exists(&db)?;
        }

        let job_id = format!("job_{}_{}",
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown")
        );

        let now = chrono::Utc::now().to_rfc3339();
        let total = total_files.unwrap_or(0) as i64;

        match db.execute(
            "INSERT INTO jobs (
                id, job_type, target_path, status, processed, total,
                created_at, updated_at
            ) VALUES (?, ?, ?, 'pending', 0, ?, ?, ?)",
            rusqlite::params![job_id, job_type, target_path, total, now, now],
        ) {
            Ok(_) => {
                app_log_info!("✅ JOB: Created new {} job: {} ({})", job_type, job_id, target_path);
                Ok(job_id)
            }
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == ErrorCode::ConstraintViolation =>
            {
                let existing_id: Option<String> = db.query_row(
                    "SELECT id FROM jobs WHERE job_type = ? AND target_path = ? AND status IN ('pending', 'running') ORDER BY created_at DESC LIMIT 1",
                    rusqlite::params![job_type, target_path],
                    |row| row.get(0),
                ).optional()?;

                if let Some(id) = existing_id {
                    app_log_info!(
                        "⚠️ JOB: Constraint prevented duplicate insert, returning existing job {} for {}",
                        id,
                        target_path
                    );
                    Ok(id)
                } else {
                    Err(anyhow!(
                        "Constraint violation creating job for {}, but no active job found",
                        target_path
                    ))
                }
            }
            Err(e) => Err(anyhow!("Failed to create job for {}: {}", target_path, e)),
        }
    }

    /// Update job status and progress
    pub fn update_job_progress(
        &self,
        job_id: &str,
        status: &str,
        current_file: Option<&str>,
        processed: Option<usize>,
        errors: Option<&[String]>,
        failed_files: Option<&serde_json::Value>
    ) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // **DEFENSIVE: Ensure jobs table exists before updating**
        if !self.schema_service.jobs_table_exists(&db) {
            app_log_warn!("⚠️ JOBS TABLE: Jobs table missing during update_job_progress, creating it now");
            self.schema_service.ensure_jobs_table_exists(&db)?;
            // Job doesn't exist if table was just created, so return early
            app_log_warn!("⚠️ JOB UPDATE: Job {} not found because jobs table was just created", job_id);
            return Ok(());
        }

        let now = chrono::Utc::now().to_rfc3339();
        let errors_json = errors.map(|e| serde_json::to_string(e).unwrap_or_else(|_| "[]".to_string()));
        let failed_files_json = failed_files.map(|f| f.to_string());

        // Set started_at timestamp when job status changes to 'running'
        let started_at_clause = if status == "running" {
            ", started_at = CASE WHEN started_at IS NULL THEN ? ELSE started_at END"
        } else {
            ""
        };

        // Set completed_at timestamp when job is completed/failed/cancelled
        let completed_at_clause = if ["completed", "failed", "cancelled"].contains(&status) {
            ", completed_at = ?"
        } else {
            ""
        };

        let query = format!(
            "UPDATE jobs SET
                status = ?,
                current_file = COALESCE(?, current_file),
                processed = COALESCE(?, processed),
                errors = COALESCE(?, errors),
                failed_files = COALESCE(?, failed_files),
                updated_at = ?
                {}{}
            WHERE id = ?",
            started_at_clause, completed_at_clause
        );

        let processed_i64 = processed.map(|p| p as i64);
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![
            &status,
            &current_file,
            &processed_i64,
            &errors_json,
            &failed_files_json,
            &now
        ];

        if status == "running" {
            params.push(&now); // started_at
        }

        if ["completed", "failed", "cancelled"].contains(&status) {
            params.push(&now); // completed_at
        }

        params.push(&job_id);

        db.execute(&query, rusqlite::params_from_iter(params))?;

        app_log_debug!("✅ JOB: Updated job {} - status: {}, processed: {:?}",
            job_id, status, processed);
        Ok(())
    }

    /// Get all jobs (recent first)
    pub fn get_jobs(&self, limit: Option<usize>) -> Result<Vec<serde_json::Value>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // **DEFENSIVE: Ensure jobs table exists before querying**
        if !self.schema_service.jobs_table_exists(&db) {
            app_log_warn!("⚠️ JOBS TABLE: Jobs table missing during get_jobs, creating it now");
            self.schema_service.ensure_jobs_table_exists(&db)?;
            // Return empty array since we just created the table
            return Ok(Vec::new());
        }

        let limit_clause = limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default();
        let query = format!(
            "SELECT id, job_type, target_path, status, current_file, processed, total,
                    errors, failed_files, metadata, retry_count, max_retries, next_retry_at,
                    created_at, started_at, completed_at, updated_at
             FROM jobs
             ORDER BY created_at DESC {}",
            limit_clause
        );

        let mut stmt = db.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params![], |row| {
            let errors_json: String = row.get(7)?;
            let failed_files_json: String = row.get(8)?;
            let metadata_json: String = row.get(9)?;

            let errors: Vec<String> = serde_json::from_str(&errors_json).unwrap_or_default();
            let failed_files: serde_json::Value = serde_json::from_str(&failed_files_json).unwrap_or(serde_json::json!([]));
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({}));

            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "job_type": row.get::<_, String>(1)?,
                "target_path": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "current_file": row.get::<_, Option<String>>(4)?,
                "processed": row.get::<_, i64>(5)?,
                "total": row.get::<_, i64>(6)?,
                "errors": errors,
                "failed_files": failed_files,
                "metadata": metadata,
                "retry_count": row.get::<_, i64>(10)?,
                "max_retries": row.get::<_, i64>(11)?,
                "next_retry_at": row.get::<_, Option<String>>(12)?,
                "created_at": row.get::<_, String>(13)?,
                "started_at": row.get::<_, Option<String>>(14)?,
                "completed_at": row.get::<_, Option<String>>(15)?,
                "updated_at": row.get::<_, String>(16)?
            }))
        })?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }

        Ok(jobs)
    }

    /// Get jobs by status
    pub fn get_jobs_by_status(&self, status: &str) -> Result<Vec<serde_json::Value>> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // **DEFENSIVE: Ensure jobs table exists before querying**
        if !self.schema_service.jobs_table_exists(&db) {
            app_log_warn!("⚠️ JOBS TABLE: Jobs table missing during get_jobs_by_status, creating it now");
            self.schema_service.ensure_jobs_table_exists(&db)?;
            // Return empty array since we just created the table
            return Ok(Vec::new());
        }

        let mut stmt = db.prepare(
            "SELECT id, job_type, target_path, status, current_file, processed, total,
                    errors, failed_files, metadata, retry_count, max_retries, next_retry_at,
                    created_at, started_at, completed_at, updated_at
             FROM jobs
             WHERE status = ?
             ORDER BY created_at DESC"
        )?;

        let rows = stmt.query_map(rusqlite::params![status], |row| {
            let errors_json: String = row.get(7)?;
            let failed_files_json: String = row.get(8)?;
            let metadata_json: String = row.get(9)?;

            let errors: Vec<String> = serde_json::from_str(&errors_json).unwrap_or_default();
            let failed_files: serde_json::Value = serde_json::from_str(&failed_files_json).unwrap_or(serde_json::json!([]));
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({}));

            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "job_type": row.get::<_, String>(1)?,
                "target_path": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "current_file": row.get::<_, Option<String>>(4)?,
                "processed": row.get::<_, i64>(5)?,
                "total": row.get::<_, i64>(6)?,
                "errors": errors,
                "failed_files": failed_files,
                "metadata": metadata,
                "retry_count": row.get::<_, i64>(10)?,
                "max_retries": row.get::<_, i64>(11)?,
                "next_retry_at": row.get::<_, Option<String>>(12)?,
                "created_at": row.get::<_, String>(13)?,
                "started_at": row.get::<_, Option<String>>(14)?,
                "completed_at": row.get::<_, Option<String>>(15)?,
                "updated_at": row.get::<_, String>(16)?
            }))
        })?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }

        Ok(jobs)
    }

    /// Get a single job by ID
    pub fn get_job_by_id(&self, job_id: &str) -> Result<serde_json::Value> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // **DEFENSIVE: Ensure jobs table exists before querying**
        if !self.schema_service.jobs_table_exists(&db) {
            app_log_warn!("⚠️ JOBS TABLE: Jobs table missing during get_job_by_id, creating it now");
            self.schema_service.ensure_jobs_table_exists(&db)?;
            // Return an error since the job doesn't exist if table was just created
            return Err(anyhow!("Job {} not found (jobs table was just created)", job_id));
        }

        let mut stmt = db.prepare(
            "SELECT id, job_type, target_path, status, current_file, processed, total,
                    errors, failed_files, metadata, retry_count, max_retries, next_retry_at,
                    created_at, started_at, completed_at, updated_at
             FROM jobs
             WHERE id = ?"
        )?;

        let job = stmt.query_row(rusqlite::params![job_id], |row| {
            let errors_json: String = row.get(7)?;
            let failed_files_json: String = row.get(8)?;
            let metadata_json: String = row.get(9)?;

            let errors: Vec<String> = serde_json::from_str(&errors_json).unwrap_or_default();
            let failed_files: serde_json::Value = serde_json::from_str(&failed_files_json).unwrap_or(serde_json::json!([]));
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({}));

            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "job_type": row.get::<_, String>(1)?,
                "target_path": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "current_file": row.get::<_, Option<String>>(4)?,
                "processed": row.get::<_, i64>(5)?,
                "total": row.get::<_, i64>(6)?,
                "errors": errors,
                "failed_files": failed_files,
                "metadata": metadata,
                "retry_count": row.get::<_, i64>(10)?,
                "max_retries": row.get::<_, i64>(11)?,
                "next_retry_at": row.get::<_, Option<String>>(12)?,
                "created_at": row.get::<_, String>(13)?,
                "started_at": row.get::<_, Option<String>>(14)?,
                "completed_at": row.get::<_, Option<String>>(15)?,
                "updated_at": row.get::<_, String>(16)?
            }))
        })?;

        Ok(job)
    }

    /// **NEW: Mark job for automatic retry with exponential backoff**
    pub fn schedule_job_retry(&self, job_id: &str, error_message: &str) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Get current retry count
        let (current_retries, max_retries): (i64, i64) = db.query_row(
            "SELECT retry_count, max_retries FROM jobs WHERE id = ?",
            rusqlite::params![job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let new_retry_count = current_retries + 1;

        if new_retry_count <= max_retries {
            // Calculate exponential backoff: 30s, 2m, 8m
            let delay_seconds = match new_retry_count {
                1 => 30,    // 30 seconds
                2 => 120,   // 2 minutes
                3 => 480,   // 8 minutes
                _ => 480,   // Cap at 8 minutes
            };

            let next_retry_time = chrono::Utc::now() + chrono::Duration::seconds(delay_seconds);
            let now = chrono::Utc::now().to_rfc3339();

            db.execute(
                "UPDATE jobs SET
                    status = 'pending',
                    retry_count = ?,
                    next_retry_at = ?,
                    updated_at = ?,
                    errors = json_insert(COALESCE(errors, '[]'), '$[#]', ?)
                 WHERE id = ?",
                rusqlite::params![
                    new_retry_count,
                    next_retry_time.to_rfc3339(),
                    now,
                    error_message,
                    job_id
                ],
            )?;

            app_log_info!("🔄 RETRY: Scheduled job {} for retry #{} in {}s",
                job_id, new_retry_count, delay_seconds);
        } else {
            // Max retries exceeded - mark as permanently failed
            let now = chrono::Utc::now().to_rfc3339();

            db.execute(
                "UPDATE jobs SET
                    status = 'failed',
                    completed_at = ?,
                    updated_at = ?,
                    errors = json_insert(COALESCE(errors, '[]'), '$[#]', ?)
                 WHERE id = ?",
                rusqlite::params![
                    now,
                    now,
                    format!("Max retries ({}) exceeded: {}", max_retries, error_message),
                    job_id
                ],
            )?;

            app_log_warn!("❌ RETRY: Job {} failed permanently after {} retries",
                job_id, max_retries);
        }

        Ok(())
    }

    /// **NEW: Manual user retry (resets retry count)**
    pub fn manual_retry_job(&self, job_id: &str) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        db.execute(
            "UPDATE jobs SET
                status = 'pending',
                retry_count = 0,
                next_retry_at = NULL,
                updated_at = ?
             WHERE id = ?",
            rusqlite::params![now, job_id],
        )?;

        app_log_info!("🔄 MANUAL RETRY: User requested retry for job {}", job_id);
        Ok(())
    }

    /// **FIXED: Truly atomic job claiming that prevents race conditions**
    pub fn claim_pending_jobs_atomic(&self, worker_id: usize, limit: usize) -> Result<Vec<serde_json::Value>> {
        let connection = self.db_service.get_connection();
        let mut db = connection.lock().unwrap();

        // **DEFENSIVE: Ensure jobs table exists before claiming jobs**
        if !self.schema_service.jobs_table_exists(&db) {
            app_log_warn!("⚠️ JOBS TABLE: Jobs table missing during claim_pending_jobs_atomic, creating it now");
            self.schema_service.ensure_jobs_table_exists(&db)?;
            return Ok(Vec::new());
        }

        // Use a transaction for atomicity
        let tx = db.transaction()?;

        // First get the IDs of pending jobs (excluding those with next_retry_at that haven't reached retry time)
        let now = chrono::Utc::now().to_rfc3339();
        let job_ids: Vec<String> = {
            let mut stmt = tx.prepare(
                "SELECT id FROM jobs
                 WHERE status = 'pending'
                 AND (next_retry_at IS NULL OR next_retry_at <= ?)
                 ORDER BY created_at ASC
                 LIMIT ?"
            )?;
            let ids = stmt.query_map(rusqlite::params![now, limit], |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            drop(stmt); // Explicitly drop the statement
            ids
        };

        // Early return if no jobs
        if job_ids.is_empty() {
            tx.commit()?;
            return Ok(Vec::new());
        }

        app_log_debug!("🔄 WORKER {}: Attempting to claim {} pending jobs", worker_id, job_ids.len());

        // Update job statuses within the same transaction with double-check for race conditions
        let placeholders = job_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let update_query = format!(
            "UPDATE jobs
             SET status = 'running',
                 started_at = COALESCE(started_at, ?),
                 updated_at = ?,
                 current_file = ?
             WHERE id IN ({}) AND status = 'pending'",
            placeholders
        );

        let worker_marker = format!("worker_{}", worker_id);
        let mut update_params: Vec<&dyn rusqlite::ToSql> = vec![&now, &now, &worker_marker];
        update_params.extend(job_ids.iter().map(|id| id as &dyn rusqlite::ToSql));

        let updated = tx.execute(&update_query, rusqlite::params_from_iter(update_params))?;

        // Verify all jobs were updated (race condition protection)
        if updated as usize != job_ids.len() {
            app_log_debug!(
                "⚠️ WORKER {}: Only {} of {} jobs were claimed (others claimed by other workers)",
                worker_id, updated, job_ids.len()
            );

            // Don't roll back, just return the successfully claimed jobs
            // This allows partial success when multiple workers compete for jobs
        }

        // Fetch full job data for successfully claimed jobs within the same transaction
        let mut jobs = Vec::new();
        {
            let mut fetch_stmt = tx.prepare(
                "SELECT id, job_type, target_path, status, current_file, processed, total,
                        errors, failed_files, metadata, retry_count, max_retries, next_retry_at,
                        created_at, started_at, completed_at, updated_at
                 FROM jobs
                 WHERE id = ? AND status = 'running'"
            )?;

            for job_id in &job_ids {
                match fetch_stmt.query_row(rusqlite::params![job_id], |row| {
                    let errors_json: String = row.get(7)?;
                    let failed_files_json: String = row.get(8)?;
                    let metadata_json: String = row.get(9)?;

                    let errors: Vec<String> = serde_json::from_str(&errors_json).unwrap_or_default();
                    let failed_files: serde_json::Value = serde_json::from_str(&failed_files_json).unwrap_or(serde_json::json!([]));
                    let metadata: serde_json::Value = serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({}));

                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "job_type": row.get::<_, String>(1)?,
                        "target_path": row.get::<_, String>(2)?,
                        "status": row.get::<_, String>(3)?,
                        "current_file": row.get::<_, Option<String>>(4)?,
                        "processed": row.get::<_, i64>(5)?,
                        "total": row.get::<_, i64>(6)?,
                        "errors": errors,
                        "failed_files": failed_files,
                        "metadata": metadata,
                        "retry_count": row.get::<_, i64>(10)?,
                        "max_retries": row.get::<_, i64>(11)?,
                        "next_retry_at": row.get::<_, Option<String>>(12)?,
                        "created_at": row.get::<_, String>(13)?,
                        "started_at": row.get::<_, Option<String>>(14)?,
                        "completed_at": row.get::<_, Option<String>>(15)?,
                        "updated_at": row.get::<_, String>(16)?
                    }))
                }) {
                    Ok(job_data) => jobs.push(job_data),
                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                        // Job was not claimed (race condition), skip it
                        app_log_debug!("⚠️ WORKER {}: Job {} was not claimed due to race condition", worker_id, job_id);
                    },
                    Err(e) => {
                        app_log_error!("❌ WORKER {}: Failed to fetch job data for {}: {}", worker_id, job_id, e);
                    }
                }
            }

            drop(fetch_stmt); // Explicitly drop the statement
        }

        // Commit the transaction
        tx.commit()?;

        if !jobs.is_empty() {
            let job_ids: Vec<String> = jobs.iter()
                .filter_map(|job| job["id"].as_str().map(|s| s.to_string()))
                .collect();
            app_log_debug!("✅ WORKER {}: Atomically claimed {} jobs: {:?}", worker_id, jobs.len(), job_ids);
        }

        Ok(jobs)
    }

    /// Cancel a job (mark it as cancelled)
    pub fn cancel_job(&self, job_id: &str) -> Result<()> {
        self.update_job_progress(job_id, "cancelled", None, None, None, None)?;
        app_log_info!("🛑 JOB: Cancelled job: {}", job_id);
        Ok(())
    }

    /// Clean up old completed jobs (older than specified days)
    pub fn cleanup_old_jobs(&self, days_old: i64) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days_old);
        let cutoff_str = cutoff_date.to_rfc3339();

        let deleted = db.execute(
            "DELETE FROM jobs
             WHERE status IN ('completed', 'failed', 'cancelled')
             AND (completed_at < ? OR (completed_at IS NULL AND updated_at < ?))",
            rusqlite::params![cutoff_str, cutoff_str],
        )?;

        app_log_info!("🧹 JOB: Cleaned up {} old jobs older than {} days", deleted, days_old);
        Ok(deleted)
    }

    /// Recover orphaned jobs that have been stuck for too long.
    /// Includes:
    /// - running jobs with stale updated_at
    /// - pending jobs that were previously claimed by a worker (current_file = worker_* marker)
    pub fn recover_orphaned_jobs(&self, timeout_seconds: i64) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Calculate cutoff time
        let cutoff_time = chrono::Utc::now() - chrono::Duration::seconds(timeout_seconds);
        let cutoff_str = cutoff_time.to_rfc3339();

        // Find jobs that appear orphaned:
        // 1) running jobs older than timeout
        // 2) pending jobs that still carry worker claim marker older than timeout
        let mut stmt = db.prepare(
            "SELECT id, status FROM jobs
             WHERE (status = 'running' OR (status = 'pending' AND current_file LIKE 'worker_%'))
             AND (updated_at < ? OR updated_at IS NULL)"
        )?;

        let orphaned_jobs: Vec<(String, String)> = stmt.query_map(
            rusqlite::params![cutoff_str],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        )?.collect::<Result<Vec<_>, _>>()?;

        if orphaned_jobs.is_empty() {
            return Ok(0);
        }

        app_log_warn!(
            "🔄 RECOVERY: Found {} orphaned jobs older than {}s, resetting to pending",
            orphaned_jobs.len(),
            timeout_seconds
        );

        // Reset orphaned jobs to pending status
        let mut recovered_count = 0;
        for (job_id, previous_status) in &orphaned_jobs {
            match db.execute(
                "UPDATE jobs
                 SET status = 'pending', current_file = NULL, updated_at = ?
                 WHERE id = ? AND status IN ('running', 'pending')",
                rusqlite::params![chrono::Utc::now().to_rfc3339(), job_id]
            ) {
                Ok(1) => {
                    app_log_info!(
                        "✅ RECOVERY: Reset orphaned job {} (was {}) to pending",
                        job_id,
                        previous_status
                    );
                    recovered_count += 1;
                }
                Ok(0) => {
                    app_log_warn!("⚠️ RECOVERY: Job {} was already processed by another worker", job_id);
                }
                Ok(n) => {
                    app_log_warn!("⚠️ RECOVERY: Unexpected update count {} for job {}", n, job_id);
                }
                Err(e) => {
                    app_log_error!("❌ RECOVERY: Failed to reset job {}: {}", job_id, e);
                }
            }
        }

        if recovered_count > 0 {
            app_log_info!("✅ RECOVERY: Successfully recovered {} orphaned jobs", recovered_count);
        }

        Ok(recovered_count)
    }

    /// **NEW: Clear jobs from the queue**
    pub fn clear_jobs_queue(&self) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let deleted = db.execute(
            "DELETE FROM jobs WHERE status IN ('pending', 'running')",
            rusqlite::params![],
        )?;

        app_log_info!("🧹 QUEUE: Cleared {} jobs from queue", deleted);
        Ok(deleted)
    }

    /// Clear all jobs regardless of status.
    pub fn clear_all_jobs(&self) -> Result<usize> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let deleted = db.execute("DELETE FROM jobs", rusqlite::params![])?;
        app_log_info!("🧹 JOBS: Cleared all {} jobs", deleted);
        Ok(deleted)
    }

    /// Get aggregate queue health metrics for UI dashboards.
    pub fn get_queue_health_snapshot(&self, stale_running_threshold_seconds: i64) -> Result<serde_json::Value> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        if !self.schema_service.jobs_table_exists(&db) {
            app_log_warn!("⚠️ JOBS TABLE: Jobs table missing during get_queue_health_snapshot, creating it now");
            self.schema_service.ensure_jobs_table_exists(&db)?;
        }

        let now = chrono::Utc::now();
        let now_str = now.to_rfc3339();
        let stale_cutoff = (now - chrono::Duration::seconds(stale_running_threshold_seconds)).to_rfc3339();
        let one_hour_ago = (now - chrono::Duration::hours(1)).to_rfc3339();

        let (
            total,
            pending,
            running,
            completed,
            failed,
            cancelled,
            retry_scheduled,
            retry_ready,
            stale_running,
            orphaned_pending_claims,
            completed_last_hour,
            failed_last_hour,
        ) = db.query_row(
            "SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) as pending,
                COALESCE(SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END), 0) as running,
                COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0) as completed,
                COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) as failed,
                COALESCE(SUM(CASE WHEN status = 'cancelled' THEN 1 ELSE 0 END), 0) as cancelled,
                COALESCE(SUM(CASE WHEN status = 'pending' AND next_retry_at IS NOT NULL AND next_retry_at > ? THEN 1 ELSE 0 END), 0) as retry_scheduled,
                COALESCE(SUM(CASE WHEN status = 'pending' AND next_retry_at IS NOT NULL AND next_retry_at <= ? THEN 1 ELSE 0 END), 0) as retry_ready,
                COALESCE(SUM(CASE WHEN status = 'running' AND updated_at < ? THEN 1 ELSE 0 END), 0) as stale_running,
                COALESCE(SUM(CASE WHEN status = 'pending' AND current_file LIKE 'worker_%' AND updated_at < ? THEN 1 ELSE 0 END), 0) as orphaned_pending_claims,
                COALESCE(SUM(CASE WHEN status = 'completed' AND completed_at >= ? THEN 1 ELSE 0 END), 0) as completed_last_hour,
                COALESCE(SUM(CASE WHEN status = 'failed' AND completed_at >= ? THEN 1 ELSE 0 END), 0) as failed_last_hour
             FROM jobs",
            rusqlite::params![
                now_str,
                now_str,
                stale_cutoff,
                stale_cutoff,
                one_hour_ago,
                one_hour_ago
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            },
        )?;

        let oldest_pending_at: Option<String> = db.query_row(
            "SELECT MIN(created_at) FROM jobs WHERE status = 'pending'",
            rusqlite::params![],
            |row| row.get(0),
        )?;

        let longest_running_since_at: Option<String> = db.query_row(
            "SELECT MIN(COALESCE(started_at, created_at)) FROM jobs WHERE status = 'running'",
            rusqlite::params![],
            |row| row.get(0),
        )?;

        let latest_update_at: Option<String> = db.query_row(
            "SELECT MAX(updated_at) FROM jobs",
            rusqlite::params![],
            |row| row.get(0),
        )?;

        let age_seconds = |timestamp: &Option<String>| -> Option<i64> {
            let parsed = timestamp.as_ref().and_then(|value| {
                chrono::DateTime::parse_from_rfc3339(value)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            })?;
            Some((now - parsed).num_seconds().max(0))
        };

        Ok(serde_json::json!({
            "total": total,
            "pending": pending,
            "running": running,
            "completed": completed,
            "failed": failed,
            "cancelled": cancelled,
            "retry_scheduled": retry_scheduled,
            "retry_ready": retry_ready,
            "stale_running": stale_running,
            "orphaned_pending_claims": orphaned_pending_claims,
            "completed_last_hour": completed_last_hour,
            "failed_last_hour": failed_last_hour,
            "oldest_pending_age_seconds": age_seconds(&oldest_pending_at),
            "longest_running_age_seconds": age_seconds(&longest_running_since_at),
            "latest_update_at": latest_update_at
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_job_queue_service_creation() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).expect("Database service failed to initialize");
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        let schema_service_arc = Arc::new(schema_service);
        
        let job_queue_service = JobQueueService::new(Arc::clone(&db_service_arc), Arc::clone(&schema_service_arc));
        assert!(job_queue_service.db_service.get_db_path().is_ok());
    }

    #[test]
    fn test_job_creation_and_retrieval() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).expect("Database service failed to initialize");
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        let schema_service_arc = Arc::new(schema_service);
        
        let job_queue_service = JobQueueService::new(Arc::clone(&db_service_arc), Arc::clone(&schema_service_arc));
        
        // Initialize schema - just ensure jobs table exists for testing
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        schema_service_arc.ensure_jobs_table_exists(&db).expect("Jobs table setup failed");
        drop(db);

        // Create a job
        let job_id = job_queue_service.create_job("test_job", "/test/path", Some(10)).expect("Failed to create job");
        assert!(!job_id.is_empty());

        // Get the job
        let job = job_queue_service.get_job_by_id(&job_id).expect("Failed to get job");
        assert_eq!(job["id"].as_str().unwrap(), job_id);
        assert_eq!(job["job_type"].as_str().unwrap(), "test_job");
        assert_eq!(job["target_path"].as_str().unwrap(), "/test/path");
        assert_eq!(job["status"].as_str().unwrap(), "pending");
    }

    #[test]
    fn test_job_update_progress() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).expect("Database service failed to initialize");
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        let schema_service_arc = Arc::new(schema_service);
        
        let job_queue_service = JobQueueService::new(Arc::clone(&db_service_arc), Arc::clone(&schema_service_arc));
        
        // Initialize schema - just ensure jobs table exists for testing
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        schema_service_arc.ensure_jobs_table_exists(&db).expect("Jobs table setup failed");
        drop(db);

        // Create a job
        let job_id = job_queue_service.create_job("test_job", "/test/path", Some(10)).expect("Failed to create job");

        // Update job progress
        job_queue_service.update_job_progress(&job_id, "running", Some("file1.jpg"), Some(5), None, None).expect("Failed to update job");

        // Get the job and verify updates
        let job = job_queue_service.get_job_by_id(&job_id).expect("Failed to get job");
        assert_eq!(job["status"].as_str().unwrap(), "running");
        assert_eq!(job["current_file"].as_str().unwrap(), "file1.jpg");
        assert_eq!(job["processed"].as_i64().unwrap(), 5);
    }

    #[test]
    fn test_job_cancellation() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).expect("Database service failed to initialize");
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        let schema_service_arc = Arc::new(schema_service);
        
        let job_queue_service = JobQueueService::new(Arc::clone(&db_service_arc), Arc::clone(&schema_service_arc));
        
        // Initialize schema - just ensure jobs table exists for testing
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        schema_service_arc.ensure_jobs_table_exists(&db).expect("Jobs table setup failed");
        drop(db);

        // Create a job
        let job_id = job_queue_service.create_job("test_job", "/test/path", Some(10)).expect("Failed to create job");

        // Cancel the job
        job_queue_service.cancel_job(&job_id).expect("Failed to cancel job");

        // Get the job and verify it's cancelled
        let job = job_queue_service.get_job_by_id(&job_id).expect("Failed to get job");
        assert_eq!(job["status"].as_str().unwrap(), "cancelled");
    }

    #[test]
    fn test_queue_health_snapshot_metrics() {
        let temp_dir = tempdir().unwrap();
        let db_service = DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).expect("Database service failed to initialize");
        let db_service_arc = Arc::new(db_service);
        let schema_service = SchemaService::new(Arc::clone(&db_service_arc));
        let schema_service_arc = Arc::new(schema_service);
        let job_queue_service = JobQueueService::new(Arc::clone(&db_service_arc), Arc::clone(&schema_service_arc));

        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        schema_service_arc.ensure_jobs_table_exists(&db).expect("Jobs table setup failed");
        drop(db);

        let pending_job = job_queue_service.create_job("file", "/test/pending.jpg", Some(1)).unwrap();
        let running_job = job_queue_service.create_job("file", "/test/running.jpg", Some(1)).unwrap();
        job_queue_service
            .update_job_progress(&running_job, "running", Some("running"), Some(0), None, None)
            .unwrap();

        let completed_job = job_queue_service.create_job("file", "/test/completed.jpg", Some(1)).unwrap();
        job_queue_service
            .update_job_progress(&completed_job, "completed", Some("done"), Some(1), None, None)
            .unwrap();

        let failed_job = job_queue_service.create_job("file", "/test/failed.jpg", Some(1)).unwrap();
        job_queue_service
            .update_job_progress(
                &failed_job,
                "failed",
                Some("failed"),
                Some(0),
                Some(&["boom".to_string()]),
                None,
            )
            .unwrap();

        let cancelled_job = job_queue_service.create_job("file", "/test/cancelled.jpg", Some(1)).unwrap();
        job_queue_service.cancel_job(&cancelled_job).unwrap();

        let retry_job = job_queue_service.create_job("file", "/test/retry.jpg", Some(1)).unwrap();
        job_queue_service
            .schedule_job_retry(&retry_job, "Connection timeout")
            .unwrap();

        let stale_running_job = job_queue_service.create_job("file", "/test/stale-running.jpg", Some(1)).unwrap();
        job_queue_service
            .update_job_progress(&stale_running_job, "running", Some("stale"), Some(0), None, None)
            .unwrap();

        let orphaned_pending_job = job_queue_service.create_job("file", "/test/orphaned-pending.jpg", Some(1)).unwrap();

        let two_hours_ago = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        let connection = db_service_arc.get_connection();
        let db = connection.lock().unwrap();
        db.execute(
            "UPDATE jobs SET updated_at = ? WHERE id = ?",
            rusqlite::params![two_hours_ago, stale_running_job],
        )
        .unwrap();
        db.execute(
            "UPDATE jobs SET current_file = 'worker_1', updated_at = ? WHERE id = ?",
            rusqlite::params![two_hours_ago, orphaned_pending_job],
        )
        .unwrap();
        drop(db);

        let snapshot = job_queue_service.get_queue_health_snapshot(600).unwrap();

        assert_eq!(snapshot["total"].as_i64().unwrap(), 8);
        assert_eq!(snapshot["pending"].as_i64().unwrap(), 3);
        assert_eq!(snapshot["running"].as_i64().unwrap(), 2);
        assert_eq!(snapshot["completed"].as_i64().unwrap(), 1);
        assert_eq!(snapshot["failed"].as_i64().unwrap(), 1);
        assert_eq!(snapshot["cancelled"].as_i64().unwrap(), 1);
        assert_eq!(snapshot["retry_scheduled"].as_i64().unwrap(), 1);
        assert_eq!(snapshot["stale_running"].as_i64().unwrap(), 1);
        assert_eq!(snapshot["orphaned_pending_claims"].as_i64().unwrap(), 1);
        assert!(snapshot["completed_last_hour"].as_i64().unwrap() >= 1);
        assert!(snapshot["failed_last_hour"].as_i64().unwrap() >= 1);
        assert!(snapshot["oldest_pending_age_seconds"].is_number());
        assert!(snapshot["longest_running_age_seconds"].is_number());

        // Keep variables used so clippy does not complain in test builds.
        assert!(!pending_job.is_empty());
    }
}
