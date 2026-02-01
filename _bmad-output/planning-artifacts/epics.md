# NBP - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for NBP v0.3 "The Pipeline & Polish", decomposing the roadmap into implementable stories.

## Requirements Inventory

### Functional Requirements

| ID | Requirement | Epic |
|----|-------------|------|
| FR-1 | Cloud transcription via OpenAI Whisper-1 | Epic 1 |
| FR-2 | AI summarization via GPT-4o | Epic 1 |
| FR-3 | Long-context summary via Google Gemini Flash 1.5 | Epic 1 |
| FR-4 | Structured data extraction via Claude Sonnet | Epic 1 |
| FR-5 | Secure API key storage | Epic 1 |
| FR-6 | Template system for structured outputs | Epic 2 |
| FR-7 | Meeting Notes template | Epic 2 |
| FR-8 | Brainstorm template | Epic 2 |
| FR-9 | Journal template | Epic 2 |
| FR-10 | In-app audio playback | Epic 3 |
| FR-11 | Waveform visualization | Epic 3 |
| FR-12 | Recording notification system | Epic 4 |

### Non-Functional Requirements

| ID | Requirement | Epic |
|----|-------------|------|
| NFR-1 | Robust audio mix error handling (drift prevention) | Epic 4 |
| NFR-2 | Signed build entitlements verification | Epic 4 |
| NFR-3 | Privacy-first: local storage before cloud | All |

## Epic List

1. **Epic 1: Cloud AI Integration** - Enable cloud-based transcription and AI processing
2. **Epic 2: Structured Output Templates** - Transform transcriptions into formatted documents
3. **Epic 3: Audio Playback & Visualization** - In-app audio control and waveform display
4. **Epic 4: Quality & Polish** - Error handling, build signing, and notifications

---

## Epic 1: Cloud AI Integration

Enable cloud-based AI services for transcription and intelligent processing, with secure API key management. Complements existing local Whisper transcription.

### Story 1.1: Secure API Key Storage

As a user,
I want to securely store my API keys for cloud services,
So that my credentials are protected and persist across sessions.

**Acceptance Criteria:**

**Given** I am in the Settings view
**When** I enter an API key for a service (OpenAI/Google/Anthropic)
**Then** the key is encrypted and stored in `~/.nbp/settings.json`
**And** the key is masked in the UI after saving

**Given** I have saved API keys
**When** I restart the application
**Then** my keys are available for use without re-entering

### Story 1.2: OpenAI Whisper-1 Transcription

As a user,
I want to transcribe recordings using OpenAI's Whisper-1 API,
So that I get fast, accurate transcriptions without local GPU requirements.

**Acceptance Criteria:**

**Given** I have a valid OpenAI API key configured
**When** I select "Process" on a recording and choose OpenAI Whisper
**Then** the audio is sent to Whisper-1 API and transcription is saved

**Given** the API call fails (network error, invalid key, quota exceeded)
**When** processing completes
**Then** I see a clear error message indicating the failure reason

### Story 1.3: GPT-4o Summarization

As a user,
I want to summarize transcriptions using GPT-4o,
So that I get concise summaries of long recordings.

**Acceptance Criteria:**

**Given** I have a transcription and valid OpenAI API key
**When** I select "Summarize with GPT-4o"
**Then** the transcription is sent to GPT-4o and summary is saved as `summary.md`

**Given** the transcription is very long
**When** processing
**Then** the system handles token limits appropriately (chunking or truncation with warning)

### Story 1.4: Google Gemini Integration

As a user,
I want to process long recordings using Google Gemini Flash 1.5,
So that I can summarize hour-long meetings without token limit issues.

**Acceptance Criteria:**

**Given** I have a valid Google API key configured
**When** I select "Process with Gemini"
**Then** the transcription is sent to Gemini Flash 1.5 for long-context processing

**Given** I have a recording longer than 1 hour
**When** I process with Gemini
**Then** the full context is handled without chunking

### Story 1.5: Anthropic Claude Integration

As a user,
I want to extract structured data using Claude Sonnet,
So that I get high-quality extraction for meeting action items, decisions, and key points.

**Acceptance Criteria:**

**Given** I have a valid Anthropic API key configured
**When** I select "Extract with Claude"
**Then** the transcription is processed and structured data is saved as JSON

**Given** I select a template type (Meeting Notes, Brainstorm, Journal)
**When** Claude processes the transcription
**Then** the output matches the template structure

---

## Epic 2: Structured Output Templates

Transform raw transcriptions into formatted, structured documents using predefined templates.

### Story 2.1: Template System Core

As a user,
I want a template system that defines output structures,
So that my transcriptions can be automatically formatted.

**Acceptance Criteria:**

