use crate::commands::indexing::{BATCH_SIZE, MAX_CONCURRENT_VIDEOS, WORKER_COUNT};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;

#[tokio::test]
async fn test_semaphore_releases_on_panic() {
    // Test that semaphore permits are properly released even when tasks panic
    let semaphore = Arc::new(Semaphore::new(2));

    // Verify we start with 2 permits
    assert_eq!(semaphore.available_permits(), 2);

    // Simulate worker panic while holding a permit
    let sem_clone = semaphore.clone();
    let handle = tokio::spawn(async move {
        let _permit = sem_clone.acquire().await.unwrap();
        // Simulate some work then panic
        tokio::time::sleep(Duration::from_millis(1)).await;
        panic!("Simulated worker crash");
    });

    // Wait for the task to complete (and panic)
    let result = handle.await;
    assert!(result.is_err(), "Task should have panicked");

    // Permit should be automatically released on panic due to RAII
    // Wait a bit for cleanup
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert_eq!(
        semaphore.available_permits(),
        2,
        "Semaphore should release permits even after panic"
    );
}

#[tokio::test]
async fn test_multiple_panic_scenarios() {
    // Test that multiple panicking tasks don't break semaphore
    let semaphore = Arc::new(Semaphore::new(3));

    // Spawn multiple tasks that will panic
    let mut handles = Vec::new();
    for i in 0..3 {
        let sem_clone = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.unwrap();
            tokio::time::sleep(Duration::from_millis(1)).await;
            panic!("Worker {} crashed", i);
        });
        handles.push(handle);
    }

    // Wait for all tasks to panic
    for handle in handles {
        let result = handle.await;
        assert!(result.is_err(), "All tasks should panic");
    }

    // All permits should be available again
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert_eq!(
        semaphore.available_permits(),
        3,
        "All permits should be released after panics"
    );
}

#[tokio::test]
async fn test_semaphore_timeout_releases_properly() {
    // Test that semaphore handles timeouts correctly
    let semaphore = Arc::new(Semaphore::new(1));

    // Acquire the only permit
    let _permit = semaphore.acquire().await.unwrap();

    // Try to acquire with timeout (should fail)
    let result = timeout(Duration::from_millis(100), semaphore.acquire()).await;
    assert!(result.is_err(), "Should timeout when no permits available");

    // Drop the permit
    drop(_permit);

    // Should be able to acquire again
    let result = timeout(Duration::from_millis(100), semaphore.acquire()).await;
    assert!(result.is_ok(), "Should acquire after permit is released");
}

#[tokio::test]
async fn test_worker_resource_isolation() {
    // Test that workers don't interfere with each other's resources
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_VIDEOS));

    // Spawn workers that acquire permits for different durations
    let mut handles = Vec::new();
    for i in 0..MAX_CONCURRENT_VIDEOS {
        let sem_clone = semaphore.clone();
        let duration = Duration::from_millis(50 * (i as u64 + 1)); // Different durations

        let handle = tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.unwrap();
            tokio::time::sleep(duration).await;
            format!("Worker {} completed", i)
        });
        handles.push(handle);
    }

    // All workers should complete successfully
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        assert_eq!(result, format!("Worker {} completed", i));
    }

    // All permits should be available
    assert_eq!(semaphore.available_permits(), MAX_CONCURRENT_VIDEOS);
}

#[test]
fn test_resource_constants_prevent_exhaustion() {
    // Test that our resource constants are safe for typical hardware

    // Memory safety: batch size shouldn't be too large
    let estimated_batch_memory_mb = BATCH_SIZE * 10; // ~10MB per image estimate
    assert!(
        estimated_batch_memory_mb <= 512, // 512MB limit for batch processing
        "Batch memory usage too high: {}MB (batch size: {})",
        estimated_batch_memory_mb,
        BATCH_SIZE
    );

    // CPU safety: workers shouldn't overwhelm system
    assert!(
        WORKER_COUNT <= num_cpus::get() * 2,
        "Too many workers {} for {} CPU cores",
        WORKER_COUNT,
        num_cpus::get()
    );

    // Video processing safety
    let max_video_cpu_usage = MAX_CONCURRENT_VIDEOS * 150; // 150% per video estimate
    assert!(
        max_video_cpu_usage <= 1000, // 10 cores worth
        "Video processing might use too much CPU: {}%",
        max_video_cpu_usage
    );
}

#[tokio::test]
async fn test_graceful_shutdown_releases_resources() {
    // Test that cancelling work releases resources properly
    let semaphore = Arc::new(Semaphore::new(2));

    // Start work that holds a permit
    let sem_clone = semaphore.clone();
    let handle = tokio::spawn(async move {
        let _permit = sem_clone.acquire().await.unwrap();
        // Simulate long-running work
        tokio::time::sleep(Duration::from_millis(1)).await;
    });

    // Let it acquire the permit
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert_eq!(semaphore.available_permits(), 1);

    // Cancel the work
    handle.abort();

    // Resource should be released
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert_eq!(
        semaphore.available_permits(),
        2,
        "Cancelled task should release its permit"
    );
}
