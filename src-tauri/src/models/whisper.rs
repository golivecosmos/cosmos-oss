/// **WHISPER MODEL IMPLEMENTATION**
///
/// Based on candle-rs whisper example but adapted for our architecture.
/// Follows the same pattern as NomicModel for consistency.
///
/// **Model Structure:**
/// ```
/// ~/Library/Application Support/cosmos/models/whisper-base/
/// ├── config.json              (Whisper config)
/// ├── tokenizer.json           (Tokenizer)
/// └── model.safetensors         (Whisper model weights)
/// ```

use anyhow::{Error as E, Result};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::{ops::softmax, VarBuilder};
use candle_transformers::models::whisper::{self as m, audio, Config};
use crate::services::download_service::DownloadService;
use crate::{app_log_info, app_log_debug};
use tokenizers::Tokenizer;
use byteorder::{ByteOrder, LittleEndian};

/// Whisper model wrapper (similar to NomicModel)
pub struct WhisperModel {
    model: Option<Model>,
    tokenizer: Option<Tokenizer>,
    config: Option<Config>,
    device: Device,
}

/// Model enum for normal/quantized variants
pub enum Model {
    Normal(m::model::Whisper),
}

/// Transcription result
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub segments: Vec<TranscriptionSegment>,
    pub duration: f64,
    pub language: Option<String>,
}

/// Individual transcription segment
#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub confidence: Option<f32>,
}

impl Model {
    pub fn config(&self) -> &Config {
        match self {
            Self::Normal(m) => &m.config,
        }
    }

    pub fn encoder_forward(&mut self, x: &Tensor, flush: bool) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.encoder.forward(x, flush),
        }
    }

    pub fn decoder_forward(
        &mut self,
        x: &Tensor,
        xa: &Tensor,
        flush: bool,
    ) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.decoder.forward(x, xa, flush),
        }
    }

    pub fn decoder_final_linear(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.decoder.final_linear(x),
        }
    }
}

impl WhisperModel {
    /// Create a new WhisperModel
    pub fn new() -> Result<Self> {
        // Try to use GPU if available, fallback to CPU
      let device = if Device::new_metal(0).is_ok() {
            app_log_info!("🚀 GPU (Metal) detected, using GPU acceleration");
            Device::new_metal(0)?
        } else {
            app_log_info!("💻 Using CPU (no GPU available)");
            Device::Cpu
        };

        Ok(Self {
            model: None,
            tokenizer: None,
            config: None,
            device,
        })
    }

