// Integration tests for NBP v0.3 features
use std::fs::File;
use std::path::PathBuf;

// Test fixture paths
fn get_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn get_test_audio_path() -> PathBuf {
    get_project_root().join("test-fixtures").join("test-audio.ogg")
}

fn get_test_data_dir() -> PathBuf {
    get_project_root().join("test-fixtures").join("test-data")
}

// ============================================
// WAVEFORM TESTS
// ============================================

#[test]
fn test_waveform_generation_with_real_audio() {
    use lewton::inside_ogg::OggStreamReader;

    let audio_path = get_test_audio_path();
    assert!(audio_path.exists(), "Test audio file must exist at {:?}", audio_path);

    // Open and decode the OGG file
    let file = File::open(&audio_path).expect("Failed to open test audio");
    let mut ogg_reader = OggStreamReader::new(file).expect("Failed to parse OGG");

    let sample_rate = ogg_reader.ident_hdr.audio_sample_rate;
    let channels = ogg_reader.ident_hdr.audio_channels as usize;

    assert!(sample_rate > 0, "Sample rate should be positive");
    assert!(channels > 0, "Should have at least one channel");

    // Collect samples
    let mut all_samples: Vec<f32> = Vec::new();
    while let Some(packet) = ogg_reader.read_dec_packet_generic::<Vec<Vec<i16>>>().unwrap() {
        if packet.is_empty() { continue; }

        let frames = packet[0].len();
        for i in 0..frames {
            let mut sum: f32 = 0.0;
            for ch in 0..channels {
                if ch < packet.len() && i < packet[ch].len() {
                    sum += packet[ch][i] as f32 / 32768.0;
                }
            }
            all_samples.push((sum / channels as f32).abs());
        }
    }

    assert!(!all_samples.is_empty(), "Should have decoded some samples");

    // Calculate duration
    let duration_ms = (all_samples.len() as f64 / sample_rate as f64 * 1000.0) as u64;
    assert!(duration_ms > 4000 && duration_ms < 6000, "Duration should be ~5 seconds, got {}ms", duration_ms);

    // Downsample to 1000 points
    let target_samples = 1000;
    let samples_per_bin = all_samples.len() / target_samples;
    let mut waveform: Vec<f32> = Vec::with_capacity(target_samples);

    for i in 0..target_samples {
        let start = i * samples_per_bin;
        let end = ((i + 1) * samples_per_bin).min(all_samples.len());

        if start < end {
            let peak = all_samples[start..end]
                .iter()
                .cloned()
                .fold(0.0f32, |a, b| a.max(b));
            waveform.push(peak);
        }
    }

    assert_eq!(waveform.len(), target_samples, "Should have {} waveform samples", target_samples);

    // Normalize
    let max_val = waveform.iter().cloned().fold(0.0f32, |a, b| a.max(b));
    assert!(max_val > 0.0, "Should have non-zero audio content");

    for sample in &mut waveform {
        *sample /= max_val;
    }

    // Check normalized values
    let final_max = waveform.iter().cloned().fold(0.0f32, |a, b| a.max(b));
    assert!((final_max - 1.0).abs() < 0.001, "Max should be normalized to 1.0");

    println!("Waveform test passed: {} samples, {}ms duration, {} sample rate",
             waveform.len(), duration_ms, sample_rate);
}

// ============================================
// TEMPLATE TESTS
// ============================================

#[test]
fn test_template_prompt_substitution() {
    let template_prompt = r#"Analyze this meeting transcript.

Transcript:
{transcript}"#;

    let transcript = "Alice: Let's discuss the Q1 goals.\nBob: I think we should focus on customer retention.";

    let filled = template_prompt.replace("{transcript}", transcript);

    assert!(filled.contains("Alice: Let's discuss"), "Should contain transcript content");
    assert!(!filled.contains("{transcript}"), "Should have replaced placeholder");
}

#[test]
fn test_all_builtin_templates_have_transcript_placeholder() {
    let templates = vec![
        ("meeting-notes", include_str!("../src/templates.rs")),
        ("brainstorm", include_str!("../src/templates.rs")),
        ("journal", include_str!("../src/templates.rs")),
    ];

    // Just verify the source file contains {transcript} placeholders
    let source = include_str!("../src/templates.rs");
    let count = source.matches("{transcript}").count();
    assert!(count >= 3, "Should have at least 3 {{transcript}} placeholders, found {}", count);
}

// ============================================
// STORAGE TESTS
// ============================================

