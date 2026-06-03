# Calendar integration — design / plan

**Branch:** `calendar-integration` (off `dev`). **Phase 1 IMPLEMENTED** (see
"Implemented — Phase 1" at the bottom). The sections below are the agreed
design; the implementation followed it with the decisions noted inline.

## Goal
When a meeting recording happens, map it to a calendar event **by time** → set
the recording **title** from the event + capture **attendees**. Fuel for
"who did I meet with" pipelines later.

## Approach: native EventKit — NO OAuth in our code
- Use the **`objc2-event-kit`** crate (sits on our existing `objc2` /
  `objc2-foundation` stack — same as the notification bindings).
- EventKit reads the **local macOS EventStore = all accounts synced to
  Calendar.app** (Google / iCloud / Exchange). We never touch OAuth — the OS
  owns the account.
- **Prereq (one-time, OS-level):** the user adds their calendar account in
  *System Settings → Internet Accounts*. Andrew's isn't set up yet → no events
  until he adds Google/iCloud there. This is NOT our OAuth; the OS does it.
- **Permission:** `EKEventStore.requestFullAccessToEvents` (macOS 14+) + Info.plist
  key `NSCalendarsFullAccessUsageDescription`. One TCC prompt, modeled on the
  existing mic/screen permission flow.
- **Silent — no notifications.** Only the single Calendar TCC permission, no
  notification prompts or alerts from this feature (per Andrew: "вырезать
  нотификахи"). _Confirm this is what he meant._

## How meeting tools do it (research)
Granola / Otter map by **calendar-event timing** — activate on event start,
pull **attendees for context**. The heuristic is a time window, not a fancy
algorithm. Sources: Granola guide (wondertools), Apple EventKit docs,
objc2-event-kit (crates.io / docs.rs).

## Matching design (Phase 1)
- Hook at recording **FINALIZE** (we already have `created_at` + transcript
  ready; auto-title is set there).
- Query events in a window around the recording start.
- **Match rule:** event interval `[start,end]` **contains** the recording start
  (strong); OR recording start within `[event.start − 5min, event.start +
  10min]` (people join early/late). Multiple → pick start closest to recording
  start. None → ad-hoc meeting, keep the auto-title.
- **Apply:** `event.title` → recording title (only if still the default
  auto-title — never overwrite a user-set title); `event.attendees` → new
  metadata `attendees: [{name, email}]`.
- Store `calendar_event_id` + a `matched` flag so it's overridable / re-runnable.

## UI
- Recording detail view: show the matched event (`📅 <title> · N attendees`),
  let the user **clear / re-pick** (false positives are expected — Andrew).

## Edge cases (Andrew's concerns)
- Ad-hoc (no event) → no match → keep auto-title.
- Started late → the window catches "a bit late"; far off → no match (fine).
- Wrong-but-overlapping event → overridable via the clear/re-pick.

## Decisions (agreed defaults — change if needed)
1. Match at **FINALIZE only** (not tentatively at recording start).
2. Tolerance: start inside the event interval, or within `[−5min, +10min]` of
   event start. Tunable.
3. Scope: **all recordings** try to match (ad-hoc just won't find one). Not
   dictation (meetings only).
4. **Light override** (show matched event + clear/re-pick) ships in Phase 1.

## Phasing
- **Phase 1 (this branch):** permission + EventKit read + match-at-finalize →
  set title + store attendees + light detail-view show/clear.
- **Phase 2 (later):** Granola-style — watch the calendar, auto-prompt/auto-start
  at event time, attendees → "who I met with" pipelines / rollups.

## Implementation notes (for the build)
- `Cargo.toml`: add `objc2-event-kit` (matching features for `EKEventStore`,
  `EKEvent`, `EKParticipant`, predicates).
- New module `src-tauri/src/calendar.rs`: request access; build
  `predicateForEvents(withStart:end:calendars:)`; `events(matching:)`; extract
  `title` + `attendees` (`EKParticipant.name`, email via `.url` `mailto:`).
- `RecordingMetadata`: add `attendees: Vec<{name, email}>` (serde default `[]`)
  + optional `calendar_event_id` / matched flag.
- Hook: the recording finalize / auto-title path (`pipeline_engine` auto-title +
  `storage`). Recordings only, NOT dictation.
- Tauri commands: `calendar_permission_status` / `request_calendar_access`;
  `rematch_calendar(recording_id)` / `clear_calendar_match(recording_id)`.
- macOS-only (`#[cfg(target_os = "macos")]`); Info.plist + entitlements update.

## Open questions
- ~~"вырезать нотификахи"~~ — RESOLVED: Andrew deletes Calendar.app notifications
  himself (he doesn't use it); the feature is read-only / data-access only.
- Attendee email may be absent for some participants — store what EventKit gives
  (`name` always-ish; email best-effort from `mailto:` url).
- ~~Window tolerance~~ — RESOLVED: replaced by span-overlap + 2-min minimum
  (capped at recording length). Tune the 2-min bar against real meetings later.

## Implemented — Phase 1

Shipped on this branch (compiles, lints, bundles, unit tests pass; reviewed by a
4-lens adversarial agent pass — 1 finding fixed). **Not committed** — pending
Andrew's live test.

**Decisions taken (defaults from the spec):**
- **Opt-in = the OS permission.** No separate enable toggle. Matching runs
  whenever Calendar access is granted; revoke in System Settings to turn it off.
- **Silent.** No notifications anywhere; the only prompt is the single,
  user-initiated TCC dialog from Settings → Recording → Calendar → Grant.
- **Match rule = span overlap** (revised from the start-instant rule): the
  recording's full span `[start, start+duration]` is intersected with each
  event; the event with the **most overlap** wins (ties → closest start). This
  catches "started 6 min early and recorded through the meeting", which the old
  `±5/+10` start-instant rule missed. Query pads ±30min around the span.
- **Minimum overlap** to count as a match = 2 min, **capped at the recording's
  own length** — so a short recording fully inside a long meeting (e.g. a 1-hour
  block done in 15 min) still matches, while a recording that merely clips a
  neighbouring event by seconds does not. (Pure `qualifying_overlap`, unit-tested.)
- **All-day events are excluded** (vacations/birthdays "contain" any same-day
  start and would mis-match when no real meeting exists).
- **Title** is replaced by the event title only when still an auto-default
  (`NBP · HH:MM`, `{App} · HH:MM`, empty, `Untitled Recording`, `Recording …`).
- **macOS 14+** for the permission request (older → clear error). Read window
  ±30min (the precise rule filters inside it).

**What it does:** at recording finalize (`audio.rs::finalize_recording`, bg
thread, recordings-only — dictation never reaches it + guarded by `source`),
`calendar::try_match_on_finalize` queries EventKit around the start time,
matches an event, and stamps `title` + `attendees` into metadata.

**Files:**
- `src-tauri/src/calendar.rs` — new module: `has_full_access` (non-prompting
  status), `request_full_access` (RcBlock + channel, 60s), `match_event_for`,
  `attendees_of` / `email_of` (mailto from `EKParticipant.URL()`),
  `is_default_title`, `parse_rfc3339_unix`, `try_match_on_finalize`, commands
  `request_calendar_permission` / `rematch_calendar` / `clear_calendar_match` /
  `open_calendar_settings`. + unit tests.
- `storage.rs` — `RecordingMetadata` gains `attendees: Vec<Attendee>`,
  `calendar_event_id: Option<String>`, `calendar_matched: bool` (all
  `#[serde(default)]`, back-compat); new `Attendee { name, email }`.
- `audio.rs` — `try_match_on_finalize` call before `write_metadata`.
- `permissions.rs` — `PermissionsState.calendar`; refreshed in `check_permissions`.
- `lib.rs` — `mod calendar` + 4 commands registered.
- `Cargo.toml` — `objc2-event-kit = "0.3"`; objc2-foundation += NSDate/NSArray/NSURL.
- `Info.plist` — `NSCalendarsFullAccessUsageDescription`; `entitlements.plist` —
  `com.apple.security.personal-information.calendars`.
- Frontend: Settings → Recording → **Calendar** section (status + Grant);
  recording detail shows `📅 Calendar · N attendees` + chips + Re-match / Clear;
  styles in `styles.css`.

**Prereq to test (Andrew):** connect a calendar account in System Settings →
Internet Accounts (none connected yet), run a real build (`bun run dev` — the new
Info.plist key needs a bundle, not just `cargo check`), grant access in
Settings → Recording → Calendar, then record during a meeting.

**Phase 2 (later, not built):** manual event picker (not just re-match);
Granola-style watch + auto-prompt at event time; attendees → "who I met with"
rollups.

## Pipeline plumbing (so calendar data is usable, not just stored)

Decided with Andrew: **don't force calendar data into the raw transcript**
(single source of truth). Instead expose it as placeholders + a free-form save
template.

- **New placeholders** (substituted by `pipeline_engine::render_template`, so
  they work in CLI prompts AND the save template): `{calendar_title}`,
  `{calendar_attendees}` (names, email fallback, comma-separated), `{date}`
  (recording date, local `YYYY-MM-DD`). No `{calendar_date}` — `{date}` covers
  it. Shell steps get the same as env: `NBP_CALENDAR_TITLE`,
  `NBP_CALENDAR_ATTENDEES`, `NBP_DATE`.
- **save_local simplified**: was folder + a "what to save" dropdown
  (transcript / previous result). Now folder + **one free-form Content
  template**, defaulting to `{transcript}`. No toggles — compose the saved note
  however you want (e.g. `# {calendar_title}\n{date} · {calendar_attendees}\n\n{transcript}`).
  The engine already renders `step.template`, so save.rs was untouched.
