# Testing Module

## Rust Unit Tests

```rust
// src-tauri/src/storage.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_item_metadata_creation() {
        let metadata = ItemMetadata::new("Test Item".to_string());

        assert!(!metadata.id.is_empty());
        assert_eq!(metadata.title, "Test Item");
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_save_and_load_metadata() {
        let temp = tempdir().unwrap();
        std::env::set_var("HOME", temp.path());

        let metadata = ItemMetadata::new("Test".to_string());
        save_metadata(&metadata).unwrap();

        let loaded = load_metadata(&metadata.id).unwrap();
        assert_eq!(loaded.title, "Test");
    }
}
```

## Integration Tests

```rust
// tests/integration_test.rs
use std::process::Command;

#[test]
fn test_app_starts() {
    let output = Command::new("cargo")
        .args(["tauri", "build"])
        .output()
        .expect("Failed to build");

    assert!(output.status.success());
}
```

## Frontend Testing (Manual Checklist)

```markdown
## Recording Flow
- [ ] Click record starts recording
- [ ] Timer updates during recording
- [ ] Click stop saves recording
- [ ] Recording appears in list
- [ ] Recording can be played back

## Settings Flow
- [ ] Settings load on app start
- [ ] Changes persist after save
- [ ] Theme changes apply immediately
- [ ] Storage path is configurable

## Permissions Flow
- [ ] Permission status shows correctly
- [ ] Request permission opens dialog
- [ ] Status updates after granting
```

## Test Commands

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_item_metadata

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test '*'
```

## Tauri Test Utilities

```rust
// Mock Tauri app for testing
#[cfg(test)]
mod test_utils {
    use tauri::test::{mock_builder, MockRuntime};

    pub fn create_test_app() -> tauri::App<MockRuntime> {
        mock_builder()
            .invoke_handler(tauri::generate_handler![
                crate::commands::my_command,
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap()
    }
}
```

## E2E Testing (WebDriver)

```rust
// tests/e2e.rs
use tauri::test::MockRuntime;

#[tokio::test]
async fn test_main_window() {
    let app = create_test_app();
    let window = app.get_window("main").unwrap();

    // Verify window exists
    assert!(window.is_visible().unwrap());
}
```

## Performance Testing

```rust
#[test]
fn benchmark_audio_processing() {
    let samples: Vec<f32> = (0..48000).map(|_| rand::random()).collect();

    let start = std::time::Instant::now();
    normalize_audio(&mut samples.clone(), -23.0);
    let duration = start.elapsed();

    // Should process 1 second of audio in < 100ms
    assert!(duration.as_millis() < 100);
}
```
