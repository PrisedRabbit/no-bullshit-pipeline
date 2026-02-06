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

---

## Networking & Serialization

### reqwest
- **Source:** https://github.com/seanmonstar/reqwest
- **License:** MIT OR Apache-2.0
- **Usage:** HTTP client for cloud AI APIs

### serde / serde_json
- **Source:** https://github.com/serde-rs/serde
- **License:** MIT OR Apache-2.0
- **Usage:** Serialization/deserialization

### tokio
- **Source:** https://github.com/tokio-rs/tokio
- **License:** MIT
- **Usage:** Async runtime

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

---

## Full License Texts

The full text of licenses can be found at:
- MIT: https://opensource.org/licenses/MIT
- Apache-2.0: https://www.apache.org/licenses/LICENSE-2.0
- BSD-3-Clause: https://opensource.org/licenses/BSD-3-Clause
- Unlicense: https://unlicense.org

---

*Last updated: February 2026*
