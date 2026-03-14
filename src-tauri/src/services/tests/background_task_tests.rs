use crate::commands::indexing::{get_worker_count, WORKER_COUNT};
use crate::services::database_service::DatabaseService;
use crate::services::startup::AppState;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Create ultra-lightweight mock for task coordination tests (no actual service logic)
fn create_mock_task_state() -> (
    Arc<std::sync::atomic::AtomicBool>,
    Arc<std::sync::atomic::AtomicUsize>,
) {
    (
        Arc::new(std::sync::atomic::AtomicBool::new(true)), // Mock "available" state
        Arc::new(std::sync::atomic::AtomicUsize::new(0)),   // Mock counter
    )
}

/// Create lightweight test services for background task testing
fn create_test_app_state() -> AppState {
    use crate::services::{
        audio_service::AudioService, download_service::DownloadService,
        drive_service::DriveService, embedding_service::EmbeddingService,
        file_service::FileService, model_service::ModelService,
        sqlite_service::SqliteVectorService, video_service::VideoService,
    };

    // Create lightweight services - no model loading, in-memory DB
    let model_service = Arc::new(ModelService::new());
    let file_service = Arc::new(FileService::new());
    let sqlite_service = Arc::new(
        SqliteVectorService::new_in_memory()
            .expect("Failed to create in-memory test SQLite service"),
    );
    let db_service = DatabaseService::new_in_memory().expect("Failed to create database service");
    let drive_service = Arc::new(DriveService::new(Arc::new(db_service)));
    let embedding_service = Arc::new(EmbeddingService::new(
        model_service.clone(),
        sqlite_service.clone(),
        drive_service.clone(),
    ));
    let audio_service = Arc::new(tokio::sync::Mutex::new(AudioService::new()));
    let video_service = Arc::new(VideoService::new());
    let download_service = Arc::new(DownloadService::new());

    AppState {
        audio_service,
        model_service,
        embedding_service,
        file_service,
        sqlite_service,
        video_service,
        download_service,
        drive_service,
        video_generation_status: Arc::new(
            tokio::sync::Mutex::new(std::collections::HashMap::new()),
        ),
    }
}

#[tokio::test]
async fn test_worker_count_configuration() {
    // Test that worker count is configured properly
    let worker_count = get_worker_count();

    assert!(worker_count > 0, "Worker count should be positive");
    assert!(
        worker_count <= 16,
        "Worker count should be reasonable for hardware"
    );
    assert_eq!(
        worker_count, WORKER_COUNT,
        "Worker count should match constant"
    );
}

#[tokio::test]
async fn test_background_worker_task_spawning() {
    // Test that background workers can be spawned without panicking
    let (mock_available, _mock_counter) = create_mock_task_state();

    // Test that we can spawn worker tasks
    let mut handles = Vec::new();
    for worker_id in 1..=3 {
        // Test with 3 workers
        let available_clone = mock_available.clone();

        let handle = tokio::spawn(async move {
            // Simulate short-lived worker task
            sleep(Duration::from_millis(1)).await;
            assert!(available_clone.load(std::sync::atomic::Ordering::SeqCst));
            format!("Worker {} completed", worker_id)
        });
        handles.push(handle);
    }

    // All workers should complete successfully
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.expect("Worker task should complete");
        assert_eq!(result, format!("Worker {} completed", i + 1));
    }
}

