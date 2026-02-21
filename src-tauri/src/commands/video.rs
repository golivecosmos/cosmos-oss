use serde::{Deserialize, Serialize};
use tauri::State;
use crate::services::startup::AppState;
use crate::services::app_installation_service::AppInstallationService;
use crate::services::generations_service::GenerationsService;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use anyhow;

// Custom deserializer to handle null values in JSON
fn null_to_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoGenerationRequest {
    pub prompt: String,
    pub duration_seconds: Option<u32>,
    pub style: Option<String>,
    pub model: Option<String>,
    pub image: Option<serde_json::Value>,
    pub aspect_ratio: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoGenerationResponse {
    pub operation_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoStatusResponse {
    pub status: String,
    pub progress: Option<f32>,
    pub video_path: Option<String>,
    pub json_prompt: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedPrompt {
    #[serde(deserialize_with = "null_to_default")]
    pub model: String,
    pub output: VideoOutput,
    pub prompt: VideoPrompt,
    pub audio: VideoAudio,
    #[serde(deserialize_with = "null_to_default")]
    pub negative_prompt: String,
    pub parameters: VideoParameters,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoOutput {
    #[serde(deserialize_with = "null_to_default")]
    pub format: String,
    #[serde(deserialize_with = "null_to_default")]
    pub resolution: String,
    pub fps: u32,
    pub duration_seconds: u32,
    #[serde(deserialize_with = "null_to_default")]
    pub bitrate: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoPrompt {
    #[serde(deserialize_with = "null_to_default")]
    pub text: String,
    #[serde(deserialize_with = "null_to_default")]
    pub style: String,
    #[serde(deserialize_with = "null_to_default")]
    pub mood: String,
    pub camera: CameraSettings,
    pub aesthetic: AestheticSettings,
    pub characters: Option<Vec<Character>>,
    pub product: Option<Product>,
    pub branding: Option<Branding>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CameraSettings {
    #[serde(deserialize_with = "null_to_default")]
    pub initial: String,
    pub transitions: Vec<String>,
    #[serde(deserialize_with = "null_to_default")]
    pub movement: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AestheticSettings {
    #[serde(deserialize_with = "null_to_default")]
    pub lighting: String,
    #[serde(deserialize_with = "null_to_default")]
    pub color_grade: String,
    #[serde(deserialize_with = "null_to_default")]
    pub aspect_ratio: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Character {
    #[serde(default = "default_character_name", deserialize_with = "null_to_default")]
    pub name: String,
    #[serde(default = "default_character_gender", deserialize_with = "null_to_default")]
    pub gender: String,
    #[serde(default = "default_character_age", deserialize_with = "null_to_default")]
    pub age: String,
    #[serde(default = "default_character_appearance", deserialize_with = "null_to_default")]
    pub appearance: String,
}

fn default_character_name() -> String { "Character".to_string() }
fn default_character_gender() -> String { "unspecified".to_string() }
fn default_character_age() -> String { "adult".to_string() }
fn default_character_appearance() -> String { "person".to_string() }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Product {
    #[serde(default = "default_product_name", deserialize_with = "null_to_default")]
    pub name: String,
    #[serde(default = "default_product_color", deserialize_with = "null_to_default")]
    pub color: String,
    #[serde(default = "default_product_features")]
    pub features: Vec<String>,
}

fn default_product_name() -> String { "Product".to_string() }
fn default_product_color() -> String { "default".to_string() }
fn default_product_features() -> Vec<String> { vec![] }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Branding {
    #[serde(default = "default_branding_logo_placement", deserialize_with = "null_to_default")]
    pub logo_placement: String,
    #[serde(default = "default_branding_tagline", deserialize_with = "null_to_default")]
    pub tagline: String,
}

fn default_branding_logo_placement() -> String { "bottom-right".to_string() }
fn default_branding_tagline() -> String { "".to_string() }

fn default_voiceover_gender() -> String { "female".to_string() }
fn default_voiceover_language() -> String { "en-US".to_string() }
fn default_voiceover_lines() -> Vec<String> { vec![] }

fn default_music_genre() -> String { "ambient".to_string() }
fn default_music_bpm() -> u32 { 90 }
fn default_music_mood() -> String { "neutral".to_string() }

fn default_voiceover_settings() -> VoiceoverSettings {
    VoiceoverSettings {
        gender: "female".to_string(),
        language: "en-US".to_string(),
        lines: vec![],
    }
}

fn default_music_settings() -> MusicSettings {
    MusicSettings {
        genre: "ambient".to_string(),
        bpm: 90,
        mood: "neutral".to_string(),
    }
}

fn default_sound_effects() -> Vec<String> { vec![] }

impl Default for VoiceoverSettings {
    fn default() -> Self {
        default_voiceover_settings()
    }
}

impl Default for MusicSettings {
    fn default() -> Self {
        default_music_settings()
    }
}

// Custom deserializer that handles null values by converting them to defaults
fn deserialize_with_null_as_default<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Default,
{
    let opt = Option::<T>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoAudio {
    #[serde(deserialize_with = "deserialize_with_null_as_default", default)]
    pub voiceover: VoiceoverSettings,
    #[serde(deserialize_with = "deserialize_with_null_as_default", default)]
    pub music: MusicSettings,
    #[serde(default = "default_sound_effects")]
    pub sound_effects: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceoverSettings {
    #[serde(default = "default_voiceover_gender", deserialize_with = "null_to_default")]
    pub gender: String,
    #[serde(default = "default_voiceover_language", deserialize_with = "null_to_default")]
    pub language: String,
    #[serde(default = "default_voiceover_lines")]
    pub lines: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MusicSettings {
    #[serde(default = "default_music_genre", deserialize_with = "null_to_default")]
    pub genre: String,
    #[serde(default = "default_music_bpm")]
    pub bpm: u32,
    #[serde(default = "default_music_mood", deserialize_with = "null_to_default")]
    pub mood: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoParameters {
    pub temperature: f32,
    pub cfg_scale: u32,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoGenerationStatus {
    pub operation_id: String,
    pub status: String,
    pub progress: Option<f32>,
    pub video_path: Option<String>,
    pub json_prompt: Option<String>,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

fn categorize_google_api_error(
    status: reqwest::StatusCode,
    message: &str,
    context: &str,
) -> String {
    match status.as_u16() {
        400 => format!(
            "{} failed: invalid request. Check request parameters. Details: {}",
            context, message
        ),
        401 => "Google API key is invalid. Update your Google Gemini API key in App Store.".to_string(),
        403 => "Google API access denied. Enable Generative Language API and verify project permissions in Google Cloud.".to_string(),
        404 => format!("{} endpoint/model not found. Verify selected model is available.", context),
        429 => "Google API rate limit exceeded. Wait and retry, or increase quota.".to_string(),
        500..=599 => format!("Google API service error ({}). Please retry shortly.", status),
        _ => format!("{} failed ({}): {}", context, status, message),
    }
}

async fn google_api_error_from_response(
    response: reqwest::Response,
    context: &str,
) -> String {
    let status = response.status();
    let error_data: serde_json::Value = response
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({"error": {"message": "Unknown error"}}));
    let message = error_data["error"]["message"]
        .as_str()
        .unwrap_or("Unknown error");
    categorize_google_api_error(status, message, context)
}

// Get Veo3 API key from installed apps
async fn get_veo3_api_key(app_state: &AppState) -> Result<String, String> {
    let app_service = AppInstallationService::new(
        app_state.sqlite_service.get_database_service(),
    );

    // Get all installed apps and find Veo3 app in one blocking operation
    let veo3_app = tokio::task::spawn_blocking(move || {
        let installed_apps = app_service.get_installed_apps()?;

        // Find Veo3 app
        let veo3_app = installed_apps.iter()
            .find(|app| app.app_name == "Google Gemini")
            .ok_or_else(|| anyhow::anyhow!("Veo3 is not installed. Please install Veo3 first."))?;

        // Check if Veo3 has an API key
        if !veo3_app.has_api_key {
            return Err(anyhow::anyhow!("Veo3 is installed but no API key is configured. Please configure the API key in the App Store."));
        }

        // Get the decrypted API key
        let api_key = app_service.get_api_key(veo3_app.id)?
            .ok_or_else(|| anyhow::anyhow!("Veo3 API key not found in database."))?;

        if api_key.is_empty() {
            return Err(anyhow::anyhow!("Veo3 API key is empty. Please configure a valid API key in the App Store."));
        }

        Ok(api_key)
    }).await
        .map_err(|e| format!("Failed to spawn blocking task: {}", e))?
        .map_err(|e| format!("Failed to get Veo3 API key: {}", e))?;

    Ok(veo3_app)
}

async fn delay(ms: u64) {
    tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
}

async fn generate_detailed_prompt(simple_prompt: &str, duration_seconds: Option<u32>, style: Option<String>, model: Option<String>, api_key: &str, aspect_ratio: Option<String>) -> Result<DetailedPrompt, String> {
    println!("🤖 Generating detailed prompt from: {}", simple_prompt);

    if api_key.is_empty() {
        return Err("Gemini API key not configured".to_string());
    }

    let duration = duration_seconds.unwrap_or(8);
    let style = style.as_deref().unwrap_or("cinematic");
    let model = model.as_deref().unwrap_or("veo-3.1-fast-generate-preview");

    let prompt_text = format!(
      r#"Create a detailed JSON prompt for video generation based on this simple description: "{}"

      IMPORTANT REQUIREMENTS:
      - Duration must be exactly {} seconds
      - Style must be "{}"
      - Make the video description match the specified style

      The JSON should follow this exact structure with realistic, detailed values inferred from the prompt:

      {{
        "model": "{}",
        "output": {{
          "format": "mp4",
          "resolution": "1920x1080",
          "fps": 24,
          "duration_seconds": {},
          "bitrate": "12M"
        }},
        "prompt": {{
          "text": "A detailed {} description of the video scene that matches the style",
          "style": "{}",
          "mood": "appropriate mood for the content",
          "camera": {{
            "initial": "opening camera movement",
            "transitions": ["list of camera transitions"],
            "movement": "overall camera movement style"
          }},
          "aesthetic": {{
            "lighting": "lighting description",
            "color_grade": "color grading style",
            "aspect_ratio": "{}"
          }},
          "characters": [
            {{
              "name": "Character name",
              "gender": "gender",
              "age": "age description",
              "appearance": "detailed appearance"
            }}
          ],
          "product": {{
            "name": "Product name",
            "color": "product color",
            "features": ["list of product features"]
          }},
          "branding": {{
            "logo_placement": "where logo appears",
            "tagline": "brand tagline"
          }}
        }},
        "audio": {{
          "voiceover": {{
            "gender": "voiceover gender",
            "language": "en-US",
            "lines": ["voiceover lines"]
          }},
          "music": {{
            "genre": "music genre",
            "bpm": 90,
            "mood": "music mood"
          }},
          "sound_effects": ["list of sound effects"]
        }},
        "negative_prompt": "what to avoid in the video",
        "parameters": {{
          "temperature": 0.7,
          "cfg_scale": 12,
          "seed": 4294967295
        }}
      }}

      Return only the JSON object, no additional text or explanation."#,
              simple_prompt, duration, style, model, duration, style, style, aspect_ratio.unwrap_or_else(|| "16:9".to_string())
    );

    let client = reqwest::Client::new();

    let response = client
        .post(&format!("{}/models/gemini-2.5-flash:generateContent", BASE_URL))
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": prompt_text
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(google_api_error_from_response(response, "Gemini prompt generation").await);
    }

    let data: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let generated_text = data["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or("No response text from Gemini API")?;

    // Extract JSON from the response (in case there's extra text)
    if let Some(json_match) = generated_text.find('{') {
        let json_text = &generated_text[json_match..];
        if let Some(end_brace) = json_text.rfind('}') {
            let json_str = &json_text[..=end_brace];
            return serde_json::from_str(json_str)
                .map_err(|e| format!("Failed to parse JSON: {}", e));
        }
    }

    serde_json::from_str(generated_text)
        .map_err(|e| format!("Failed to parse JSON response: {}", e))
}

async fn generate_video(
    prompt_data: &DetailedPrompt,
    output_file: &str,
    api_key: &str,
    model: &str,
    image: Option<serde_json::Value>,
    aspect_ratio: Option<String>,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("Gemini API key not configured".to_string());
    }

    let ar_from_prompt = if prompt_data.prompt.aesthetic.aspect_ratio.trim().is_empty() {
        None
    } else {
        Some(prompt_data.prompt.aesthetic.aspect_ratio.clone())
    };
    let is_veo2 = model.starts_with("veo-2");
    let ar_to_use = if is_veo2 { aspect_ratio.clone().or(ar_from_prompt) } else { None };
    if let Some(ref ar) = ar_to_use { println!("📝 Aspect ratio (included): {}", ar); } else { println!("📝 Aspect ratio omitted for this model"); }

    let client = reqwest::Client::new();

    // Build request instance with optional fields
    let mut instance = serde_json::Map::new();
    instance.insert(
        "prompt".to_string(),
        serde_json::Value::String(serde_json::to_string(prompt_data).unwrap()),
    );
    if let Some(img) = image.clone() { instance.insert("image".to_string(), img); }
    if let Some(ar) = ar_to_use { instance.insert("aspectRatio".to_string(), serde_json::Value::String(ar)); }

    let request_body = serde_json::json!({
        "instances": [serde_json::Value::Object(instance)]
    });

    // Send request to generate video and capture the operation name
    let response = client
        .post(&format!("{}/models/{}:predictLongRunning", BASE_URL, model))
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(google_api_error_from_response(response, "Video generation request").await);
    }

    let data: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let operation_name = data["name"]
        .as_str()
        .ok_or("Failed to get operation name from response")?;

    // Poll the operation status until the video is ready
    loop {
        let status_response = client
            .get(&format!("{}/{}", BASE_URL, operation_name))
            .header("x-goog-api-key", api_key)
            .send()
            .await
            .map_err(|e| format!("Status check failed: {}", e))?;

        if !status_response.status().is_success() {
            return Err(google_api_error_from_response(status_response, "Video generation status check").await);
        }

        let status_data: serde_json::Value = status_response.json().await
            .map_err(|e| format!("Failed to parse status response: {}", e))?;

        // Check if there was an error in the operation
        if let Some(error) = status_data.get("error") {
            return Err(format!("Operation failed: {}",
                error["message"].as_str().unwrap_or("Unknown error")));
        }

        // Check the "done" field
        if let Some(done) = status_data.get("done") {
            if done.as_bool().unwrap_or(false) {
                // Extract the download URI from the final response
                let video_uri = status_data["response"]["generateVideoResponse"]["generatedSamples"][0]["video"]["uri"]
                    .as_str()
                    .ok_or("Failed to extract video URI from response")?;

                // Download the video using the URI and API key
                let download_response = client
                    .get(video_uri)
                    .header("x-goog-api-key", api_key)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to download video: {}", e))?;

                if !download_response.status().is_success() {
                    return Err("Failed to download video".to_string());
                }

                // Save the video to file
                let video_bytes = download_response.bytes().await
                    .map_err(|e| format!("Failed to read video bytes: {}", e))?;

                let mut file = fs::File::create(output_file)
                    .map_err(|e| format!("Failed to create output file: {}", e))?;
                file.write_all(&video_bytes)
                    .map_err(|e| format!("Failed to write video file: {}", e))?;

                return Ok(output_file.to_string());
            }
        }

        delay(10000).await;
    }
}

#[tauri::command]
pub async fn generate_video_prompt(
    request: VideoGenerationRequest,
    app_state: State<'_, AppState>,
) -> Result<VideoGenerationResponse, String> {

    // Get Veo3 API key from installed apps
    let api_key = tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        get_veo3_api_key(&app_state)
    ).await
        .map_err(|_| "Timeout getting Veo3 API key".to_string())??;

    // Generate detailed prompt (include selected model so it is embedded in the JSON)
    let detailed_prompt = generate_detailed_prompt(&request.prompt, request.duration_seconds, request.style, request.model.clone(), &api_key, request.aspect_ratio.clone()).await?;

    // Create generation record in database
    let generations_service = GenerationsService::new(app_state.sqlite_service.get_database_service());
    let generation_id = generations_service.create_generation(
        &request.prompt,
        &serde_json::to_string_pretty(&detailed_prompt).unwrap(),
        "veo3"
    ).map_err(|e| format!("Failed to create generation record: {}", e))?;
    // Get desktop path
    let desktop_path = dirs::desktop_dir()
        .ok_or("Could not find desktop directory")?
        .join("cosmos_videos");

    // Create videos directory if it doesn't exist
    fs::create_dir_all(&desktop_path)
        .map_err(|e| format!("Failed to create videos directory: {}", e))?;

    // Generate unique filename
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("cosmos_video_{}.mp4", timestamp);
    let output_path = desktop_path.join(&filename);

    // Store the operation details in app state for status tracking
    let operation_id = format!("video_gen_{}", timestamp);

    // Initialize status in app state
    let initial_status = VideoGenerationStatus {
        operation_id: operation_id.clone(),
        status: "generating".to_string(),
        progress: Some(0.0),
        video_path: None,
        json_prompt: Some(serde_json::to_string_pretty(&detailed_prompt).unwrap()),
        error: None,
        created_at: chrono::Utc::now(),
    };

    {
        let mut status_map = app_state.video_generation_status.lock().await;
        status_map.insert(operation_id.clone(), initial_status);
    }

    // Start video generation in background
    let detailed_prompt_clone = detailed_prompt.clone();
    let output_path_clone = output_path.clone();
    let operation_id_clone = operation_id.clone();
    let video_generation_status = app_state.video_generation_status.clone();
    let api_key_clone = api_key.clone();
    let generation_id_clone = generation_id.clone();
    let generations_service_clone = generations_service;
    let selected_model = request.model.unwrap_or_else(|| "veo-3.1-fast-generate-preview".to_string());
    let selected_model_clone = selected_model.clone();
    let image_opt: Option<serde_json::Value> = request.image.clone();
    let aspect_ratio_opt: Option<String> = request.aspect_ratio.clone();

    tokio::spawn(async move {
        match generate_video(
            &detailed_prompt_clone,
            output_path_clone.to_str().unwrap(),
            &api_key_clone,
            &selected_model_clone,
            image_opt,
            aspect_ratio_opt,
        ).await {
            Ok(video_path) => {
                // Update generation record with file path
                if let Err(e) = generations_service_clone.update_generation_file_path(&generation_id_clone, &video_path) {
                    println!("⚠️ Failed to update generation record: {}", e);
                }

                // Update status in app state
                let mut status_map = video_generation_status.lock().await;
                if let Some(status) = status_map.get_mut(&operation_id_clone) {
                    status.status = "completed".to_string();
                    status.progress = Some(1.0);
                    status.video_path = Some(video_path.clone());
                }
            }
            Err(e) => {
                println!("❌ Video generation failed: {}", e);
                // Update status in app state
                let mut status_map = video_generation_status.lock().await;
                if let Some(status) = status_map.get_mut(&operation_id_clone) {
                    status.status = "failed".to_string();
                    status.error = Some(e);
                }
            }
        }
    });

    Ok(VideoGenerationResponse {
        operation_id,
        status: "started".to_string(),
        message: "Video generation started".to_string(),
    })
}

#[tauri::command]
pub async fn get_video_generation_status(
    operation_id: String,
    app_state: State<'_, AppState>,
) -> Result<VideoStatusResponse, String> {
    let status_map = app_state.video_generation_status.lock().await;

    if let Some(status) = status_map.get(&operation_id) {
        Ok(VideoStatusResponse {
            status: status.status.clone(),
            progress: status.progress,
            video_path: status.video_path.clone(),
            json_prompt: status.json_prompt.clone(),
            error: status.error.clone(),
        })
    } else {
        Err("Operation not found".to_string())
    }
}

#[tauri::command]
pub async fn get_generated_json_prompt(
    operation_id: String,
    app_state: State<'_, AppState>,
) -> Result<String, String> {
    let status_map = app_state.video_generation_status.lock().await;

    if let Some(status) = status_map.get(&operation_id) {
        if let Some(json_prompt) = &status.json_prompt {
            Ok(json_prompt.clone())
        } else {
            Err("JSON prompt not found for this operation".to_string())
        }
    } else {
        Err("Operation not found".to_string())
    }
}

#[tauri::command]
pub async fn get_all_generations(
    app_state: State<'_, AppState>,
) -> Result<Vec<crate::services::generations_service::Generation>, String> {
    let generations_service = GenerationsService::new(app_state.sqlite_service.get_database_service());

    generations_service.get_all_generations()
        .map_err(|e| format!("Failed to get generations: {}", e))
}

#[tauri::command]
pub async fn get_generation_by_id(
    generation_id: String,
    app_state: State<'_, AppState>,
) -> Result<Option<crate::services::generations_service::Generation>, String> {
    let generations_service = GenerationsService::new(app_state.sqlite_service.get_database_service());

    generations_service.get_generation_by_id(&generation_id)
        .map_err(|e| format!("Failed to get generation: {}", e))
}

use crate::ffmpeg_thumbnail::get_bundled_ffmpeg_path;
use std::process::Command as ProcessCommand;
use uuid::Uuid;
use dirs;

fn get_cosmos_videos_dir() -> Result<PathBuf, String> {
    let desktop_dir = dirs::desktop_dir()
        .ok_or("Could not find Desktop directory")?;

    let cosmos_videos_dir = desktop_dir.join("cosmos_videos");

    // Create directory if it doesn't exist
    if !cosmos_videos_dir.exists() {
        std::fs::create_dir_all(&cosmos_videos_dir)
            .map_err(|e| format!("Failed to create cosmos_videos directory: {}", e))?;
    }

    Ok(cosmos_videos_dir)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoEditRequest {
    pub input_path: String,
    pub output_path: Option<String>,
    // Trim options
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub preserve_timecodes: Option<bool>,
    // Resize options
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub maintain_aspect_ratio: Option<bool>,
    // Aspect ratio options
    pub aspect_ratio: Option<String>, // "16:9", "1:1", "9:16", etc.
    pub aspect_mode: Option<String>,  // "crop" or "pad"
    // Crop positioning (-1.0 to 1.0, where 0.0 is center)
    pub crop_x: Option<f64>,
    pub crop_y: Option<f64>,
    // Source video dimensions (provided by frontend)
    pub source_width: Option<u32>,
    pub source_height: Option<u32>,
    // Pre-calculated crop dimensions (provided by frontend)
    pub crop_width: Option<u32>,
    pub crop_height: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrimVideoRequest {
    pub input_path: String,
    pub output_path: Option<String>,
    pub start_time: f64,
    pub end_time: f64,
    pub preserve_timecodes: Option<bool>,
}

#[tauri::command]
pub async fn edit_video(request: VideoEditRequest) -> Result<String, String> {
    // Validate input file exists
    if !Path::new(&request.input_path).exists() {
        return Err(format!("Input file not found: {}", request.input_path));
    }

    // Get the ffmpeg path
    let ffmpeg_path = get_bundled_ffmpeg_path();
    if !ffmpeg_path.exists() {
        return Err(format!("FFmpeg not found at: {}", ffmpeg_path.display()));
    }

    // Generate output path in cosmos_videos directory
    let output_path = if let Some(path) = request.output_path {
        PathBuf::from(path)
    } else {
        let cosmos_dir = get_cosmos_videos_dir()?;
        let input_path = Path::new(&request.input_path);
        let stem = input_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let extension = input_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");

        let mut suffix = String::new();
        if request.start_time.is_some() || request.end_time.is_some() {
            suffix.push_str("_trimmed");
        }
        if request.width.is_some() || request.height.is_some() || request.aspect_ratio.is_some() {
            suffix.push_str("_resized");
        }

        cosmos_dir.join(format!("{}{}_{}.{}", stem, suffix,
            Uuid::new_v4().to_string().chars().take(8).collect::<String>(),
            extension))
    };

    // Build ffmpeg command
    let mut cmd = ProcessCommand::new(&ffmpeg_path);

    // Input seeking (if trimming)
    if let Some(start_time) = request.start_time {
        cmd.arg("-ss").arg(start_time.to_string());
    }

    cmd.arg("-i").arg(&request.input_path);

    // Duration (if trimming)
    if let (Some(start), Some(end)) = (request.start_time, request.end_time) {
        let duration = end - start;
        cmd.arg("-t").arg(duration.to_string());
    }

    // Build video filter chain
    let mut filters = Vec::new();

    // Handle resize
    if let (Some(width), Some(height)) = (request.width, request.height) {
        if request.maintain_aspect_ratio.unwrap_or(true) {
            // Scale maintaining aspect ratio (fit within bounds)
            filters.push(format!("scale={}:{}:force_original_aspect_ratio=decrease", width, height));
        } else {
            // Scale to exact dimensions
            filters.push(format!("scale={}:{}", width, height));
        }
    }

    // Handle aspect ratio change
    if let Some(aspect_ratio) = &request.aspect_ratio {
        let (target_w, target_h) = match aspect_ratio.as_str() {
            "16:9" => (16.0, 9.0),
            "4:3" => (4.0, 3.0),
            "1:1" => (1.0, 1.0),
            "9:16" => (9.0, 16.0),
            "21:9" => (21.0, 9.0),
            _ => (16.0, 9.0), // Default to 16:9
        };

        let mode = request.aspect_mode.as_deref().unwrap_or("crop");

        if mode == "crop" {
            // Use pre-calculated dimensions from frontend if available
            if let (Some(crop_w), Some(crop_h)) = (request.crop_width, request.crop_height) {
                let crop_x = request.crop_x.unwrap_or(0.0);
                let crop_y = request.crop_y.unwrap_or(0.0);

                // Get source dimensions for offset calculation
                let source_w = request.source_width.unwrap_or(1920);
                let source_h = request.source_height.unwrap_or(1080);

                // Calculate pixel offsets based on crop position
                // crop_x/crop_y range from -1 to 1, where 0 is center
                let max_x_offset = (source_w as f64 - crop_w as f64) / 2.0;
                let max_y_offset = (source_h as f64 - crop_h as f64) / 2.0;

                let x_offset = max_x_offset * (1.0 + crop_x);
                let y_offset = max_y_offset * (1.0 - crop_y); // Invert Y for intuitive direction

                let crop_filter = format!("crop={}:{}:{}:{}",
                    crop_w, crop_h,
                    x_offset.round() as i32,
                    y_offset.round() as i32
                );
                eprintln!("DEBUG: Simple crop filter: {}", crop_filter);
                filters.push(crop_filter);
            } else {
                // Fallback to aspect ratio calculation if dimensions not provided
                let ratio = target_w / target_h;
                let crop_filter = format!("crop=ih*{}:ih", ratio);
                filters.push(crop_filter);
            }
        } else {
            // Pad to aspect ratio (black bars)
            filters.push(format!("pad=ih*{}:ih:(ow-iw)/2:(oh-ih)/2:black", target_w / target_h));
        }
    }

    // Apply video filters if any
    if !filters.is_empty() {
        cmd.arg("-vf").arg(filters.join(","));
        // Use libx264 for compatibility when filtering
        cmd.arg("-c:v").arg("libx264");
        cmd.arg("-preset").arg("fast");
        cmd.arg("-crf").arg("23");
    } else if request.start_time.is_some() || request.end_time.is_some() {
        // Just trimming, use stream copy
        cmd.arg("-c").arg("copy");
    }

    // Audio codec
    if !filters.is_empty() {
        cmd.arg("-c:a").arg("aac");
    } else {
        cmd.arg("-c:a").arg("copy");
    }

    // Preserve timecodes if requested
    if request.preserve_timecodes.unwrap_or(false) {
        cmd.arg("-copyts");
    }

    // Avoid negative timestamps
    cmd.arg("-avoid_negative_ts").arg("make_zero");

    // Output file
    cmd.arg(&output_path);
    cmd.arg("-y"); // Overwrite if exists

    // Execute ffmpeg
    let output = cmd.output()
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg failed: {}", stderr));
    }

    // Verify output file was created
    if !output_path.exists() {
        return Err("Output file was not created".to_string());
    }

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn trim_video(request: TrimVideoRequest) -> Result<String, String> {
    // Validate input file exists
    if !Path::new(&request.input_path).exists() {
        return Err(format!("Input file not found: {}", request.input_path));
    }

    // Get the ffmpeg path
    let ffmpeg_path = get_bundled_ffmpeg_path();
    if !ffmpeg_path.exists() {
        return Err(format!("FFmpeg not found at: {}", ffmpeg_path.display()));
    }

    // Generate output path in cosmos_videos directory
    let output_path = if let Some(path) = request.output_path {
        PathBuf::from(path)
    } else {
        let cosmos_dir = get_cosmos_videos_dir()?;
        let input_path = Path::new(&request.input_path);
        let stem = input_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let extension = input_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4");

        cosmos_dir.join(format!("{}_trimmed_{}.{}", stem, Uuid::new_v4().to_string().chars().take(8).collect::<String>(), extension))
    };

    // Build ffmpeg command
    let mut cmd = ProcessCommand::new(&ffmpeg_path);

    // Input file and seek to start
    cmd.arg("-ss").arg(request.start_time.to_string());
    cmd.arg("-i").arg(&request.input_path);

    // Set duration (end_time - start_time)
    let duration = request.end_time - request.start_time;
    cmd.arg("-t").arg(duration.to_string());

    // Copy streams without re-encoding for speed
    cmd.arg("-c").arg("copy");

    // Preserve timecodes if requested
    if request.preserve_timecodes.unwrap_or(false) {
        cmd.arg("-copyts");
    }

    // Avoid re-encoding by using keyframe seeking
    cmd.arg("-avoid_negative_ts").arg("make_zero");

    // Output file
    cmd.arg(&output_path);
    cmd.arg("-y"); // Overwrite output file if exists

    // Execute ffmpeg
    let output = cmd.output()
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg failed: {}", stderr));
    }

    // Verify output file was created
    if !output_path.exists() {
        return Err("Output file was not created".to_string());
    }

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn delete_generation(
    generation_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let generations_service = GenerationsService::new(app_state.sqlite_service.get_database_service());

    generations_service.delete_generation(&generation_id)
        .map_err(|e| format!("Failed to delete generation: {}", e))
}

#[tauri::command]
pub async fn get_generation_stats(
    app_state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let generations_service = GenerationsService::new(app_state.sqlite_service.get_database_service());

    generations_service.get_generation_stats()
        .map_err(|e| format!("Failed to get generation stats: {}", e))
}

/// Send a video file to Studio by copying it to the cosmos_videos directory
#[tauri::command]
pub async fn send_video_to_studio(video_path: String) -> Result<String, String> {
    use std::path::Path;
    use tokio::fs;

    // Validate input path
    let source_path = Path::new(&video_path);
    if !source_path.exists() {
        return Err("Video file does not exist".to_string());
    }

    if !source_path.is_file() {
        return Err("Path is not a file".to_string());
    }

    // Get the file name
    let file_name = source_path
        .file_name()
        .ok_or("Could not get file name")?
        .to_string_lossy()
        .to_string();

    // Validate it's a video file
    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();

    if !["mp4", "mov", "avi", "mkv", "webm"].contains(&extension.as_str()) {
        return Err("File is not a supported video format".to_string());
    }

    // Get desktop path and create cosmos_videos directory
    let desktop_path = dirs::desktop_dir()
        .ok_or("Could not find desktop directory")?;
    let cosmos_videos_path = desktop_path.join("cosmos_videos");

    // Create directory if it doesn't exist
    fs::create_dir_all(&cosmos_videos_path)
        .await
        .map_err(|e| format!("Failed to create cosmos_videos directory: {}", e))?;

    // Handle potential filename conflicts
    let mut destination_path = cosmos_videos_path.join(&file_name);
    let mut counter = 1;

    while destination_path.exists() {
        let stem = source_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("video");
        let new_name = format!("{}_{}.{}", stem, counter, extension);
        destination_path = cosmos_videos_path.join(new_name);
        counter += 1;
    }

    // Copy the file
    fs::copy(&source_path, &destination_path)
        .await
        .map_err(|e| format!("Failed to copy video file: {}", e))?;

    let final_path = destination_path.to_string_lossy().to_string();
    Ok(format!("Video successfully sent to Studio: {}", final_path))
}

/// Check if a video file already exists in Studio (cosmos_videos directory)
#[tauri::command]
pub async fn is_video_in_studio(video_path: String) -> Result<bool, String> {
    use std::path::Path;

    // Get the file name from the source path
    let source_path = Path::new(&video_path);
    let file_name = source_path
        .file_name()
        .ok_or("Could not get file name")?
        .to_string_lossy()
        .to_string();

    // Get desktop path and cosmos_videos directory
    let desktop_path = dirs::desktop_dir()
        .ok_or("Could not find desktop directory")?;
    let cosmos_videos_path = desktop_path.join("cosmos_videos");

    // Check if the file exists in cosmos_videos directory
    let destination_path = cosmos_videos_path.join(&file_name);
    Ok(destination_path.exists())
}
