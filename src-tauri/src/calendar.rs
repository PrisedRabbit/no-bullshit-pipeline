//! Calendar matching via EventKit — read-only, no OAuth.
//!
//! When a recording finalizes we look for a calendar event around its start
//! time and, on a hit, stamp the event's attendees onto the recording and
//! adopt the event title (if the recording still carries its auto-generated
//! default). The macOS EventStore reads every account the user has synced to
//! Calendar.app (Google / iCloud / Exchange) — NBP never touches OAuth; the OS
//! owns the account. The user connects one in System Settings → Internet
//! Accounts; access is gated by a single TCC prompt
//! (`NSCalendarsFullAccessUsageDescription`).
//!
//! Silent by design: matching only runs when access is already granted and
//! never prompts on its own (the prompt is a deliberate, user-initiated action
//! from Settings → Recording → Calendar).

use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEvent, EKEventStore, EKParticipant};
use objc2_foundation::{NSDate, NSError};

use crate::storage::Attendee;

/// Padding (seconds) added to each side of the recording span when querying the
/// calendar, so events that start a touch before / after the recording still
/// come back from EventKit; the precise overlap rule is applied in Rust below.
const QUERY_PAD_SECS: f64 = 30.0 * 60.0;
/// Minimum overlap (seconds) for an event to count as a match — filters out a
/// recording that merely clips the tail/head of an adjacent event. Capped at
/// the recording's own length, so a short recording fully inside a meeting
/// still matches.
const MIN_OVERLAP_SECS: f64 = 2.0 * 60.0;

/// A matched calendar event, reduced to plain (Send) data.
pub struct MatchedEvent {
    pub id: Option<String>,
    pub title: String,
    pub attendees: Vec<Attendee>,
}

/// macOS major version (e.g. 14, 15, 26). `requestFullAccessToEvents` is 14+.
fn os_major() -> i64 {
    use objc2_foundation::NSProcessInfo;
    let info = NSProcessInfo::processInfo();
    info.operatingSystemVersion().majorVersion as i64
}

/// Non-prompting status check. True once the user has granted full access.
/// (The legacy `Authorized` value shares `FullAccess`'s discriminant, so this
/// also covers pre-14 grants.)
pub fn has_full_access() -> bool {
    let status = unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
    matches!(status, EKAuthorizationStatus::FullAccess)
}

/// Prompt for full calendar access (macOS 14+) and block until the user
/// answers. The completion handler fires on an arbitrary GCD queue, so we
/// bounce the result back over a channel and wait on it here. Safe to run on a
/// Tauri command worker thread (EventKit doesn't require the main thread).
pub fn request_full_access() -> Result<bool, String> {
    if os_major() < 14 {
        return Err("Calendar matching requires macOS 14 or later.".to_string());
    }

    let store = unsafe { EKEventStore::new() };
    let (tx, rx) = mpsc::channel::<Result<bool, String>>();

    // RcBlock (heap, retain-counted): the block outlives this call and fires
    // later off-thread, so a StackBlock would be a use-after-free.
    let block = RcBlock::new(move |granted: Bool, err: *mut NSError| {
        if !err.is_null() {
            // SAFETY: non-null NSError owned by the framework for this call.
            let err: &NSError = unsafe { &*err };
            let _ = tx.send(Err(err.localizedDescription().to_string()));
        } else {
            let _ = tx.send(Ok(granted.as_bool()));
        }
    });

    // SAFETY: generated `unsafe` FFI; the completion typedef is a raw block
    // pointer. `block` is kept alive on this stack until recv() returns.
    unsafe {
        store.requestFullAccessToEventsWithCompletion(RcBlock::as_ptr(&block));
    }

    match rx.recv_timeout(Duration::from_secs(60)) {
        Ok(Ok(granted)) => Ok(granted),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Timed out waiting for the calendar permission dialog.".to_string()),
    }
}