    /// Load model from local files (same pattern as NomicModel)
    pub fn load_model(&mut self) -> Result<()> {
        // Get model directory
        let model_dir = DownloadService::get_whisper_model_path()?;

        // Check if all required files exist
        let config_path = model_dir.join("config.json");
        let tokenizer_path = model_dir.join("tokenizer.json");
        let model_path = model_dir.join("model.safetensors");

        if !config_path.exists() || !tokenizer_path.exists() || !model_path.exists() {
            return Err(anyhow::anyhow!(
                "Whisper model files not found. Please download models first."
            ));
        }

        // Load config
        let config_content = std::fs::read_to_string(&config_path)?;
        let config: Config = serde_json::from_str(&config_content)?;
        app_log_debug!("✅ Loaded Whisper config");

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(E::msg)?;
        app_log_debug!("✅ Loaded Whisper tokenizer");

        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path], m::DTYPE, &self.device)?
        };
        let model = Model::Normal(m::model::Whisper::load(&vb, config.clone())?);
        app_log_debug!("✅ Loaded Whisper model weights");

        self.config = Some(config);
        self.tokenizer = Some(tokenizer);
        self.model = Some(model);
        Ok(())
    }

    /// Check if model is loaded
    pub fn is_loaded(&self) -> bool {
        self.model.is_some() && self.tokenizer.is_some() && self.config.is_some()
    }

    /// Transcribe audio data (main transcription method)
    pub fn transcribe_audio(&mut self, audio_data: &[f32]) -> Result<TranscriptionResult> {
        if !self.is_loaded() {
            return Err(anyhow::anyhow!("Model not loaded. Call load_model() first."));
        }

        let config = self.config.as_ref().unwrap();
        let tokenizer = self.tokenizer.as_ref().unwrap();
        let model = self.model.as_mut().unwrap();

        app_log_debug!("🔧 Processing {} audio samples ({:.1}s duration)",
            audio_data.len(), audio_data.len() as f32 / 16000.0);

        // Validate audio data
        let audio_min = audio_data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let audio_max = audio_data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let audio_mean = audio_data.iter().sum::<f32>() / audio_data.len() as f32;
        let audio_rms = (audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
        app_log_debug!("📊 Audio stats: min={:.3}, max={:.3}, mean={:.3}, rms={:.3}",
            audio_min, audio_max, audio_mean, audio_rms);

        // Check if audio is too quiet or silent
        if audio_rms < 0.001 {
            app_log_debug!("⚠️ Audio appears very quiet (RMS: {:.6}), may cause poor transcription", audio_rms);
        }

        // Convert PCM to mel spectrogram using proper mel filters
        let mel_filters = Self::load_mel_filters(config)?;
        let mel = audio::pcm_to_mel(config, audio_data, &mel_filters);
        let mel_len = mel.len();
        let mel = Tensor::from_vec(
            mel,
            (1, config.num_mel_bins, mel_len / config.num_mel_bins),
            &self.device,
        )?;

        app_log_debug!("🔧 Generated mel spectrogram: {:?}", mel.dims());

        // Force English language token for now
        let lang_token = token_id(tokenizer, "<|en|>")?;

        // Create decoder and run transcription
        let mut decoder = Decoder::new(
            model,
            tokenizer,
            &self.device,
            Some(lang_token), // Force English
            Some(Task::Transcribe), // Explicit transcribe task
            true,
        )?;

        app_log_debug!("🔧 Starting decoder with mel spectrogram dims: {:?}", mel.dims());
        let segments = decoder.run(&mel)?;
        app_log_debug!("✅ Decoder completed, got {} segments", segments.len());

        // Convert segments to our format - use word segments if available
        let mut transcription_segments = Vec::new();
        let mut full_text = String::new();
        let mut total_duration = 0.0;

        for segment in segments {
            // Use word segments if timestamps are enabled, otherwise fall back to segment-level
            if !segment.dr.word_segments.is_empty() {
                app_log_debug!("📝 Using {} word segments from decoder", segment.dr.word_segments.len());
                for word_segment in &segment.dr.word_segments {
                    if !word_segment.text.trim().is_empty() {
                        transcription_segments.push(TranscriptionSegment {
                            start: word_segment.start,
                            end: word_segment.end,
                            text: word_segment.text.clone(),
                            confidence: None,
                        });

                        if !full_text.is_empty() {
                            full_text.push(' ');
                        }
                        full_text.push_str(&word_segment.text);
                        total_duration = word_segment.end.max(total_duration);
                    }
                }
            } else {
                // Fallback to segment-level if no word segments
                let text = segment.dr.text.trim();
                if !text.is_empty() {
                    transcription_segments.push(TranscriptionSegment {
                        start: segment.start,
                        end: segment.start + segment.duration,
                        text: text.to_string(),
                        confidence: None,
                    });

                    if !full_text.is_empty() {
                        full_text.push(' ');
                    }
                    full_text.push_str(text);
                    total_duration = (segment.start + segment.duration).max(total_duration);
                }
            }
        }

        let result = TranscriptionResult {
            text: full_text,
            segments: transcription_segments,
            duration: total_duration,
            language: Some("en".to_string()), // TODO: Add language detection
        };

        Ok(result)
    }

    /// Load mel filters based on config (using candle's pre-computed filters)
    fn load_mel_filters(config: &Config) -> Result<Vec<f32>> {
        let mel_bytes = match config.num_mel_bins {
            80 => include_bytes!("../assets/melfilters80.bytes").as_slice(),
            128 => include_bytes!("../assets/melfilters128.bytes").as_slice(),
            nmel => return Err(anyhow::anyhow!("Unsupported num_mel_bins: {}", nmel)),
        };

        let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
        LittleEndian::read_f32_into(mel_bytes, &mut mel_filters);

        app_log_debug!("✅ Loaded {} mel filters for {} mel bins from candle binary file", mel_filters.len(), config.num_mel_bins);
        Ok(mel_filters)
    }

}

