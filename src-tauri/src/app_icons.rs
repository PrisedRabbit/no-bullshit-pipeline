//! Resolves an application icon (PNG, base64 data URL) from a bundle id.
//!
//! macOS can hand us the icon of any *installed* app via NSWorkspace, even when
//! the app isn't running — so an auto-recorded call keeps its icon in the list
//! long after the call (and the app) is gone. We only need the bundle id, which
//! `audio_process_detector` captures at detection time and stores in the
//! recording metadata.
//!
//! Two-tier cache:
//!   • in-memory `HashMap<bundle_id, data_url>` — avoids repeated FFI within a
//!     session (the list re-renders a lot).
//!   • on-disk `{data_dir}/app_icons/{sanitized}.png` — survives restarts so we
//!     don't re-render the NSImage every launch.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;

use base64::Engine;

static MEM_CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

/// Tauri command: returns a `data:image/png;base64,...` URL for the given bundle
/// id, or `None` if the app can't be resolved (frontend then shows a neutral
/// default glyph). Result is cached in memory and on disk.
#[tauri::command]
pub fn get_app_icon(bundle_id: String) -> Option<String> {
    if bundle_id.is_empty() {
        return None;
    }

    // 1. Memory cache.
    if let Ok(mut guard) = MEM_CACHE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        if let Some(hit) = map.get(&bundle_id) {
            return hit.clone();
        }
    }

    // 2. Disk cache.
    let cache_path = icon_cache_path(&bundle_id);
    if let Some(ref path) = cache_path {
        if path.exists() {
            if let Ok(bytes) = std::fs::read(path) {
                let url = png_to_data_url(&bytes);
                store_mem(&bundle_id, Some(url.clone()));
                return Some(url);
            }
        }
    }

    // 3. Resolve from the OS, render to PNG, persist.
    let png = render_icon_png(&bundle_id);
    log::info!(
        "app_icons: resolve bundle={:?} -> {}",
        bundle_id,
        png.as_ref().map(|b| format!("{} bytes", b.len())).unwrap_or_else(|| "UNRESOLVED".to_string())
    );
    if let Some(ref bytes) = png {
        if let Some(ref path) = cache_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, bytes);
        }
    }
    let result = png.map(|b| png_to_data_url(&b));
    store_mem(&bundle_id, result.clone());
    result
}

fn store_mem(bundle_id: &str, value: Option<String>) {
    if let Ok(mut guard) = MEM_CACHE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(bundle_id.to_string(), value);
    }
}

fn png_to_data_url(bytes: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:image/png;base64,{}", b64)
}

fn icon_cache_path(bundle_id: &str) -> Option<std::path::PathBuf> {
    let safe: String = bundle_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    Some(crate::storage::get_data_dir().join("app_icons").join(format!("{}.png", safe)))
}

// --- AppKit FFI --------------------------------------------------------------
//
// Raw msg_send (same style as audio_process_detector) to avoid pulling in the
// full objc2-app-kit typed surface. All AppKit calls return autoreleased
// objects, so everything runs inside an autoreleasepool.

/// NSBitmapImageFileType.png == 4 (AppKit, stable since 10.0).
const NS_BITMAP_FILE_TYPE_PNG: u64 = 4;

/// Target raster size for app icons — 64px covers a 16px display at up to @3x.
const ICON_RENDER_PX: f64 = 64.0;

// CoreGraphics geometry structs (NSPoint/NSSize/NSRect are these on 64-bit).
// Defined locally with Encode impls so we can pass an explicit destination rect
// to CGImageForProposedRect without pulling in objc2-app-kit / objc2-core-foundation.
#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}
unsafe impl objc2::encode::Encode for CGPoint {
    const ENCODING: objc2::encode::Encoding =
        objc2::encode::Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}
unsafe impl objc2::encode::Encode for CGSize {
    const ENCODING: objc2::encode::Encoding =
        objc2::encode::Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}
unsafe impl objc2::encode::Encode for CGRect {
    const ENCODING: objc2::encode::Encoding =
        objc2::encode::Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
}
unsafe impl objc2::encode::RefEncode for CGRect {
    const ENCODING_REF: objc2::encode::Encoding =
        objc2::encode::Encoding::Pointer(&<Self as objc2::encode::Encode>::ENCODING);
}

