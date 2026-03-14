use crate::models::whisper::WhisperModel;
use crate::services::download_service::{DownloadService, WhisperStatus};
use crate::{app_log_debug, app_log_info};
use anyhow::Result;
use std::path::Path;

/// Transcription result with basic info
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub segments: Vec<TranscriptionSegment>,
    pub duration: f64,
    pub language: Option<String>,
}

/// Individual transcription segment
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub confidence: Option<f32>,
}

/// Audio service with candle-rs Whisper integration
pub struct AudioService {
    whisper_model: Option<WhisperModel>,
}

impl AudioService {
    /// Create a new audio service
    pub fn new() -> Self {
        app_log_info!("🎵 Initializing AudioService with candle-rs...");
        Self {
            whisper_model: None,
        }
    }

    /// Check if audio transcription is available
    pub fn is_available(&self) -> bool {
        matches!(DownloadService::get_whisper_status(), WhisperStatus::Ready)
            && self.whisper_model.is_some()
    }

    /// Load Whisper model
    pub async fn load_model(&mut self) -> Result<()> {
        // Create and load WhisperModel
        let mut whisper_model = WhisperModel::new()?;
        whisper_model.load_model()?;
        self.whisper_model = Some(whisper_model);
        app_log_info!("✅ Whisper model loaded, ready for transcription");
        Ok(())
    }

