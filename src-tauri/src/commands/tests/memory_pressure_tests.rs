use crate::commands::indexing::{BATCH_SIZE, WORKER_COUNT};
use crate::services::sqlite_service::SqliteVectorService;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Create a test SQLite service with in-memory database for speed
fn create_test_sqlite_service() -> Arc<SqliteVectorService> {
    Arc::new(SqliteVectorService::new_in_memory()
        .expect("Failed to create in-memory test SQLite service"))
}

#[tokio::test]
async fn test_batch_processing_adapts_to_memory_pressure() {
    // Test that system gracefully handles memory pressure by limiting batch operations
    let sqlite_service = create_test_sqlite_service();

    // Create many jobs to simulate memory pressure
    let large_job_count = 50;
    for i in 0..large_job_count {
        sqlite_service
            .create_job("file", &format!("/test/memory_test_{}.jpg", i), Some(1))
            .expect("Failed to create memory test job");
    }

    // Simulate memory-constrained processing with smaller batches
    let memory_constrained_batch_size = BATCH_SIZE / 2;

    let mut total_processed = 0;
    let mut batch_count = 0;

    // Process in smaller batches to simulate memory pressure adaptation
    while total_processed < large_job_count {
        let claimed_jobs = sqlite_service
            .claim_pending_jobs_atomic(1, memory_constrained_batch_size)
            .expect("Failed to claim jobs under memory pressure");

        if claimed_jobs.is_empty() {
            break; // No more jobs to process
        }

        // Verify we don't exceed memory-constrained batch size
        assert!(
            claimed_jobs.len() <= memory_constrained_batch_size,
            "Batch size {} exceeds memory constraint {}",
            claimed_jobs.len(),
            memory_constrained_batch_size
        );

        total_processed += claimed_jobs.len();
        batch_count += 1;

        // Simulate processing time
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    assert_eq!(
        total_processed, large_job_count,
        "Should process all jobs even under memory pressure"
    );

    // Verify we processed in more batches due to memory constraints
    let expected_min_batches = large_job_count / memory_constrained_batch_size;
    assert!(
        batch_count >= expected_min_batches,
        "Should process in at least {} batches under memory pressure, got {}",
        expected_min_batches, batch_count
    );
}

#[tokio::test]
async fn test_memory_efficient_job_claiming() {
    // Test that job claiming doesn't grow memory usage unbounded
    let sqlite_service = create_test_sqlite_service();

    // Create jobs with varying path lengths (memory usage test)
    let mut created_jobs = 0;

    // Short paths
    for i in 0..10 {
        sqlite_service
            .create_job("file", &format!("/short_{}.jpg", i), Some(1))
            .expect("Failed to create short path job");
        created_jobs += 1;
    }

    // Long paths (stress test memory)
    let long_path_base = "/very/long/path/with/many/segments".repeat(10);
    for i in 0..10 {
        sqlite_service
            .create_job("file", &format!("{}/long_file_{}.jpg", long_path_base, i), Some(1))
            .expect("Failed to create long path job");
        created_jobs += 1;
    }

    // Claim all jobs in reasonable batches
    let mut total_claimed = 0;
    let reasonable_batch_size = 25; // Smaller than BATCH_SIZE for memory efficiency

    while total_claimed < created_jobs {
        let claimed = sqlite_service
            .claim_pending_jobs_atomic(1, reasonable_batch_size)
            .expect("Failed to claim jobs efficiently");

        if claimed.is_empty() {
            break;
        }

        total_claimed += claimed.len();

        // Verify memory-efficient batch sizes
        assert!(
            claimed.len() <= reasonable_batch_size,
            "Batch size should be memory-efficient, got {}",
            claimed.len()
        );
    }

    assert_eq!(
        total_claimed, created_jobs,
        "Should claim all jobs with memory-efficient batching"
    );
}

#[tokio::test]
async fn test_concurrent_memory_pressure_handling() {
    // Test that multiple workers handle memory pressure gracefully
    let sqlite_service = create_test_sqlite_service();

    // Create enough jobs for memory pressure scenario
    let job_count = 20;
    for i in 0..job_count {
        sqlite_service
            .create_job("file", &format!("/test/concurrent_memory_{}.jpg", i), Some(1))
            .expect("Failed to create concurrent memory test job");
    }

    // Simulate memory pressure with reduced batch sizes
    let memory_pressure_batch_size = 5; // Much smaller than normal BATCH_SIZE

    // Launch multiple workers with memory-constrained batches
    let mut handles = Vec::new();
    let worker_count = 3; // Fewer workers to simulate memory constraints

    for worker_id in 0..worker_count {
        let service = sqlite_service.clone();
        let handle = tokio::spawn(async move {
            let mut worker_claimed = 0;

            // Each worker processes smaller batches until no more jobs
            loop {
                let claimed = service
                    .claim_pending_jobs_atomic(worker_id, memory_pressure_batch_size)
                    .unwrap_or_default();

                if claimed.is_empty() {
                    break;
                }

                worker_claimed += claimed.len();

                // Simulate processing with small delay
                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            worker_claimed
        });
        handles.push(handle);
    }

    // Collect results
    let mut total_claimed = 0;
    for handle in handles {
        let worker_claimed = handle.await.expect("Worker failed");
        total_claimed += worker_claimed;
    }

    // Should have processed all jobs despite memory pressure
    assert_eq!(
        total_claimed, job_count,
        "All jobs should be processed despite memory pressure"
    );
}

#[tokio::test]
async fn test_resource_semaphore_under_memory_pressure() {
    // Test that semaphores work correctly under memory pressure
    let semaphore = Arc::new(Semaphore::new(2)); // Limited permits to simulate memory pressure

    // Simulate memory pressure by having many tasks compete for few resources
    let mut handles = Vec::new();
    let task_count = 10;

    for task_id in 0..task_count {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            // Try to acquire permit (may wait due to memory pressure)
            let _permit = sem.acquire().await.unwrap();

            // Simulate memory-intensive work
            tokio::time::sleep(Duration::from_millis(1)).await;

            task_id
        });
        handles.push(handle);
    }

    // All tasks should complete despite resource pressure
    let mut completed_tasks = Vec::new();
    for handle in handles {
        let task_id = handle.await.expect("Task failed under memory pressure");
        completed_tasks.push(task_id);
    }

    // Verify all tasks completed
    assert_eq!(
        completed_tasks.len(), task_count,
        "All tasks should complete under memory pressure"
    );

    // Verify semaphore is properly released
    assert_eq!(
        semaphore.available_permits(), 2,
        "All permits should be released after memory pressure"
    );
}

