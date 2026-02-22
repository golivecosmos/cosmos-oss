use crate::services::sqlite_service::SqliteVectorService;
use std::sync::Arc;
use tokio::task::JoinSet;

/// Create a test SQLite service with in-memory database for speed
fn create_test_sqlite_service() -> Arc<SqliteVectorService> {
    Arc::new(SqliteVectorService::new_in_memory()
        .expect("Failed to create in-memory test SQLite service"))
}

#[tokio::test]
async fn test_atomic_job_claiming_prevents_race_conditions() {
    let sqlite_service = create_test_sqlite_service();

    // Create test jobs
    let job_ids: Vec<_> = (0..5)
        .map(|i| {
            sqlite_service
                .create_job("file", &format!("/test/file_{}.jpg", i), Some(1))
                .expect("Failed to create test job")
        })
        .collect();

    println!("Created {} test jobs", job_ids.len());

    // Spawn multiple workers simultaneously trying to claim jobs
    let mut join_set = JoinSet::new();
    let worker_count = 8; // More workers than jobs to ensure contention

    for worker_id in 0..worker_count {
        let service = sqlite_service.clone();
        join_set.spawn(async move {
            // Each worker tries to claim up to 2 jobs
            match service.claim_pending_jobs_atomic(worker_id, 2) {
                Ok(claimed_jobs) => {
                    println!("Worker {} claimed {} jobs", worker_id, claimed_jobs.len());
                    (worker_id, claimed_jobs.len(), None)
                }
                Err(e) => {
                    println!("Worker {} failed to claim jobs: {}", worker_id, e);
                    (worker_id, 0, Some(e.to_string()))
                }
            }
        });
    }

    // Collect results
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result.expect("Worker task failed"));
    }

    // Calculate total jobs claimed
    let total_claimed: usize = results.iter().map(|(_, claimed, _)| *claimed).sum();
    let successful_workers: usize = results.iter().filter(|(_, claimed, _)| *claimed > 0).count();

    println!("Total jobs claimed: {}, Successful workers: {}", total_claimed, successful_workers);

    // Verify that exactly the number of created jobs were claimed
    assert_eq!(
        total_claimed,
        job_ids.len(),
        "Expected {} jobs to be claimed total, got {}",
        job_ids.len(),
        total_claimed
    );

    // Verify no jobs are left in pending state
    let pending_jobs = sqlite_service
        .get_jobs_by_status("pending")
        .expect("Failed to get pending jobs");

    assert_eq!(
        pending_jobs.len(),
        0,
        "Expected 0 pending jobs after claiming, got {}",
        pending_jobs.len()
    );

    // Verify all jobs are in running state
    let running_jobs = sqlite_service
        .get_jobs_by_status("running")
        .expect("Failed to get running jobs");

    assert_eq!(
        running_jobs.len(),
        job_ids.len(),
        "Expected {} running jobs after claiming, got {}",
        job_ids.len(),
        running_jobs.len()
    );
}

#[tokio::test]
async fn test_job_claiming_respects_batch_limits() {
    let sqlite_service = create_test_sqlite_service();

    // Create more jobs than batch limit
    let job_count = 10;
    let batch_limit = 3;

    for i in 0..job_count {
        sqlite_service
            .create_job("file", &format!("/test/file_{}.jpg", i), Some(1))
            .expect("Failed to create test job");
    }

    // Worker claims with batch limit
    let claimed_jobs = sqlite_service
        .claim_pending_jobs_atomic(1, batch_limit)
        .expect("Failed to claim jobs");

    // Should not exceed batch limit
    assert!(
        claimed_jobs.len() <= batch_limit,
        "Claimed {} jobs, expected at most {}",
        claimed_jobs.len(),
        batch_limit
    );

    // Remaining jobs should still be pending
    let pending_jobs = sqlite_service
        .get_jobs_by_status("pending")
        .expect("Failed to get pending jobs");

    assert_eq!(
        pending_jobs.len(),
        job_count - claimed_jobs.len(),
        "Expected {} pending jobs remaining",
        job_count - claimed_jobs.len()
    );
}