    /// Validate audio file
    pub fn validate_audio_file(&self, file_path: &Path) -> Result<()> {
        if !file_path.exists() {
            return Err(anyhow::anyhow!("Audio file not found: {:?}", file_path));
        }

        // Check file extension
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("wav") | Some("mp3") | Some("mp4") | Some("m4a") | Some("flac") | Some("ogg")
            | Some("mov") | Some("avi") | Some("mkv") | Some("webm") => {
                app_log_debug!("✅ Supported audio/video format: {:?}", extension);
                Ok(())
            }
            Some(ext) => Err(anyhow::anyhow!("Unsupported audio/video format: {}", ext)),
            None => Err(anyhow::anyhow!("Could not determine audio/video format")),
        }
    }

    /// Transcribe audio file using Whisper
    pub async fn transcribe_file(&mut self, file_path: &Path) -> Result<TranscriptionResult> {
        app_log_info!("🎤 Transcribing: {:?}", file_path);
        app_log_debug!(
            "🔍 File path absolute: {:?}",
            file_path
                .canonicalize()
                .unwrap_or_else(|_| file_path.to_path_buf())
        );

        // Validate the file first
        self.validate_audio_file(file_path)?;

        // Ensure model is loaded
        if self.whisper_model.is_none() {
            app_log_info!("🔧 Model not loaded, loading now...");
            self.load_model().await?;
        }

        // Load and process audio file first
        let audio_data = self.load_audio_data(file_path)?;
        let whisper_model = self
            .whisper_model
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Whisper model not available"))?;
        // Use WhisperModel to transcribe
        let whisper_result = whisper_model.transcribe_audio(&audio_data)?;
        // Convert to our format
        let result = TranscriptionResult {
            text: whisper_result.text,
            segments: whisper_result
                .segments
                .into_iter()
                .map(|seg| TranscriptionSegment {
                    start: seg.start,
                    end: seg.end,
                    text: seg.text,
                    confidence: seg.confidence,
                })
                .collect(),
            duration: whisper_result.duration,
            language: whisper_result.language,
        };

        app_log_info!(
            "✅ Transcription completed: {} characters",
            result.text.len()
        );
        Ok(result)
    }

    /// Load audio data from file (supports multiple formats via Symphonia)
    fn load_audio_data(&self, file_path: &Path) -> Result<Vec<f32>> {
        app_log_debug!("🔧 Loading audio data from: {:?}", file_path);

        use std::fs::File;
        use symphonia::core::audio::SampleBuffer;
        use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
        use symphonia::core::errors::Error as SymphoniaError;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        // Open the file
        let file = File::open(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to open audio file: {}", e))?;
        // Create media source stream
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a probe hint using the file extension
        let mut hint = Hint::new();
        if let Some(extension) = file_path.extension() {
            if let Some(extension_str) = extension.to_str() {
                hint.with_extension(extension_str);
            }
        }

        // Use the default options
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        // Probe the media source
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|e| anyhow::anyhow!("Failed to probe audio format: {}", e))?;

        // Get the instantiated format reader
        let mut format = probed.format;

        // Find the first audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow::anyhow!("No audio track found"))?;

        // Store track ID and codec params for use in decoding loop
        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        // Create a decoder for the track
        let dec_opts: DecoderOptions = Default::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &dec_opts)
            .map_err(|e| anyhow::anyhow!("Failed to create decoder: {}", e))?;

        // Store decoded audio samples
        let mut audio_buf = None;
        let mut samples = Vec::new();
        let mut audio_spec = None;

        // Decode all packets
        loop {
            // Get the next packet from the media format
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::ResetRequired) => {
                    // The track list has been changed. Re-examine it and create a new set of decoders
                    break;
                }
                Err(SymphoniaError::IoError(err)) => {
                    if err.kind() == std::io::ErrorKind::UnexpectedEof {
                        break;
                    }
                    return Err(anyhow::anyhow!("IO error while reading: {}", err));
                }
                Err(err) => {
                    return Err(anyhow::anyhow!("Decode error: {}", err));
                }
            };

            // If the packet does not belong to the selected track, skip over it
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // If the audio buffer is not allocated yet, allocate it
                    if audio_buf.is_none() {
                        // Get the audio buffer specification
                        let spec = *decoded.spec();
                        audio_spec = Some(spec);

                        // Create the audio buffer
                        let duration = decoded.capacity() as u64;
                        audio_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                    }

                    // Copy the audio buffer
                    if let Some(buf) = &mut audio_buf {
                        buf.copy_interleaved_ref(decoded);
                        samples.extend_from_slice(buf.samples());
                    }
                }
                Err(SymphoniaError::IoError(_)) => {
                    // The packet failed to decode due to an IO error, skip the packet
                    continue;
                }
                Err(SymphoniaError::DecodeError(_)) => {
                    // The packet failed to decode due to invalid data, skip the packet
                    continue;
                }
                Err(err) => {
                    return Err(anyhow::anyhow!("Decode error: {}", err));
                }
            }
        }

        if samples.is_empty() {
            return Err(anyhow::anyhow!("No audio samples decoded"));
        }

        // Get audio specification for resampling/conversion
        let spec = audio_spec.ok_or_else(|| anyhow::anyhow!("No audio specification available"))?;
        app_log_debug!(
            "🔍 Audio spec: channels={}, sample_rate={}",
            spec.channels.count(),
            spec.rate
        );

        // Convert to mono if needed
        let mut audio_data = if spec.channels.count() > 1 {
            app_log_debug!("🔧 Converting {} channels to mono", spec.channels.count());
            let channels = spec.channels.count();
            samples
                .chunks_exact(channels)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect()
        } else {
            samples
        };

        // Normalize audio to [-1, 1] range
        let max_val = audio_data
            .iter()
            .fold(0.0f32, |max, &sample| max.max(sample.abs()));
        if max_val > 0.0 {
            app_log_debug!("🔧 Normalizing audio, max value: {}", max_val);
            for sample in audio_data.iter_mut() {
                *sample /= max_val;
            }
        }

        // Resample to 16kHz if needed
        if spec.rate != 16000 {
            app_log_debug!("🔧 Resampling from {}Hz to 16kHz", spec.rate);
            let ratio = spec.rate as f32 / 16000.0;
            let new_len = (audio_data.len() as f32 / ratio) as usize;
            let mut resampled = Vec::with_capacity(new_len);

            for i in 0..new_len {
                let source_index = (i as f32 * ratio) as usize;
                if source_index < audio_data.len() {
                    resampled.push(audio_data[source_index]);
                }
            }
            audio_data = resampled;
        }

        app_log_debug!("✅ Loaded {} audio samples at 16kHz mono", audio_data.len());
        Ok(audio_data)
    }
}

/// Audio model status for frontend
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioModelStatus {
    pub available: bool,
    pub model_loaded: bool,
    pub supported_formats: Vec<String>,
}