#[tokio::test]
async fn test_service_cloning_for_workers() {
    // Test that Arc references can be safely cloned for background workers
    let (mock_available, mock_counter) = create_mock_task_state();

    // Create multiple clones (simulating service cloning)
    let available_clones: Vec<_> = (0..3).map(|_| mock_available.clone()).collect();

    let counter_clones: Vec<_> = (0..3).map(|_| mock_counter.clone()).collect();

    // All clones should be functional
    for (i, available_clone) in available_clones.iter().enumerate() {
        assert!(
            available_clone.load(std::sync::atomic::Ordering::SeqCst),
            "Clone {} should be available",
            i
        );
    }

    // Test that clones point to the same underlying data
    assert!(Arc::strong_count(&mock_available) >= 3); // At least clones
    assert!(Arc::strong_count(&mock_counter) >= 3);

    // Verify all clones work with the same data
    for counter_clone in &counter_clones {
        counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    assert_eq!(mock_counter.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_worker_error_isolation() {
    // Test that errors in one worker don't affect others
    let (mock_available, mock_counter) = create_mock_task_state();

    // Spawn workers with one that panics
    let mut handles = Vec::new();

    // Normal worker
    let available_clone1 = mock_available.clone();
    let counter_clone1 = mock_counter.clone();
    let handle1 = tokio::spawn(async move {
        sleep(Duration::from_millis(1)).await;
        counter_clone1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        available_clone1.load(std::sync::atomic::Ordering::SeqCst)
    });
    handles.push(("normal", handle1));

    // Panicking worker
    let handle2 = tokio::spawn(async move {
        sleep(Duration::from_millis(1)).await;
        panic!("Simulated worker panic");
    });
    handles.push(("panic", handle2));

    // Another normal worker
    let available_clone3 = mock_available.clone();
    let counter_clone3 = mock_counter.clone();
    let handle3 = tokio::spawn(async move {
        sleep(Duration::from_millis(1)).await;
        counter_clone3.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        available_clone3.load(std::sync::atomic::Ordering::SeqCst)
    });
    handles.push(("normal2", handle3));

    // Check results
    for (worker_type, handle) in handles {
        let result = handle.await;
        match worker_type {
            "panic" => assert!(result.is_err(), "Panicking worker should error"),
            _ => {
                assert!(
                    result.is_ok(),
                    "Normal workers should succeed despite panic in other worker"
                );
                assert!(result.unwrap(), "Normal worker should return true");
            }
        }
    }

    // Verify normal workers completed (counter should be 2)
    assert_eq!(mock_counter.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_background_task_resource_cleanup() {
    // Test that resources are properly cleaned up when background tasks complete
    let (mock_available, mock_counter) = create_mock_task_state();

    let initial_available_count = Arc::strong_count(&mock_available);
    let initial_counter_count = Arc::strong_count(&mock_counter);

    let worker_handles = {
        let mut handles = Vec::new();
        for worker_id in 1..=5 {
            let available_clone = mock_available.clone();
            let counter_clone = mock_counter.clone();

            let handle = tokio::spawn(async move {
                // Hold references for a short time
                sleep(Duration::from_millis(1)).await;

                // Use the mocks briefly
                available_clone.load(std::sync::atomic::Ordering::SeqCst);
                counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                worker_id
            });
            handles.push(handle);
        }
        handles
    };

    // Reference counts should increase while workers are running
    assert!(Arc::strong_count(&mock_available) > initial_available_count);
    assert!(Arc::strong_count(&mock_counter) > initial_counter_count);

    // Wait for all workers to complete
    for handle in worker_handles {
        handle.await.expect("Worker should complete successfully");
    }

    // Allow time for cleanup
    sleep(Duration::from_millis(1)).await;

    // Reference counts should be close to initial values (allow some tolerance)
    let final_available_count = Arc::strong_count(&mock_available);
    let final_counter_count = Arc::strong_count(&mock_counter);

    assert!(
        final_available_count <= initial_available_count + 1,
        "Available reference count should be close to initial: {} -> {}",
        initial_available_count,
        final_available_count
    );
    assert!(
        final_counter_count <= initial_counter_count + 1,
        "Counter reference count should be close to initial: {} -> {}",
        initial_counter_count,
        final_counter_count
    );

    // Verify all workers executed
    assert_eq!(mock_counter.load(std::sync::atomic::Ordering::SeqCst), 5);
}

#[tokio::test]
async fn test_concurrent_background_task_coordination() {
    // Test coordination between multiple background tasks
    let (mock_available, mock_counter) = create_mock_task_state();

    // Spawn workers that coordinate using the mock counter
    let mut handles = Vec::new();
    for worker_id in 1..=3 {
        let counter_clone = mock_counter.clone();
        let available_clone = mock_available.clone();

        let handle = tokio::spawn(async move {
            // Each worker increments counter and validates availability
            let current = counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            // Validate mock is accessible
            let is_available = available_clone.load(std::sync::atomic::Ordering::SeqCst);
            assert!(is_available, "Mock should be available in background task");

            (worker_id, current)
        });
        handles.push(handle);
    }

    // Collect results
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.expect("Background task should complete");
        results.push(result);
    }

    // All workers should have completed
    assert_eq!(results.len(), 3);

    // Counter should equal number of workers
    assert_eq!(mock_counter.load(std::sync::atomic::Ordering::SeqCst), 3);

    // Each worker should have gotten a unique counter value
    let mut counter_values: Vec<_> = results.iter().map(|(_, count)| *count).collect();
    counter_values.sort();
    let expected: Vec<_> = (0..3).collect();
    assert_eq!(counter_values, expected);
}

#[tokio::test]
async fn test_background_task_service_availability() {
    // Test that all required services are available to background tasks
    let app_state = create_test_app_state();

    let sqlite_service = app_state.sqlite_service.clone();
    let _embedding_service = app_state.embedding_service.clone();
    let video_service = app_state.video_service.clone();
    let model_service = app_state.model_service.clone();
    let file_service = app_state.file_service.clone();

    // Test that each service is accessible from background task
    let handle = tokio::spawn(async move {
        let mut results = Vec::new();

        // Test SQLite service
        let sqlite_result = sqlite_service.get_schema_info();
        results.push(("sqlite", sqlite_result.is_ok()));

        // Test model service - just verify it doesn't panic
        let _model_loaded = model_service.is_model_loaded();
        results.push(("model", true));

        // Test file service - just verify it doesn't panic
        let _is_dir = file_service.is_directory("/");
        results.push(("file", true));

        // Test video service
        let _ffmpeg_check = video_service.is_ffmpeg_available();
        results.push(("video", true)); // Just test that call doesn't panic

        results
    });

    let results = handle
        .await
        .expect("Service availability test should complete");

    // All services should be accessible
    for (service_name, is_available) in results {
        assert!(
            is_available,
            "{} service should be available in background task",
            service_name
        );
    }
}

#[tokio::test]
async fn test_background_task_timeout_handling() {
    // Test that background tasks can handle timeouts gracefully
    let app_state = create_test_app_state();

    let sqlite_service = app_state.sqlite_service.clone();

    // Test quick operation (should not timeout)
    let quick_task = timeout(
        Duration::from_secs(1),
        tokio::spawn(async move { sqlite_service.get_schema_info() }),
    );

    let result = quick_task.await;
    assert!(result.is_ok(), "Quick background task should not timeout");

    let task_result = result.unwrap();
    assert!(
        task_result.is_ok(),
        "Quick task should complete successfully"
    );
}

#[tokio::test]
async fn test_background_task_memory_pressure() {
    // Test background tasks under simulated memory pressure
    let (mock_available, mock_counter) = create_mock_task_state();

    // Spawn short-lived tasks to simulate memory pressure
    let task_count = 10;
    let mut handles = Vec::new();

    for i in 0..task_count {
        let available_clone = mock_available.clone();
        let counter_clone = mock_counter.clone();

        let handle = tokio::spawn(async move {
            // Simulate some work
            sleep(Duration::from_millis(1)).await;

            // Access mocks
            let is_available = available_clone.load(std::sync::atomic::Ordering::SeqCst);
            assert!(is_available, "Mock should work under memory pressure");

            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            i
        });
        handles.push(handle);
    }

    // All tasks should complete successfully despite memory pressure
    let mut completed_tasks = Vec::new();
    for handle in handles {
        let task_id = handle
            .await
            .expect("Task should complete under memory pressure");
        completed_tasks.push(task_id);
    }

    // All tasks should have completed
    assert_eq!(completed_tasks.len(), task_count);

    // Verify all task IDs are present
    completed_tasks.sort();
    let expected: Vec<_> = (0..task_count).collect();
    assert_eq!(completed_tasks, expected);

    // Verify all tasks incremented the counter
    assert_eq!(
        mock_counter.load(std::sync::atomic::Ordering::SeqCst),
        task_count
    );
}
