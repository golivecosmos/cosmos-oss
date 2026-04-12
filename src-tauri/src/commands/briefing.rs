use crate::services::clustering_service::FileCluster;
use crate::AppState;
use crate::{app_log_error, app_log_info, app_log_warn};
use serde::Serialize;
use tauri::{Emitter, State};

#[derive(Debug, Clone, Serialize)]
pub struct ClusterInsight {
    pub cluster_id: i64,
    pub llm_name: String,
    pub llm_insight: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BriefingNotice {
    pub notice_text: String,
    pub notice_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BriefingResult {
    pub cluster_insights: Vec<ClusterInsight>,
    pub notices: Vec<BriefingNotice>,
    pub used_llm: bool,
}

/// Generate a briefing for the current file collection.
/// Uses Gemma 4 for natural language insights, falls back to TF-IDF stats.
#[tauri::command]
pub async fn generate_briefing(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<BriefingResult, String> {
    app_log_info!("📊 BRIEFING: Generating briefing");

    // Get current clusters
    let clusters = state
        .clustering_service
        .get_clusters()
        .map_err(|e| format!("Failed to get clusters: {}", e))?;

    if clusters.is_empty() {
        return Ok(BriefingResult {
            cluster_insights: vec![],
            notices: vec![],
            used_llm: false,
        });
    }

    // Try Gemma 4 enrichment
    let gemma_available = state.gemma4_service.is_available();

    let mut insights = Vec::new();
    let mut notices = Vec::new();
    let mut used_llm = false;

    if gemma_available {
        app_log_info!("🧠 BRIEFING: Gemma 4 available, generating LLM insights");

        // Ensure model is loaded before inference
        if let Err(e) = state.gemma4_service.ensure_loaded() {
            app_log_warn!("⚠️ BRIEFING: Failed to load Gemma 4: {}, falling back to TF-IDF", e);
        } else {
            used_llm = true;
        }

        // Generate per-cluster insights
        for cluster in &clusters {
            let _ = app_handle.emit(
                "briefing_progress",
                serde_json::json!({
                    "stage": "enriching_cluster",
                    "cluster_name": cluster.name,
                    "cluster_id": cluster.cluster_id,
                }),
            );

            // Get representative files for this cluster
            let files = state
                .clustering_service
                .get_cluster_files(cluster.cluster_id)
                .unwrap_or_default();

            let file_summary: String = files
                .iter()
                .take(10)
                .map(|f| {
                    let name = f.file_path.split('/').last().unwrap_or(&f.file_path);
                    format!("- {} ({})", name, f.source_type)
                })
                .collect::<Vec<_>>()
                .join("\n");

            let prompt = format!(
                "<start_of_turn>user\nYou are analyzing a group of local files. Here are some files from this cluster:\n\n{}\n\nTotal files in group: {}\nCurrent name: {}\n\nGive this group a short, descriptive name (3-5 words). Then write one sentence describing what makes this group interesting or notable.\n\nFormat:\nName: [name]\nInsight: [insight]\n<end_of_turn>\n<start_of_turn>model\n",
                file_summary, cluster.file_count, cluster.name
            );

            if let Some(response) = state.gemma4_service.infer(&prompt, 256) {
                let (name, insight) = parse_cluster_response(&response, cluster);
                insights.push(ClusterInsight {
                    cluster_id: cluster.cluster_id,
                    llm_name: name.clone(),
                    llm_insight: insight.clone(),
                });

                // Persist to DB
                let _ = state.sqlite_service.update_cluster_enrichment(
                    cluster.cluster_id,
                    &name,
                    &insight,
                );
            } else {
                // Fallback for this cluster
                insights.push(tfidf_fallback(cluster));
            }
        }

        // Generate cross-cluster "What I noticed" observations
        let cluster_summary: String = clusters
            .iter()
            .take(10)
            .map(|c| {
                let insight = insights
                    .iter()
                    .find(|i| i.cluster_id == c.cluster_id)
                    .map(|i| i.llm_name.as_str())
                    .unwrap_or(&c.name);
                format!("- {} ({} files, type: {})", insight, c.file_count, c.dominant_type)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let notices_prompt = format!(
            "<start_of_turn>user\nYou are analyzing someone's local file collection. Here are the top clusters:\n\n{}\n\nList 3-5 surprising or useful observations about this collection. Focus on:\n- Connections between different clusters\n- Missing things (e.g., code with no tests, projects with no documentation)\n- Patterns (growing topics, stale areas)\n- Anomalies (outliers, misplaced files)\n\nBe specific. Reference actual cluster names and file counts. One sentence each. Output as numbered list.\n<end_of_turn>\n<start_of_turn>model\n",
            cluster_summary
        );

        if let Some(response) = state.gemma4_service.infer(&notices_prompt, 512) {
            notices = parse_notices(&response);
        }

        // Persist notices
        let notice_tuples: Vec<(String, String)> = notices
            .iter()
            .map(|n| (n.notice_text.clone(), n.notice_type.clone()))
            .collect();
        let _ = state.sqlite_service.save_briefing_notices(&notice_tuples);
    } else {
        app_log_info!("📊 BRIEFING: Gemma 4 not available, using TF-IDF fallback");

        // TF-IDF fallback — structured stats, not natural language
        for cluster in &clusters {
            insights.push(tfidf_fallback(cluster));
        }

        // Statistical notices
        let total_files: usize = clusters.iter().map(|c| c.file_count).sum();
        let largest = clusters.iter().max_by_key(|c| c.file_count);

        if let Some(largest) = largest {
            notices.push(BriefingNotice {
                notice_text: format!(
                    "Your largest cluster \"{}\" has {} files ({:.0}% of total)",
                    largest.name,
                    largest.file_count,
                    (largest.file_count as f64 / total_files as f64) * 100.0
                ),
                notice_type: "stat".to_string(),
            });
        }

        notices.push(BriefingNotice {
            notice_text: format!(
                "{} files organized into {} clusters",
                total_files,
                clusters.len()
            ),
            notice_type: "stat".to_string(),
        });
    }

    let _ = app_handle.emit(
        "briefing_complete",
        serde_json::json!({ "used_llm": used_llm }),
    );

    app_log_info!(
        "✅ BRIEFING: Generated {} insights, {} notices (LLM: {})",
        insights.len(),
        notices.len(),
        used_llm
    );

    Ok(BriefingResult {
        cluster_insights: insights,
        notices,
        used_llm,
    })
}

fn parse_cluster_response(response: &str, cluster: &FileCluster) -> (String, String) {
    let mut name = cluster.name.clone();
    let mut insight = String::new();

    for line in response.lines() {
        let line = line.trim();
        if let Some(n) = line.strip_prefix("Name:") {
            name = n.trim().to_string();
        } else if let Some(i) = line.strip_prefix("Insight:") {
            insight = i.trim().to_string();
        }
    }

    if name.is_empty() {
        name = cluster.name.clone();
    }
    if insight.is_empty() {
        insight = format!("{} files, mostly {}", cluster.file_count, cluster.dominant_type);
    }

    (name, insight)
}

fn parse_notices(response: &str) -> Vec<BriefingNotice> {
    response
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            // Match numbered list items: "1. ...", "2. ...", etc.
            let text = if let Some(rest) = line.strip_prefix(|c: char| c.is_ascii_digit()) {
                rest.trim_start_matches('.').trim_start_matches(')').trim()
            } else if line.starts_with('-') || line.starts_with('•') {
                line.trim_start_matches(['-', '•']).trim()
            } else {
                return None;
            };

            if text.len() < 10 {
                return None;
            }

            let notice_type = if text.contains("missing") || text.contains("no ") {
                "missing"
            } else if text.contains("connection") || text.contains("related") || text.contains("across") {
                "connection"
            } else if text.contains("growing") || text.contains("new") || text.contains("added") {
                "growth"
            } else {
                "observation"
            };

            Some(BriefingNotice {
                notice_text: text.to_string(),
                notice_type: notice_type.to_string(),
            })
        })
        .take(5)
        .collect()
}

fn tfidf_fallback(cluster: &FileCluster) -> ClusterInsight {
    ClusterInsight {
        cluster_id: cluster.cluster_id,
        llm_name: cluster.name.clone(),
        llm_insight: format!(
            "{} {} files",
            cluster.file_count, cluster.dominant_type
        ),
    }
}
