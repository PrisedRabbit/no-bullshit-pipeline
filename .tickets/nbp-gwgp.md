---
id: nbp-gwgp
status: open
deps: []
links: []
created: 2026-03-24T19:34:44Z
type: feature
priority: 1
assignee: hltm-loop
---
# System notification on call detection to launch NBP

Detect when a phone/video call starts (Zoom, Teams, FaceTime, phone call, etc.) and send a macOS system notification so the user can quickly open NBP and start recording. This runs as a background listener — NBP doesn't need to be open.


## Notes

**2026-03-24T19:42:05Z**

## Research: How to Detect Calls on macOS

### Recommended Approach (two signals combined)

**1. Core Audio mic activation (primary trigger)**
- Monitor `kAudioDevicePropertyDeviceIsRunningSomewhere` on input devices
- Flips to true when ANY app activates the mic — catches Zoom, Teams, FaceTime, Meet, phone continuity, everything
- Event-driven (no polling), low overhead
- Use `coreaudio-sys` crate (project already uses Core Audio via cpal/cidre)
- No special permissions needed

**2. Process enumeration (secondary — identify which app)**
- When mic activates, scan running processes for known call apps (Zoom, Teams, FaceTime, etc.)
- Use `sysinfo` crate to enumerate processes by name/bundle ID
- Helps distinguish a Zoom call from a voice memo (reduce false positives)

### Optional enhancements
- **CoreMediaIO camera monitoring** — `kCMIODevicePropertyDeviceIsRunningSomewhere` for video call confidence
- **Audio device list monitoring** — detect Zoom/Teams virtual audio devices (ZoomAudioDevice, etc.)
- **NSWorkspace notifications** — track when call apps launch/quit to maintain a running set

### Known call app process names
- Zoom: `zoom.us`, `CptHost`
- Teams: `Microsoft Teams`
- FaceTime: `avconferenced`, `callservicesd`
- Slack: `Slack`
- WebEx: `Webex`
- Browser-based (Meet): detected via mic activation only

### Reference projects
- **OverSight** (Objective-See) — open-source macOS mic/camera monitor, uses same Core Audio approach
- **MicSwitch** — open-source, uses `kAudioDevicePropertyDeviceIsRunningSomewhere`

### Permissions needed
- None for mic activity monitoring (listening to property changes != recording)
- No Accessibility, no Full Disk Access, no special entitlements
- Works in non-sandboxed Tauri app

### macOS notification
- Use `notify-rust` crate or Tauri's notification plugin to send system notification
- Notification can include action button to launch/focus NBP