/// Decoder implementation (adapted from candle example)
struct Decoder<'a> {
    model: &'a mut Model,
    task: Option<Task>,
    timestamps: bool,
    tokenizer: &'a Tokenizer,
    suppress_tokens: Tensor,
    sot_token: u32,
    transcribe_token: u32,
    translate_token: u32,
    eot_token: u32,
    no_speech_token: u32,
    no_timestamps_token: u32,
    language_token: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
enum Task {
    Transcribe,
}

#[derive(Debug, Clone)]
struct DecodingResult {
    text: String,
    avg_logprob: f64,
    no_speech_prob: f64,
    word_segments: Vec<WordSegment>, // New: word-level timing
}

#[derive(Debug, Clone)]
struct WordSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Debug, Clone)]
struct Segment {
    start: f64,
    duration: f64,
    dr: DecodingResult,
}

impl<'a> Decoder<'a> {
    // Helper function to check if a token is a timestamp token
    fn is_timestamp_token(&self, token: u32) -> bool {
        // Whisper timestamp tokens are typically in the range 50364-51864 (for 30s max)
        // These represent 0.02s increments: <|0.00|>, <|0.02|>, <|0.04|>, etc.
        token >= 50364 && token < 51864
    }

    // Convert timestamp token to seconds
    fn timestamp_token_to_seconds(&self, token: u32) -> f64 {
        if self.is_timestamp_token(token) {
            // Timestamp tokens start at 50364 and increment by 1 for each 0.02s
            ((token - 50364) as f64) * 0.02
        } else {
            0.0
        }
    }

    // Parse tokens into word segments with timestamps
    fn parse_tokens_with_timestamps(&self, tokens: &[u32], segment_start_time: f64) -> Vec<WordSegment> {
        let mut word_segments = Vec::new();
        let mut current_start_time = segment_start_time;
        let mut current_text_tokens = Vec::new();

        for &token in tokens {
            if self.is_timestamp_token(token) {
                // If we have accumulated text tokens, create a segment
                if !current_text_tokens.is_empty() {
                    if let Ok(text) = self.tokenizer.decode(&current_text_tokens, true) {
                        let trimmed_text = text.trim();
                        if !trimmed_text.is_empty() {
                            let timestamp = self.timestamp_token_to_seconds(token);
                            word_segments.push(WordSegment {
                                start: current_start_time,
                                end: segment_start_time + timestamp,
                                text: trimmed_text.to_string(),
                            });
                            current_start_time = segment_start_time + timestamp;
                        }
                    }
                    current_text_tokens.clear();
                }
            } else if token != self.sot_token
                && token != self.eot_token
                && token != self.transcribe_token
                && token != self.translate_token
                && token != self.no_timestamps_token
                && Some(token) != self.language_token {
                // Accumulate non-special tokens for text
                current_text_tokens.push(token);
            }
        }

        // Handle remaining text tokens without a final timestamp
        if !current_text_tokens.is_empty() {
            if let Ok(text) = self.tokenizer.decode(&current_text_tokens, true) {
                let trimmed_text = text.trim();
                if !trimmed_text.is_empty() {
                    // For the last segment, estimate end time based on segment duration
                    word_segments.push(WordSegment {
                        start: current_start_time,
                        end: current_start_time + 1.0, // Default 1 second if no final timestamp
                        text: trimmed_text.to_string(),
                    });
                }
            }
        }

        word_segments
    }

