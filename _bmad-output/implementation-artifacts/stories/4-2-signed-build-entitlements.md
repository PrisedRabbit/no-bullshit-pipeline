# Story 4.2: Signed Build Entitlements

Status: ready-for-dev

## Story

As a developer,
I want verified entitlements for signed builds,
so that the app passes Gatekeeper and works correctly when distributed.

## Acceptance Criteria

1. **Given** the app is built with `./build.sh`
   **When** signing completes
   **Then** all required entitlements are present (microphone, screen recording)

2. **Given** a signed DMG is installed
   **When** the user launches the app
   **Then** macOS Gatekeeper allows execution without quarantine warnings

## Tasks / Subtasks

- [ ] Task 1: Verify entitlements file (AC: 1)
  - [ ] Review `src-tauri/entitlements.plist`
  - [ ] Ensure microphone entitlement present
  - [ ] Ensure screen recording entitlement present
  - [ ] Add any missing entitlements

- [ ] Task 2: Build script verification (AC: 1, 2)
  - [ ] Review `build.sh` signing process
  - [ ] Verify entitlements are applied during signing
  - [ ] Add codesign verification step
  - [ ] Test with `codesign -dvvv` output

- [ ] Task 3: Gatekeeper testing (AC: 2)
  - [ ] Build signed DMG
  - [ ] Test installation on clean system
  - [ ] Verify no quarantine warnings
  - [ ] Document notarization process if needed

## Dev Notes

### Required Entitlements
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.device.audio-input</key>
    <true/>
    <key>com.apple.security.device.screen-capture</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
</dict>
</plist>
```

### Verification Commands
```bash
# Check entitlements
codesign -d --entitlements :- /path/to/NBP.app

# Verify signature
codesign -dvvv /path/to/NBP.app

# Check Gatekeeper
spctl -a -vvv /path/to/NBP.app
```

### Source Tree Components
- `src-tauri/entitlements.plist` - Entitlements file
- `build.sh` - Build and signing script
- `src-tauri/tauri.conf.json` - Tauri build config

### References
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.2]
- [Apple Entitlements Docs](https://developer.apple.com/documentation/bundleresources/entitlements)

## Dev Agent Record

### Agent Model Used
(To be filled by dev agent)

### Completion Notes List
(To be filled during implementation)

### File List
(To be filled during implementation)