fn render_icon_png(bundle_id: &str) -> Option<Vec<u8>> {
    use objc2::rc::autoreleasepool;
    use objc2::runtime::{AnyClass, AnyObject};

    autoreleasepool(|_| unsafe {
        let ns_workspace = AnyClass::get(c"NSWorkspace")?;
        let workspace: *mut AnyObject = objc2::msg_send![ns_workspace, sharedWorkspace];
        if workspace.is_null() {
            return None;
        }

        let bundle_ns = nsstring(bundle_id)?;
        let url: *mut AnyObject =
            objc2::msg_send![workspace, URLForApplicationWithBundleIdentifier: bundle_ns];
        if url.is_null() {
            return None;
        }
        let path: *mut AnyObject = objc2::msg_send![url, path];
        if path.is_null() {
            return None;
        }

        // NSImage* icon = [workspace iconForFile:path];
        let icon: *mut AnyObject = objc2::msg_send![workspace, iconForFile: path];
        if icon.is_null() {
            return None;
        }

        // Flatten the NSImage to a CGImage at an explicit size. macOS 26 (Tahoe)
        // ships app icons as dynamic `.icon` resources: `TIFFRepresentation`
        // returns blank, and `CGImageForProposedRect` with a NULL rect doesn't
        // know what size to rasterize so it also yields nothing. Setting the
        // image size AND passing a concrete destination rect forces a real
        // raster of the icon's current rendering — works on macOS 15 and 26.
        let target = CGSize {
            width: ICON_RENDER_PX,
            height: ICON_RENDER_PX,
        };
        let _: () = objc2::msg_send![icon, setSize: target];
        let mut rect = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: target,
        };
        let cg_image: *mut std::ffi::c_void = objc2::msg_send![
            icon,
            CGImageForProposedRect: &mut rect as *mut CGRect,
            context: std::ptr::null::<AnyObject>(),
            hints: std::ptr::null::<AnyObject>()
        ];
        if cg_image.is_null() {
            return None;
        }

        // NSBitmapImageRep* rep = [[NSBitmapImageRep alloc] initWithCGImage:cgImage];
        let ns_rep = AnyClass::get(c"NSBitmapImageRep")?;
        let rep_alloc: *mut AnyObject = objc2::msg_send![ns_rep, alloc];
        let rep: *mut AnyObject = objc2::msg_send![rep_alloc, initWithCGImage: cg_image];
        if rep.is_null() {
            return None;
        }

        // NSData* png = [rep representationUsingType:NSPNGFileType properties:@{}];
        let ns_dict = AnyClass::get(c"NSDictionary")?;
        let empty: *mut AnyObject = objc2::msg_send![ns_dict, dictionary];
        let png: *mut AnyObject = objc2::msg_send![
            rep,
            representationUsingType: NS_BITMAP_FILE_TYPE_PNG,
            properties: empty
        ];
        if png.is_null() {
            return None;
        }

        nsdata_to_vec(png)
    })
}

/// Build an autoreleased NSString from a Rust &str.
unsafe fn nsstring(s: &str) -> Option<*mut objc2::runtime::AnyObject> {
    use objc2::runtime::{AnyClass, AnyObject};
    let cls = AnyClass::get(c"NSString")?;
    let cstr = std::ffi::CString::new(s).ok()?;
    let obj: *mut AnyObject = objc2::msg_send![cls, stringWithUTF8String: cstr.as_ptr()];
    if obj.is_null() {
        None
    } else {
        Some(obj)
    }
}

/// Copy the bytes out of an NSData into an owned Vec (before the pool drains).
unsafe fn nsdata_to_vec(data: *mut objc2::runtime::AnyObject) -> Option<Vec<u8>> {
    let len: usize = objc2::msg_send![data, length];
    if len == 0 {
        return None;
    }
    let bytes: *const c_void = objc2::msg_send![data, bytes];
    if bytes.is_null() {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(bytes as *const u8, len) };
    Some(slice.to_vec())
}