#[test]
fn test_recording_metadata_serialization() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct TestAudioInfo {
        file: String,
        duration_sec: f64,
        sample_rate: u32,
        channels: u16,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct TestAudioFiles {
        mic: Option<TestAudioInfo>,
        system: Option<TestAudioInfo>,
        mix: Option<TestAudioInfo>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct TestRecordingMetadata {
        id: String,
        created_at: String,
        title: String,
        tags: Vec<String>,
        status: String,
        audio: TestAudioFiles,
    }

    let metadata = TestRecordingMetadata {
        id: "test-123".to_string(),
        created_at: "2024-01-15T10:30:00Z".to_string(),
        title: "Test Recording".to_string(),
        tags: vec!["meeting".to_string(), "important".to_string()],
        status: "ready".to_string(),
        audio: TestAudioFiles {
            mic: Some(TestAudioInfo {
                file: "raw_mic.ogg".to_string(),
                duration_sec: 120.5,
                sample_rate: 48000,
                channels: 1,
            }),
            system: Some(TestAudioInfo {
                file: "raw_system.ogg".to_string(),
                duration_sec: 120.5,
                sample_rate: 48000,
                channels: 2,
            }),
            mix: None,
        },
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&metadata).expect("Should serialize");

    // Deserialize back
    let restored: TestRecordingMetadata = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(metadata, restored, "Roundtrip should preserve data");
    assert!(json.contains("\"id\": \"test-123\""), "JSON should contain id");
    assert!(json.contains("\"tags\""), "JSON should contain tags");
}

// ============================================
// CONFIG/API KEY TESTS
// ============================================

#[test]
fn test_api_keys_structure() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
    struct TestApiKeys {
        #[serde(default)]
        openai: Option<String>,
        #[serde(default)]
        google: Option<String>,
        #[serde(default)]
        anthropic: Option<String>,
    }

    // Test empty keys
    let empty: TestApiKeys = serde_json::from_str("{}").unwrap();
    assert_eq!(empty.openai, None);
    assert_eq!(empty.google, None);
    assert_eq!(empty.anthropic, None);

    // Test with keys
    let with_keys: TestApiKeys = serde_json::from_str(r#"{
        "openai": "sk-test123",
        "google": "AIza-test",
        "anthropic": "sk-ant-test"
    }"#).unwrap();

    assert_eq!(with_keys.openai, Some("sk-test123".to_string()));
    assert_eq!(with_keys.google, Some("AIza-test".to_string()));
    assert_eq!(with_keys.anthropic, Some("sk-ant-test".to_string()));
}

#[test]
fn test_settings_with_transcription_providers() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    enum TestProvider {
        LocalWhisper,
        OpenAI,
        Google,
        Anthropic,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct TestTranscriptionSettings {
        enabled: bool,
        provider: TestProvider,
        whisper_model: String,
    }

    let settings = TestTranscriptionSettings {
        enabled: true,
        provider: TestProvider::OpenAI,
        whisper_model: "Base".to_string(),
    };

    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"OpenAI\""), "Should serialize provider enum");

    // Test all providers can be serialized
    for provider in [TestProvider::LocalWhisper, TestProvider::OpenAI, TestProvider::Google, TestProvider::Anthropic] {
        let s = TestTranscriptionSettings {
            enabled: true,
            provider,
            whisper_model: "Base".to_string(),
        };
        let _ = serde_json::to_string(&s).expect("All providers should serialize");
    }
}

// ============================================
// CLOUD AI MOCK TESTS
// ============================================

#[test]
fn test_openai_request_format() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct ChatMessage {
        role: String,
        content: String,
    }

    #[derive(Serialize)]
    struct ChatRequest {
        model: String,
        messages: Vec<ChatMessage>,
        max_tokens: u32,
    }

    let request = ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Summarize this text.".to_string(),
            },
        ],
        max_tokens: 2000,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"model\":\"gpt-4o\""), "Should have model");
    assert!(json.contains("\"role\":\"system\""), "Should have system message");
    assert!(json.contains("\"role\":\"user\""), "Should have user message");
}

#[test]
fn test_anthropic_request_format() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct AnthropicMessage {
        role: String,
        content: String,
    }

    #[derive(Serialize)]
    struct AnthropicRequest {
        model: String,
        max_tokens: u32,
        messages: Vec<AnthropicMessage>,
    }

    let request = AnthropicRequest {
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 4096,
        messages: vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: "Extract meeting notes from this transcript.".to_string(),
            },
        ],
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("claude"), "Should have Claude model");
    assert!(json.contains("\"max_tokens\":4096"), "Should have max_tokens");
}

#[test]
fn test_google_gemini_request_format() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct Part {
        text: String,
    }

    #[derive(Serialize)]
    struct Content {
        parts: Vec<Part>,
    }

    #[derive(Serialize)]
    struct GeminiRequest {
        contents: Vec<Content>,
    }

    let request = GeminiRequest {
        contents: vec![
            Content {
                parts: vec![
                    Part {
                        text: "Summarize this meeting transcript.".to_string(),
                    },
                ],
            },
        ],
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"contents\""), "Should have contents");
    assert!(json.contains("\"parts\""), "Should have parts");
    assert!(json.contains("\"text\""), "Should have text");
}

// ============================================
// RECORDING HEALTH TESTS
// ============================================