    fn new(
        model: &'a mut Model,
        tokenizer: &'a Tokenizer,
        device: &Device,
        language_token: Option<u32>,
        task: Option<Task>,
        timestamps: bool,
    ) -> Result<Self> {
        let no_timestamps_token = token_id(tokenizer, m::NO_TIMESTAMPS_TOKEN)?;

        let suppress_tokens: Vec<f32> = (0..model.config().vocab_size as u32)
            .map(|i| {
                if model.config().suppress_tokens.contains(&i)
                    || timestamps && i == no_timestamps_token
                {
                    f32::NEG_INFINITY
                } else {
                    0f32
                }
            })
            .collect();
        let suppress_tokens = Tensor::new(suppress_tokens.as_slice(), device)?;

        let sot_token = token_id(tokenizer, m::SOT_TOKEN)?;
        let transcribe_token = token_id(tokenizer, m::TRANSCRIBE_TOKEN)?;
        let translate_token = token_id(tokenizer, m::TRANSLATE_TOKEN)?;
        let eot_token = token_id(tokenizer, m::EOT_TOKEN)?;

        let no_speech_token = m::NO_SPEECH_TOKENS
            .iter()
            .find_map(|token| token_id(tokenizer, token).ok())
            .ok_or_else(|| anyhow::anyhow!("unable to find any non-speech token"))?;

        Ok(Self {
            model,
            tokenizer,
            task,
            timestamps,
            suppress_tokens,
            sot_token,
            transcribe_token,
            translate_token,
            eot_token,
            no_speech_token,
            language_token,
            no_timestamps_token,
        })
    }

    fn decode(&mut self, mel: &Tensor, segment_start_time: f64) -> Result<DecodingResult> {
        let audio_features = self.model.encoder_forward(mel, true)?;
        let sample_len = self.model.config().max_target_positions / 2;
        let mut sum_logprob = 0f64;
        let mut no_speech_prob = f64::NAN;
        let mut tokens = vec![self.sot_token];

        if let Some(language_token) = self.language_token {
            tokens.push(language_token);
        }

        match self.task {
            None | Some(Task::Transcribe) => tokens.push(self.transcribe_token),
        }

        if !self.timestamps {
            tokens.push(self.no_timestamps_token);
        }

        for i in 0..sample_len {
            let tokens_t = Tensor::new(tokens.as_slice(), mel.device())?.unsqueeze(0)?;
            let ys = self.model.decoder_forward(&tokens_t, &audio_features, i == 0)?;

            if i == 0 {
                let logits = self.model.decoder_final_linear(&ys.i(..1)?)?.i(0)?.i(0)?;
                no_speech_prob = softmax(&logits, 0)?
                    .i(self.no_speech_token as usize)?
                    .to_scalar::<f32>()? as f64;
            }

            let (_, seq_len, _) = ys.dims3()?;
            let logits = self.model
                .decoder_final_linear(&ys.i((..1, seq_len - 1..))?)?
                .i(0)?
                .i(0)?;
            let logits = logits.broadcast_add(&self.suppress_tokens)?;

            // Simplified sampling - just use greedy (highest probability) for now
            let logits_v: Vec<f32> = logits.to_vec1()?;
            let next_token = logits_v
                .iter()
                .enumerate()
                .max_by(|(_, u), (_, v)| u.total_cmp(v))
                .map(|(i, _)| i as u32)
                .unwrap();

            tokens.push(next_token);
            let prob = softmax(&logits, candle_core::D::Minus1)?
                .i(next_token as usize)?
                .to_scalar::<f32>()? as f64;

            if next_token == self.eot_token || tokens.len() > self.model.config().max_target_positions {
                break;
            }

            // Detect repetition loops (same token repeated multiple times)
            if tokens.len() >= 10 {
                let recent_tokens = &tokens[tokens.len()-10..];
                if recent_tokens.iter().all(|&t| t == next_token) {
                    app_log_debug!("⚠️ Detected repetition loop, breaking");
                    break;
                }
            }

            sum_logprob += prob.ln();
        }

        let text = self.tokenizer.decode(&tokens, true).map_err(E::msg)?;
        let avg_logprob = sum_logprob / tokens.len() as f64;

        // Parse word segments with timestamps if timestamps are enabled
        let word_segments = if self.timestamps {
            self.parse_tokens_with_timestamps(&tokens, segment_start_time)
        } else {
            Vec::new()
        };

        Ok(DecodingResult {
            text,
            avg_logprob,
            no_speech_prob,
            word_segments,
        })
    }

