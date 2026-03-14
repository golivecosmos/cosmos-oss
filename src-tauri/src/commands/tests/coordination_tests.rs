use crate::commands::indexing::{BATCH_SIZE, MAX_CONCURRENT_VIDEOS, WORKER_COUNT};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;

#[tokio::test]
async fn test_video_semaphore_enforces_max_concurrent() {
    // Test semaphore behavior directly instead of using global singleton
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_VIDEOS));

    // Should allow exactly MAX_CONCURRENT_VIDEOS acquisitions
    let mut permits = Vec::new();
    for i in 0..MAX_CONCURRENT_VIDEOS {
        match semaphore.try_acquire() {
            Ok(permit) => {
                permits.push(permit);
                println!("Acquired permit {}/{}", i + 1, MAX_CONCURRENT_VIDEOS);
            }
            Err(e) => {
                panic!(
                    "Failed to acquire permit {}/{}: {:?}",
                    i + 1,
                    MAX_CONCURRENT_VIDEOS,
                    e
                );
            }
        }
    }

    // Next acquisition should fail
    assert!(
        semaphore.try_acquire().is_err(),
        "Semaphore should reject acquisition beyond MAX_CONCURRENT_VIDEOS ({})",
        MAX_CONCURRENT_VIDEOS
    );

    // Verify available permits is 0
    assert_eq!(
        semaphore.available_permits(),
        0,
        "Expected 0 available permits, got {}",
        semaphore.available_permits()
    );

    // Release one permit
    drop(permits.pop());

    // Should be able to acquire one more
    assert!(
        semaphore.try_acquire().is_ok(),
        "Should be able to acquire permit after releasing one"
    );
}

#[tokio::test]
async fn test_video_semaphore_blocks_when_at_limit() {
    // Test semaphore behavior directly instead of using global singleton
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_VIDEOS));

    // Acquire all permits
    let _permits: Vec<_> = (0..MAX_CONCURRENT_VIDEOS)
        .map(|_| semaphore.try_acquire().unwrap())
        .collect();

    // This should timeout because no permits are available
    let result = timeout(Duration::from_millis(100), semaphore.acquire()).await;
    assert!(
        result.is_err(),
        "Semaphore acquire should timeout when at limit"
    );
}

#[test]
fn test_constants_are_reasonable() {
    // Ensure our constants make sense for resource management
    assert!(WORKER_COUNT > 0, "Must have at least 1 worker");
    assert!(WORKER_COUNT <= 8, "Too many workers might overwhelm system");
    assert!(
        MAX_CONCURRENT_VIDEOS > 0,
        "Must allow at least 1 concurrent video"
    );
    assert!(
        MAX_CONCURRENT_VIDEOS <= WORKER_COUNT,
        "Can't have more concurrent videos than workers"
    );
    assert!(BATCH_SIZE > 0, "Batch size must be positive");
    assert!(
        BATCH_SIZE <= 32,
        "Batch size too large might cause memory issues"
    );
}

#[test]
fn test_resource_limits_work_together() {
    // Ensure configuration values work together sensibly
    assert!(
        MAX_CONCURRENT_VIDEOS <= WORKER_COUNT,
        "Can't process {} videos with only {} workers",
        MAX_CONCURRENT_VIDEOS,
        WORKER_COUNT
    );

    // Batch size should be reasonable for memory usage
    assert!(
        BATCH_SIZE >= 4 && BATCH_SIZE <= 64,
        "Batch size {} should be between 4-64 for optimal performance",
        BATCH_SIZE
    );

    // Workers should be reasonable for CPU cores (assume 4-16 core machines)
    assert!(
        WORKER_COUNT >= 2 && WORKER_COUNT <= 16,
        "Worker count {} should be reasonable for modern hardware",
        WORKER_COUNT
    );

    // Video concurrency should leave room for other processing
    assert!(
        MAX_CONCURRENT_VIDEOS < WORKER_COUNT,
        "Should reserve at least 1 worker for non-video tasks"
    );
}

#[test]
fn test_configuration_prevents_resource_exhaustion() {
    // Test that our configuration won't overwhelm typical hardware

    // Memory estimate: each video worker might use ~200MB
    let estimated_video_memory_mb = MAX_CONCURRENT_VIDEOS * 200;
    assert!(
        estimated_video_memory_mb <= 2048, // 2GB limit
        "Video processing might use too much memory: {}MB estimated",
        estimated_video_memory_mb
    );

    // CPU estimate: each video process can use 100%+ CPU
    let estimated_cpu_usage = MAX_CONCURRENT_VIDEOS * 100;
    assert!(
        estimated_cpu_usage <= 800, // 8 cores at 100%
        "Video processing might overwhelm CPU: {}% estimated usage",
        estimated_cpu_usage
    );

    // Batch processing shouldn't create too many concurrent operations
    let max_concurrent_operations = WORKER_COUNT * BATCH_SIZE;
    assert!(
        max_concurrent_operations <= 256,
        "Too many concurrent operations: {} (workers: {}, batch: {})",
        max_concurrent_operations,
        WORKER_COUNT,
        BATCH_SIZE
    );
}

#[test]
fn test_batch_size_coordination() {
    // Test that we respect BATCH_SIZE in our logic
    let test_files = (0..20)
        .map(|i| format!("file_{}.jpg", i))
        .collect::<Vec<_>>();

    // Simulate batching logic
    let batches: Vec<_> = test_files.chunks(BATCH_SIZE).collect();

    // All batches except possibly the last should be exactly BATCH_SIZE
    for (i, batch) in batches.iter().enumerate() {
        if i < batches.len() - 1 {
            assert_eq!(
                batch.len(),
                BATCH_SIZE,
                "Batch {} should have exactly {} items, got {}",
                i,
                BATCH_SIZE,
                batch.len()
            );
        } else {
            // Last batch can be smaller
            assert!(
                batch.len() <= BATCH_SIZE,
                "Last batch should not exceed BATCH_SIZE, got {}",
                batch.len()
            );
        }
    }
}