/// Overlap (seconds) between a recording span and an event, or `None` if it
/// doesn't clear the bar. The bar is `MIN_OVERLAP_SECS`, capped at the
/// recording's own length — so a 15-min recording fully inside a 1-hour meeting
/// still qualifies (its whole length overlaps), while a long recording that
/// merely clips a neighbouring event by seconds does not. Pure (no FFI) so the
/// matching math is unit-tested without EventKit.
fn qualifying_overlap(rec_start: f64, rec_end: f64, ev_start: f64, ev_end: f64) -> Option<f64> {
    let overlap = ev_end.min(rec_end) - ev_start.max(rec_start);
    let min_overlap = MIN_OVERLAP_SECS.min(rec_end - rec_start).max(1.0);
    (overlap >= min_overlap).then_some(overlap)
}

/// Find the calendar event that best overlaps a recording spanning
/// `[rec_start, rec_end]` (Unix epoch seconds). "Best" = most overlap, ties
/// broken by closest start. Returns `None` when access isn't granted (silent)
/// or nothing overlaps. The store is created, used, and dropped here —
/// `EKEventStore` is `!Send`, so this must run start-to-finish on one thread.
pub fn match_event_for(rec_start: f64, rec_end: f64) -> Option<MatchedEvent> {
    if !has_full_access() {
        return None;
    }
    // Guard against a zero / unknown duration: treat the recording as at least
    // a 1-second span so containment still yields a positive overlap.
    let rec_end = rec_end.max(rec_start + 1.0);

    let store = unsafe { EKEventStore::new() };

    let win_start = NSDate::dateWithTimeIntervalSince1970(rec_start - QUERY_PAD_SECS);
    let win_end = NSDate::dateWithTimeIntervalSince1970(rec_end + QUERY_PAD_SECS);

    // calendars = None → all calendars the user has synced.
    let predicate = unsafe {
        store.predicateForEventsWithStartDate_endDate_calendars(&win_start, &win_end, None)
    };
    let events = unsafe { store.eventsMatchingPredicate(&predicate) };

    // Track (overlap_secs, start_diff, event). Most overlap wins; ties → the
    // event whose start is closest to the recording start.
    let mut best: Option<(f64, f64, MatchedEvent)> = None;
    for event in events.iter() {
        // All-day events (vacations, birthdays) span the whole day and would
        // spuriously overlap any same-day recording — skip them.
        if unsafe { event.isAllDay() } {
            continue;
        }
        let ev_start = unsafe { event.startDate() }.timeIntervalSince1970();
        let ev_end = unsafe { event.endDate() }.timeIntervalSince1970();

        let Some(overlap) = qualifying_overlap(rec_start, rec_end, ev_start, ev_end) else {
            continue;
        };

        let diff = (rec_start - ev_start).abs();
        let better = match &best {
            None => true,
            Some((bo, bd, _)) => overlap > *bo || (overlap == *bo && diff < *bd),
        };
        if better {
            let title = unsafe { event.title() }.to_string();
            let id = unsafe { event.eventIdentifier() }.map(|s| s.to_string());
            let attendees = attendees_of(&event);
            best = Some((overlap, diff, MatchedEvent { id, title, attendees }));
        }
    }

    best.map(|(_, _, m)| m)
}

/// Best available recording duration (seconds) from whichever audio track was
/// kept. Falls back to 0 (caller's span guard handles it).
fn meta_duration(meta: &crate::storage::RecordingMetadata) -> f64 {
    let a = &meta.audio;
    a.mix
        .as_ref()
        .or(a.mic.as_ref())
        .or(a.system.as_ref())
        .map(|i| i.duration_sec)
        .unwrap_or(0.0)
}