    fn decode_with_fallback(&mut self, segment: &Tensor, segment_start_time: f64) -> Result<DecodingResult> {
        for (i, &_) in m::TEMPERATURES.iter().enumerate() {
            let dr = self.decode(segment, segment_start_time);
            if i == m::TEMPERATURES.len() - 1 {
                return dr;
            }
            match dr {
                Ok(dr) => {
                    let needs_fallback = dr.avg_logprob < m::LOGPROB_THRESHOLD;
                    if !needs_fallback || dr.no_speech_prob > m::NO_SPEECH_THRESHOLD {
                        return Ok(dr);
                    }
                }
                Err(_) => continue,
            }
        }
        unreachable!()
    }

    fn run(&mut self, mel: &Tensor) -> Result<Vec<Segment>> {
        let (_, _, content_frames) = mel.dims3()?;
        let mut seek = 0;
        let mut segments = vec![];

        app_log_debug!("🔧 Decoder processing {} content frames", content_frames);

        let mut segment_count = 0;
        while seek < content_frames {
            let time_offset = (seek * m::HOP_LENGTH) as f64 / m::SAMPLE_RATE as f64;
            let segment_size = usize::min(content_frames - seek, m::N_FRAMES);
            let mel_segment = mel.narrow(2, seek, segment_size)?;
            let segment_duration = (segment_size * m::HOP_LENGTH) as f64 / m::SAMPLE_RATE as f64;

            app_log_debug!("🔧 Processing segment {} (frames {}-{}, time {:.1}s-{:.1}s)",
                segment_count, seek, seek + segment_size, time_offset, time_offset + segment_duration);

            let dr = self.decode_with_fallback(&mel_segment, time_offset)?;
            seek += segment_size;
            segment_count += 1;

            // For very short audio (< 10s), be more lenient with no_speech_threshold
            let adjusted_no_speech_threshold = if content_frames < 1000 {  // ~10s
                0.8  // More lenient for short audio
            } else {
                m::NO_SPEECH_THRESHOLD  // Standard threshold
            };

            if dr.no_speech_prob > adjusted_no_speech_threshold && dr.avg_logprob < m::LOGPROB_THRESHOLD {
                app_log_debug!("⏭️ Skipping segment {} (no_speech_prob={:.3} > {:.3}, avg_logprob={:.3})",
                    segment_count - 1, dr.no_speech_prob, adjusted_no_speech_threshold, dr.avg_logprob);
                continue;
            }

            let segment = Segment {
                start: time_offset,
                duration: segment_duration,
                dr,
            };

            segments.push(segment);

            // Safety: Limit processing to prevent infinite loops on very long audio
            if segment_count > 1000 {
                app_log_debug!("⚠️ Hit segment limit (1000), stopping transcription");
                break;
            }
        }

        app_log_debug!("✅ Decoder completed {} segments", segments.len());
        Ok(segments)
    }
}

pub fn token_id(tokenizer: &Tokenizer, token: &str) -> candle_core::Result<u32> {
    match tokenizer.token_to_id(token) {
        None => Err(candle_core::Error::Msg(format!("no token-id for {}", token))),
        Some(id) => Ok(id),
    }
}
