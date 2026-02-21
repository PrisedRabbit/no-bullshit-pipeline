# Third-Party Licenses

This project uses the following third-party libraries and models.

---

## Audio Transcription

### Whisper.cpp
- **Source:** https://github.com/ggerganov/whisper.cpp
- **Models:** https://huggingface.co/ggerganov/whisper.cpp
- **License:** MIT
- **Copyright:** Copyright (c) 2023 Georgi Gerganov

Whisper is OpenAI's speech recognition model. whisper.cpp is a C/C++ port by Georgi Gerganov.

### whisper-rs
- **Source:** https://github.com/tazz4843/whisper-rs
- **License:** Unlicense
- **Usage:** Rust bindings for whisper.cpp

---

## Local LLM Inference

### llama-cpp-2
- **Source:** https://github.com/utilityai/llama-cpp-rs
- **License:** MIT
- **Usage:** Rust bindings for llama.cpp — local GGUF model inference with Metal acceleration

---

## Core Audio & System Integration

### cidre
- **Source:** https://github.com/yury/cidre
- **License:** MIT
- **Copyright:** Copyright (c) Yury
- **Usage:** Rust bindings for Apple Core Audio, used for system audio capture via Process Taps

### cpal
- **Source:** https://github.com/RustAudio/cpal
- **License:** Apache-2.0
- **Usage:** Cross-platform audio I/O for microphone capture

### coreaudio-rs
- **Source:** https://github.com/RustAudio/coreaudio-rs
- **License:** MIT OR Apache-2.0
- **Usage:** macOS Core Audio bindings

### security-framework
- **Source:** https://github.com/kornelski/rust-security-framework
- **License:** MIT OR Apache-2.0
- **Usage:** macOS Security framework bindings

---

## Audio Processing

### rodio
- **Source:** https://github.com/RustAudio/rodio
- **License:** MIT OR Apache-2.0
- **Usage:** Audio playback

### rubato
- **Source:** https://github.com/HEnquist/rubato
- **License:** MIT
- **Usage:** High-quality audio resampling (sinc interpolation)

### hound
- **Source:** https://github.com/ruuda/hound
- **License:** Apache-2.0
- **Usage:** WAV file reading/writing

### lewton
- **Source:** https://github.com/RustAudio/lewton
- **License:** MIT OR Apache-2.0
- **Usage:** Vorbis audio decoding

### vorbis_rs
- **Source:** https://github.com/ComunidadAyworkers/vorbis_rs
- **License:** BSD-3-Clause
- **Usage:** Vorbis audio encoding

### ebur128
- **Source:** https://github.com/sdroege/ebur128
- **License:** MIT
- **Usage:** Audio loudness measurement (EBU R128)

### ringbuf
- **Source:** https://github.com/agerasev/ringbuf
- **License:** MIT OR Apache-2.0
- **Usage:** Lock-free ring buffer for real-time audio mixing

---

## Application Framework

### Tauri
- **Source:** https://github.com/tauri-apps/tauri
- **Website:** https://tauri.app
- **License:** Apache-2.0 OR MIT
- **Copyright:** Copyright (c) 2017-present Tauri Programme within The Commons Conservancy
- **Usage:** Desktop application framework (Rust backend + WebView frontend)

### Tauri Plugins
- tauri-plugin-dialog (Apache-2.0 OR MIT)
- tauri-plugin-shell (Apache-2.0 OR MIT)
- tauri-plugin-opener (Apache-2.0 OR MIT)
- tauri-plugin-fs (Apache-2.0 OR MIT)

### Sortable.js
- **Source:** https://github.com/SortableJS/Sortable
- **Version:** 1.15.6
- **License:** MIT
- **Usage:** Drag-and-drop reordering of pipeline steps in the builder

### marked
- **Source:** https://github.com/markedjs/marked
- **Version:** 17.0.3
- **License:** MIT
- **Copyright:** Copyright (c) 2018-2026, MarkedJS
- **Usage:** Markdown rendering for pipeline step outputs

---

## Networking & Serialization

### reqwest
- **Source:** https://github.com/seanmonstar/reqwest
- **License:** MIT OR Apache-2.0
- **Usage:** HTTP client for cloud AI APIs and integrations

### serde / serde_json
- **Source:** https://github.com/serde-rs/serde
- **License:** MIT OR Apache-2.0
- **Usage:** Serialization/deserialization

### serde_yaml
- **Source:** https://github.com/dtolnay/serde-yaml
- **License:** MIT OR Apache-2.0
- **Usage:** YAML configuration parsing

### encoding_rs
- **Source:** https://github.com/nicowillis/encoding_rs
- **License:** MIT OR Apache-2.0
- **Usage:** Character encoding conversion

### futures-util
- **Source:** https://github.com/rust-lang/futures-rs
- **License:** MIT OR Apache-2.0
- **Usage:** Async stream utilities for streaming API responses

### tokio
- **Source:** https://github.com/tokio-rs/tokio
- **License:** MIT
- **Usage:** Async runtime

### notion-client
- **Source:** https://github.com/Mathspy/notion-client
- **License:** MIT
- **Usage:** Notion API client for Notion integration connector

---

## Utilities

### chrono
- **Source:** https://github.com/chronotope/chrono
- **License:** MIT OR Apache-2.0
- **Usage:** Date/time handling

### uuid
- **Source:** https://github.com/uuid-rs/uuid
- **License:** Apache-2.0 OR MIT
- **Usage:** UUID generation

### anyhow
- **Source:** https://github.com/dtolnay/anyhow
- **License:** MIT OR Apache-2.0
- **Usage:** Error handling

### lazy_static
- **Source:** https://github.com/rust-lang-nursery/lazy-static.rs
- **License:** MIT OR Apache-2.0
- **Usage:** Lazily-evaluated static variables

### log
- **Source:** https://github.com/rust-lang/log
- **License:** MIT OR Apache-2.0
- **Usage:** Logging facade

### libc
- **Source:** https://github.com/rust-lang/libc
- **License:** MIT OR Apache-2.0
- **Usage:** C standard library bindings

### tempfile
- **Source:** https://github.com/Stebalien/tempfile
- **License:** MIT OR Apache-2.0
- **Usage:** Temporary file creation

### opener
- **Source:** https://github.com/Seeker14491/opener
- **License:** MIT OR Apache-2.0
- **Usage:** Open files and URLs with the default system application

---

## Full License Texts

The full text of licenses can be found at:
- MIT: https://opensource.org/licenses/MIT
- Apache-2.0: https://www.apache.org/licenses/LICENSE-2.0
- BSD-3-Clause: https://opensource.org/licenses/BSD-3-Clause
- Unlicense: https://unlicense.org

---

*Last updated: February 2026*
