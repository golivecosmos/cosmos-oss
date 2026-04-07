use crate::services::clustering_service::{FileCluster, FilePosition2D};
use crate::AppState;
use crate::{app_log_error, app_log_info};
use tauri::State;

/// Compute file clusters from existing embeddings
#[tauri::command]
pub async fn compute_clusters(state: State<'_, AppState>) -> Result<Vec<FileCluster>, String> {
    app_log_info!("🧮 CMD: compute_clusters called");

    match state.clustering_service.compute_clusters() {
        Ok(clusters) => {
            app_log_info!(
                "✅ CMD: compute_clusters returned {} clusters",
                clusters.len()
            );
            Ok(clusters)
        }
        Err(e) => {
            app_log_error!("❌ CMD: compute_clusters failed: {}", e);
            Err(format!("Failed to compute clusters: {}", e))
        }
    }
}

/// Get all computed clusters
#[tauri::command]
pub async fn get_clusters(state: State<'_, AppState>) -> Result<Vec<FileCluster>, String> {
    state
        .clustering_service
        .get_clusters()
        .map_err(|e| format!("Failed to get clusters: {}", e))
}

/// Get all file positions for the visual map
#[tauri::command]
pub async fn get_file_positions(state: State<'_, AppState>) -> Result<Vec<FilePosition2D>, String> {
    state
        .clustering_service
        .get_file_positions()
        .map_err(|e| format!("Failed to get file positions: {}", e))
}

/// Get files belonging to a specific cluster
#[tauri::command]
pub async fn get_cluster_files(
    cluster_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<FilePosition2D>, String> {
    state
        .clustering_service
        .get_cluster_files(cluster_id)
        .map_err(|e| format!("Failed to get cluster files: {}", e))
}