/// Extract attendees from an event. EventKit has no email accessor — the
/// address lives in the participant's `mailto:` URL — so both fields are
/// optional and we skip participants that carry neither.
fn attendees_of(event: &EKEvent) -> Vec<Attendee> {
    let participants = match unsafe { event.attendees() } {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    let mut out = Vec::with_capacity(participants.len());
    for p in participants.iter() {
        let name = unsafe { p.name() }.map(|n| n.to_string());
        let email = email_of(&p);
        if name.is_none() && email.is_none() {
            continue;
        }
        out.push(Attendee { name, email });
    }
    out
}

/// Pull the email out of a participant's `mailto:` URL, tolerating query
/// params (`mailto:a@b?subject=…`). `None` for resource/room participants.
fn email_of(p: &EKParticipant) -> Option<String> {
    let url = unsafe { p.URL() };
    let s = url.absoluteString()?.to_string();
    let rest = s.strip_prefix("mailto:")?;
    let addr = rest.split('?').next().unwrap_or(rest).trim();
    if addr.is_empty() {
        None
    } else {
        Some(addr.to_string())
    }
}

/// Parse a recording's stored RFC3339 `created_at` into Unix epoch seconds.
pub fn parse_rfc3339_unix(created_at: &str) -> Option<f64> {
    chrono::DateTime::parse_from_rfc3339(created_at)
        .ok()
        .map(|dt| dt.timestamp() as f64)
}

/// Whether a title is one of NBP's auto-generated defaults (so calendar
/// matching may replace it). Covers "NBP · HH:MM" / "{App} · HH:MM" (manual /
/// call), the empty / "Untitled Recording" / "Recording …" forms — but never a
/// title the user has typed.
pub fn is_default_title(title: &str) -> bool {
    let t = title.trim();
    if t.is_empty()
        || t == "Untitled Recording"
        || t.starts_with("Recording ")
        || t.starts_with("NBP · ")
    {
        return true;
    }
    // Trailing " · HH:MM" marks the auto title for any app label.
    if let Some((_, tail)) = t.rsplit_once(" · ") {
        let b = tail.as_bytes();
        if tail.len() == 5
            && b[2] == b':'
            && b[0].is_ascii_digit()
            && b[1].is_ascii_digit()
            && b[3].is_ascii_digit()
            && b[4].is_ascii_digit()
        {
            return true;
        }
    }
    false
}

// ===== Tauri commands =====

/// Prompt for calendar access (Settings → Recording → Calendar). Caches the
/// result on the shared permissions state so `check_permissions` reflects it.
#[tauri::command]
pub fn request_calendar_permission(
    state: tauri::State<'_, crate::permissions::PermissionsStateCache>,
) -> Result<bool, String> {
    let granted = request_full_access()?;
    if let Ok(mut cache) = state.0.lock() {
        cache.calendar = granted;
    }
    Ok(granted)
}

/// Re-run the time-based match for one recording (user action from the detail
/// view). Errors with a clear message when nothing lines up, leaving metadata
/// untouched so the UI can surface "no event found".
#[tauri::command]
pub fn rematch_calendar(recording_id: String) -> Result<crate::storage::RecordingMetadata, String> {
    if !has_full_access() {
        return Err("Calendar access isn't granted. Enable it in Settings → Recording.".to_string());
    }
    let mut meta = crate::storage::read_metadata(&recording_id)?;
    let start = parse_rfc3339_unix(&meta.created_at)
        .ok_or_else(|| "Couldn't parse this recording's start time.".to_string())?;
    let end = start + meta_duration(&meta);

    match match_event_for(start, end) {
        Some(m) => {
            if is_default_title(&meta.title) && !m.title.is_empty() {
                meta.title = m.title;
            }
            meta.calendar_event_id = m.id;
            meta.attendees = m.attendees;
            meta.calendar_matched = true;
            crate::storage::write_metadata(&meta)?;
            Ok(meta)
        }
        None => Err("No calendar event found around this recording's start time.".to_string()),
    }
}

/// Drop a recording's calendar association (attendees + event id). Leaves the
/// title as-is — we don't store the pre-match default to restore.
#[tauri::command]
pub fn clear_calendar_match(
    recording_id: String,
) -> Result<crate::storage::RecordingMetadata, String> {
    let mut meta = crate::storage::read_metadata(&recording_id)?;
    meta.attendees = vec![];
    meta.calendar_event_id = None;
    meta.calendar_matched = false;
    crate::storage::write_metadata(&meta)?;
    Ok(meta)
}

/// Deep-link to the Calendar privacy pane so the user can flip access in
/// System Settings.
#[tauri::command]
pub fn open_calendar_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars")
        .spawn();
}