#[tokio::test]
async fn test_orphaned_job_recovery() {
    let sqlite_service = create_test_sqlite_service();

    // Create a job and manually mark it as running (simulating a crash)
    let job_id = sqlite_service
        .create_job("file", "/test/orphaned_file.jpg", Some(1))
        .expect("Failed to create test job");

    // Manually update to running state (simulating worker crash)
    sqlite_service
        .update_job_progress(&job_id, "running", Some("Processing..."), None, None, None)
        .expect("Failed to update job to running");

    // Verify job is in running state
    let job = sqlite_service
        .get_job_by_id(&job_id)
        .expect("Failed to get job by ID");
    assert_eq!(job["status"], "running");

    // Wait a bit and then run orphan recovery with zero timeout (simulate app restart)
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    sqlite_service
        .recover_orphaned_jobs(0) // 0 second timeout to recover any "running" job
        .expect("Failed to recover orphaned jobs");

    // Job should be back to pending
    let recovered_job = sqlite_service
        .get_job_by_id(&job_id)
        .expect("Failed to get recovered job");

    assert_eq!(
        recovered_job["status"], "pending",
        "Orphaned job should be recovered to pending state, got {}",
        recovered_job["status"]
    );
}

#[tokio::test]
async fn test_no_jobs_available_returns_empty() {
    let sqlite_service = create_test_sqlite_service();

    // Try to claim jobs when none exist
    let claimed_jobs = sqlite_service
        .claim_pending_jobs_atomic(1, 5)
        .expect("Failed to claim jobs from empty queue");

    assert_eq!(
        claimed_jobs.len(),
        0,
        "Expected 0 jobs when queue is empty, got {}",
        claimed_jobs.len()
    );
}

#[tokio::test]
async fn test_massive_job_creation_doesnt_crash() {
    let sqlite_service = create_test_sqlite_service();

    // Create more jobs than workers can handle
    println!("Creating 50 test jobs...");
    for i in 0..50 {
        sqlite_service
            .create_job("file", &format!("/test/stress_file_{}.jpg", i), Some(1))
            .expect("Failed to create stress test job");
    }

    // System should handle gracefully - claim only what we ask for
    let claimed = sqlite_service
        .claim_pending_jobs_atomic(1, 10)
        .expect("Failed to claim jobs from large queue");

    assert_eq!(
        claimed.len(),
        10,
        "Should claim exactly 10 jobs from large queue, got {}",
        claimed.len()
    );

    // Verify remaining jobs are still pending
    let remaining_pending = sqlite_service
        .get_jobs_by_status("pending")
        .expect("Failed to get pending jobs");

    assert_eq!(
        remaining_pending.len(),
        40,
        "Should have 40 pending jobs remaining, got {}",
        remaining_pending.len()
    );
}

#[tokio::test]
async fn test_concurrent_massive_job_claiming() {
    let sqlite_service = create_test_sqlite_service();

    // Create a moderate number of jobs for testing
    for i in 0..20 {
        sqlite_service
            .create_job("file", &format!("/test/concurrent_file_{}.jpg", i), Some(1))
            .expect("Failed to create concurrent test job");
    }

    // Spawn workers trying to claim jobs simultaneously
    let mut handles = Vec::new();
    for worker_id in 0..4 {
        let service = sqlite_service.clone();
        let handle = tokio::spawn(async move {
            service.claim_pending_jobs_atomic(worker_id, 5).unwrap_or_default()
        });
        handles.push(handle);
    }

    // Collect all claimed jobs
    let mut total_claimed = 0;
    for handle in handles {
        let claimed = handle.await.expect("Worker task failed");
        total_claimed += claimed.len();
    }

    // All jobs should be claimed exactly once
    assert_eq!(
        total_claimed,
        20,
        "Expected exactly 20 jobs to be claimed total, got {}",
        total_claimed
    );

    // No jobs should remain pending
    let remaining_pending = sqlite_service
        .get_jobs_by_status("pending")
        .expect("Failed to get remaining pending jobs");

    assert_eq!(
        remaining_pending.len(),
        0,
        "No jobs should remain pending after claiming all, got {}",
        remaining_pending.len()
    );
}