**Given** templates are defined in `~/.nbp/templates/`
**When** I process a transcription with a template
**Then** the output follows the template structure

**Given** I want to add a custom template
**When** I create a new `.json` or `.md` template file
**Then** it appears in the template selection dropdown

### Story 2.2: Meeting Notes Template

As a user,
I want a Meeting Notes template,
So that my meeting recordings become structured notes with attendees, agenda, decisions, and action items.

**Acceptance Criteria:**

**Given** I have a meeting transcription
**When** I apply the Meeting Notes template
**Then** the output includes: Date, Attendees, Agenda Items, Key Decisions, Action Items (with owners), Follow-ups

**Given** the AI cannot identify certain sections
**When** processing completes
**Then** those sections are marked as "Not identified" rather than omitted

### Story 2.3: Brainstorm Template

As a user,
I want a Brainstorm template,
So that my ideation sessions become organized lists of ideas with categories and priorities.

**Acceptance Criteria:**

**Given** I have a brainstorm session transcription
**When** I apply the Brainstorm template
**Then** the output includes: Topic, Ideas (grouped by theme), Top 3 Priorities, Next Steps

### Story 2.4: Journal Template

As a user,
I want a Journal template,
So that my voice journal entries become formatted diary entries.

**Acceptance Criteria:**

**Given** I have a journal recording
**When** I apply the Journal template
**Then** the output includes: Date, Mood (inferred), Key Thoughts, Reflections, Gratitude items

---

## Epic 3: Audio Playback & Visualization

Add in-app audio controls and visual feedback for recorded audio.

### Story 3.1: In-App Audio Playback

As a user,
I want to play recordings directly in the app,
So that I don't need to open external applications.

**Acceptance Criteria:**

**Given** I am viewing a recording's detail view
**When** I click the play button
**Then** the mixed audio plays through system speakers

**Given** audio is playing
**When** I click pause
**Then** playback pauses and can be resumed

**Given** audio is playing
**When** I use the seek bar
**Then** playback jumps to the selected position

**Given** I switch to a different recording
**When** audio is playing
**Then** playback stops automatically

### Story 3.2: Waveform Preview

As a user,
I want to see a waveform visualization of my recordings,
So that I can visually navigate and identify sections.

**Acceptance Criteria:**

**Given** I am viewing a recording's detail view
**When** the view loads
**Then** a waveform of the audio is displayed

**Given** the waveform is displayed
**When** audio is playing
**Then** a playhead indicator shows the current position

**Given** the waveform is displayed
**When** I click on a position in the waveform
**Then** playback seeks to that position

---

## Epic 4: Quality & Polish

Improve reliability, build process, and user awareness features.

### Story 4.1: Audio Mix Error Handling

As a user,
I want robust error handling during recording,
So that audio drift and mix failures are detected and reported gracefully.

**Acceptance Criteria:**

**Given** mic and system audio are being recorded
**When** sample rate drift is detected
**Then** the system compensates or warns me without crashing

**Given** one audio source fails during recording
**When** the failure occurs
**Then** I receive a notification and the other source continues recording

**Given** a recording completes with errors
**When** I view the recording
**Then** I see an indicator that issues occurred during capture

### Story 4.2: Signed Build Entitlements

As a developer,
I want verified entitlements for signed builds,
So that the app passes Gatekeeper and works correctly when distributed.

**Acceptance Criteria:**

**Given** the app is built with `./build.sh`
**When** signing completes
**Then** all required entitlements are present (microphone, screen recording)

**Given** a signed DMG is installed
**When** the user launches the app
**Then** macOS Gatekeeper allows execution without quarantine warnings

### Story 4.3: Recording Notification

As a user,
I want to optionally notify others when NBP is recording,
So that meeting participants are aware of the recording.

**Acceptance Criteria:**

**Given** I am about to start a recording
**When** notification setting is enabled
**Then** a system notification is displayed indicating recording is active

**Given** I prefer silent recording
**When** I disable the notification setting
**Then** no notification is shown when recording starts

**Given** recording is in progress
**When** the notification is visible
**Then** it clearly identifies NBP as the recording application

---

## FR Coverage Map

| Requirement | Stories |
|-------------|---------|
| FR-1 | 1.2 |
| FR-2 | 1.3 |
| FR-3 | 1.4 |
| FR-4 | 1.5 |
| FR-5 | 1.1 |
| FR-6 | 2.1 |
| FR-7 | 2.2 |
| FR-8 | 2.3 |
| FR-9 | 2.4 |
| FR-10 | 3.1 |
| FR-11 | 3.2 |
| FR-12 | 4.3 |
| NFR-1 | 4.1 |
| NFR-2 | 4.2 |