/// Best-effort calendar match during recording finalize. Silent: no prompt, no
/// error surfacing — on no permission / no event it simply leaves the metadata
/// unchanged. Mutates `meta` in place; the caller writes it. `duration_sec` is
/// the just-finished recording's length, used to match by span overlap.
pub fn try_match_on_finalize(meta: &mut crate::storage::RecordingMetadata, duration_sec: f64) {
    // Recordings only (dictation never reaches finalize_recording, but guard
    // anyway), and don't re-match what's already matched.
    if meta.source == "dictation" || meta.calendar_matched {
        return;
    }
    let Some(start) = parse_rfc3339_unix(&meta.created_at) else {
        return;
    };
    if let Some(m) = match_event_for(start, start + duration_sec) {
        if is_default_title(&meta.title) && !m.title.is_empty() {
            meta.title = m.title;
        }
        meta.calendar_event_id = m.id;
        meta.attendees = m.attendees;
        meta.calendar_matched = true;
        log::info!(
            "[calendar] matched recording {} → \"{}\" ({} attendees)",
            meta.id,
            meta.title,
            meta.attendees.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_titles_are_replaceable() {
        assert!(is_default_title(""));
        assert!(is_default_title("Untitled Recording"));
        assert!(is_default_title("Recording 2026-06-03"));
        assert!(is_default_title("NBP · 09:05"));
        assert!(is_default_title("Zoom · 14:30"));
        assert!(is_default_title("FaceTime · 23:59"));
    }

    #[test]
    fn user_titles_are_preserved() {
        assert!(!is_default_title("Q3 Planning"));
        assert!(!is_default_title("1:1 with Sam"));
        assert!(!is_default_title("Standup · notes")); // tail isn't HH:MM
        assert!(!is_default_title("Sync · 9:5")); // not zero-padded HH:MM
    }

    #[test]
    fn parses_rfc3339() {
        let ts = parse_rfc3339_unix("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(ts, 1705314600.0);
        assert!(parse_rfc3339_unix("not a date").is_none());
    }

    // Times in minutes from an arbitrary epoch, scaled to seconds.
    fn m(min: f64) -> f64 {
        min * 60.0
    }

    #[test]
    fn short_recording_fully_inside_long_event_matches() {
        // 15-min recording inside a 1-hour meeting (the "did the hour meeting
        // in 15 min" case) — its whole length overlaps, clears the capped bar.
        let got = qualifying_overlap(m(10.0), m(25.0), m(0.0), m(60.0));
        assert_eq!(got, Some(m(15.0)));
    }

    #[test]
    fn tiny_recording_inside_event_matches() {
        // 1-min note inside the meeting — bar is capped at 1 min, overlap = 1 min.
        assert_eq!(qualifying_overlap(m(30.0), m(31.0), m(0.0), m(60.0)), Some(m(1.0)));
    }

    #[test]
    fn clipping_a_neighbour_by_seconds_does_not_match() {
        // Hour-long recording that overlaps an adjacent event by only 30s.
        // rec [0,60min], event [59.5min, 90min] → overlap 30s < 2-min bar.
        assert!(qualifying_overlap(m(0.0), m(60.0), m(59.5), m(90.0)).is_none());
    }

    #[test]
    fn no_overlap_does_not_match() {
        assert!(qualifying_overlap(m(0.0), m(10.0), m(20.0), m(50.0)).is_none());
    }

    #[test]
    fn normal_overlap_matches() {
        // Started 6 min early, recorded 50 min into a 1-hour meeting.
        let got = qualifying_overlap(m(-6.0), m(44.0), m(0.0), m(60.0));
        assert_eq!(got, Some(m(44.0)));
    }
}