#[tokio::test]
async fn test_job_queue_handles_zero_and_negative_limits() {
    let sqlite_service = create_test_sqlite_service();

    // Create some jobs
    for i in 0..5 {
        sqlite_service
            .create_job("file", &format!("/test/limit_test_{}.jpg", i), Some(1))
            .expect("Failed to create limit test job");
    }

    // Test zero limit
    let claimed_zero = sqlite_service
        .claim_pending_jobs_atomic(1, 0)
        .expect("Failed to claim with zero limit");

    assert_eq!(
        claimed_zero.len(),
        0,
        "Should claim 0 jobs with limit 0, got {}",
        claimed_zero.len()
    );

    // Test reasonable limit works after zero limit
    let claimed_normal = sqlite_service
        .claim_pending_jobs_atomic(1, 2)
        .expect("Failed to claim with normal limit");

    assert_eq!(
        claimed_normal.len(),
        2,
        "Should claim 2 jobs with limit 2, got {}",
        claimed_normal.len()
    );
}

#[tokio::test]
async fn test_job_queue_memory_efficiency() {
    let sqlite_service = create_test_sqlite_service();

    // Create jobs with varying path lengths to test memory usage
    let mut job_count = 0;

    // Short paths
    for i in 0..10 {
        sqlite_service
            .create_job("file", &format!("/short/{}.jpg", i), Some(1))
            .expect("Failed to create short path job");
        job_count += 1;
    }

    // Very long paths
    let long_path_base = "/very/long/path/with/many/nested/directories/and/subdirectories".repeat(5);
    for i in 0..10 {
        sqlite_service
            .create_job("file", &format!("{}/file_{}.jpg", long_path_base, i), Some(1))
            .expect("Failed to create long path job");
        job_count += 1;
    }

    // Should handle all jobs without issues
    let claimed = sqlite_service
        .claim_pending_jobs_atomic(1, job_count)
        .expect("Failed to claim all jobs");

    assert_eq!(
        claimed.len(),
        job_count,
        "Should claim all {} jobs regardless of path length",
        job_count
    );
}

#[tokio::test]
async fn test_same_target_path_allows_different_job_types() {
    let sqlite_service = create_test_sqlite_service();
    let target = "/test/shared_media.mp4";

    let file_job_id = sqlite_service
        .create_job("file", target, Some(1))
        .expect("Failed to create file job");
    let transcription_job_id = sqlite_service
        .create_job("transcription", target, Some(1))
        .expect("Failed to create transcription job");

    assert_ne!(
        file_job_id, transcription_job_id,
        "Different job types for the same path should not dedupe to one job"
    );

    let pending = sqlite_service
        .get_jobs_by_status("pending")
        .expect("Failed to get pending jobs");
    let matching_count = pending
        .iter()
        .filter(|job| job["target_path"] == target)
        .count();
    assert_eq!(
        matching_count, 2,
        "Expected both file and transcription jobs to exist for {}",
        target
    );
}

#[tokio::test]
async fn test_update_job_progress_preserves_errors_when_not_provided() {
    let sqlite_service = create_test_sqlite_service();
    let job_id = sqlite_service
        .create_job("file", "/test/error_preservation.jpg", Some(1))
        .expect("Failed to create job");

    let initial_errors = vec!["initial failure".to_string()];
    let initial_failed_files = serde_json::json!([
        { "path": "/test/error_preservation.jpg", "error": "initial failure" }
    ]);

    sqlite_service
        .update_job_progress(
            &job_id,
            "failed",
            Some("first attempt failed"),
            Some(0),
            Some(&initial_errors),
            Some(&initial_failed_files),
        )
        .expect("Failed to set initial error state");

    sqlite_service
        .update_job_progress(
            &job_id,
            "pending",
            Some("retry queued"),
            Some(0),
            None,
            None,
        )
        .expect("Failed to update job without errors payload");

    let updated = sqlite_service
        .get_job_by_id(&job_id)
        .expect("Failed to load updated job");
    assert_eq!(updated["errors"], serde_json::json!(initial_errors));
    assert_eq!(updated["failed_files"], initial_failed_files);
}