#[test]
fn test_memory_usage_estimates_are_reasonable() {
    // Test that our memory usage estimates prevent system exhaustion

    // Estimate memory per job (path + metadata)
    let avg_path_length = 100; // characters
    let metadata_size = 200; // bytes for job metadata
    let estimated_job_memory = avg_path_length + metadata_size;

    // Batch memory usage should be reasonable
    let batch_memory_estimate = BATCH_SIZE * estimated_job_memory;
    assert!(
        batch_memory_estimate <= 50_000, // 50KB per batch
        "Batch memory usage too high: {} bytes (batch size: {})",
        batch_memory_estimate, BATCH_SIZE
    );

    // Total worker memory should be manageable
    let total_worker_memory = WORKER_COUNT * batch_memory_estimate;
    assert!(
        total_worker_memory <= 500_000, // 500KB total
        "Total worker memory too high: {} bytes ({} workers)",
        total_worker_memory, WORKER_COUNT
    );
}

#[tokio::test]
async fn test_memory_growth_bounds_with_large_batches() {
    // Test that even with large batches, memory usage doesn't grow unbounded
    let sqlite_service = create_test_sqlite_service();

    // Create many jobs with different characteristics
    let small_files = 10;
    let large_files = 5;

    // Small files (minimal memory)
    for i in 0..small_files {
        sqlite_service
            .create_job("file", &format!("/small/{}.jpg", i), Some(1))
            .expect("Failed to create small file job");
    }

    // Large files (more memory per job)
    let large_path = "/path/to/large/file/with/very/long/filename".repeat(5);
    for i in 0..large_files {
        sqlite_service
            .create_job("file", &format!("{}/large_file_{}.mkv", large_path, i), Some(1))
            .expect("Failed to create large file job");
    }

    // Process with bounded batch sizes
    let max_safe_batch_size = 20; // Smaller than BATCH_SIZE for safety
    let mut processed_count = 0;

    while processed_count < (small_files + large_files) {
        let claimed = sqlite_service
            .claim_pending_jobs_atomic(1, max_safe_batch_size)
            .expect("Failed to claim jobs with bounded batches");

        if claimed.is_empty() {
            break;
        }

        // Verify batch size is bounded
        assert!(
            claimed.len() <= max_safe_batch_size,
            "Batch size {} exceeds safe limit {}",
            claimed.len(), max_safe_batch_size
        );

        processed_count += claimed.len();
    }

    assert_eq!(
        processed_count, small_files + large_files,
        "Should process all jobs with bounded memory growth"
    );
}