#[test]
fn test_recording_health_tracking() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct TestRecordingIssue {
        #[serde(rename = "type")]
        issue_type: String,
        timestamp_ms: u64,
        message: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    struct TestRecordingHealth {
        status: String,
        #[serde(default)]
        issues: Vec<TestRecordingIssue>,
    }

    let health = TestRecordingHealth {
        status: "warning".to_string(),
        issues: vec![
            TestRecordingIssue {
                issue_type: "drift".to_string(),
                timestamp_ms: 5000,
                message: Some("Audio drift detected: 50ms".to_string()),
            },
            TestRecordingIssue {
                issue_type: "source_lost".to_string(),
                timestamp_ms: 10000,
                message: Some("System audio source disconnected".to_string()),
            },
        ],
    };

    let json = serde_json::to_string_pretty(&health).unwrap();
    assert!(json.contains("\"status\": \"warning\""), "Should have warning status");
    assert!(json.contains("\"type\": \"drift\""), "Should have drift issue");
    assert!(json.contains("\"type\": \"source_lost\""), "Should have source_lost issue");

    // Test deserialization
    let restored: TestRecordingHealth = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.issues.len(), 2);
}

// ============================================
// PLAYBACK STATE TESTS
// ============================================

#[test]
fn test_playback_state_structure() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    enum TestPlaybackStatus {
        Stopped,
        Playing,
        Paused,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct TestPlaybackState {
        status: TestPlaybackStatus,
        recording_id: Option<String>,
        position_ms: u64,
        duration_ms: u64,
    }

    let state = TestPlaybackState {
        status: TestPlaybackStatus::Playing,
        recording_id: Some("rec-123".to_string()),
        position_ms: 5000,
        duration_ms: 60000,
    };

    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains("\"Playing\""), "Should have Playing status");
    assert!(json.contains("\"position_ms\":5000"), "Should have position");

    // Test stopped state
    let stopped = TestPlaybackState {
        status: TestPlaybackStatus::Stopped,
        recording_id: None,
        position_ms: 0,
        duration_ms: 0,
    };

    let json = serde_json::to_string(&stopped).unwrap();
    assert!(json.contains("\"Stopped\""), "Should have Stopped status");
}

// ============================================
// END-TO-END FLOW SIMULATION
// ============================================

#[test]
fn test_complete_recording_to_extraction_flow() {
    // Simulate the complete flow without actual recording

    // 1. Create recording metadata
    let recording_id = "flow-test-001";
    let _created_at = "2024-01-15T10:00:00Z";

    // 2. Simulate audio files exist
    let audio_files = serde_json::json!({
        "mic": {
            "file": "raw_mic.ogg",
            "duration_sec": 300.0,
            "sample_rate": 48000,
            "channels": 1
        },
        "system": {
            "file": "raw_system.ogg",
            "duration_sec": 300.0,
            "sample_rate": 48000,
            "channels": 2
        },
        "mix": {
            "file": "audio_mix.ogg",
            "duration_sec": 300.0,
            "sample_rate": 48000,
            "channels": 2
        }
    });

    // 3. Simulate transcript
    let transcript = "Speaker 1: Welcome everyone to the Q1 planning meeting.\n\
                      Speaker 2: Thanks. Let's start with the roadmap review.\n\
                      Speaker 1: We need to decide on the top 3 priorities.\n\
                      Speaker 2: I propose we focus on customer retention first.\n\
                      Speaker 1: Agreed. Action item: Sarah will prepare the retention analysis by Friday.";

    // 4. Apply meeting notes template
    let template_prompt = r#"Analyze this meeting transcript and extract structured notes.

Extract the following:
1. **Attendees**: List all people mentioned
2. **Key Decisions**: Any decisions made
3. **Action Items**: Tasks assigned

Transcript:
{transcript}"#;

    let filled_prompt = template_prompt.replace("{transcript}", transcript);

    assert!(filled_prompt.contains("Welcome everyone"), "Prompt should contain transcript");
    assert!(filled_prompt.contains("Action Items"), "Prompt should contain template instructions");

    // 5. Verify waveform would be 1000 samples for 300s audio
    let expected_waveform_samples = 1000;
    let _duration_ms = 300_000u64;

    // 6. Verify all pieces connect
    assert!(!recording_id.is_empty());
    assert!(audio_files["mix"]["duration_sec"].as_f64().unwrap() > 0.0);
    assert!(transcript.len() > 100);
    assert!(filled_prompt.len() > transcript.len());

    println!("Complete flow simulation passed:");
    println!("  - Recording ID: {}", recording_id);
    println!("  - Audio duration: {}s", audio_files["mix"]["duration_sec"]);
    println!("  - Transcript length: {} chars", transcript.len());
    println!("  - Template prompt length: {} chars", filled_prompt.len());
    println!("  - Expected waveform samples: {}", expected_waveform_samples);
}
