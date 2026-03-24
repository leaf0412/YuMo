//! macOS native hotkey listener via CGEventTap.
//!
//! Runs on a dedicated background thread. Hot-path reads use Atomics;
//! only callback dispatch acquires a Mutex.

use core_foundation::base::TCFType;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use super::{HotkeyCallback, NO_KEYCODE};

// CGEvent type constants
const KCG_EVENT_KEY_DOWN: u32 = 10;
const KCG_EVENT_KEY_UP: u32 = 11;
const KCG_EVENT_FLAGS_CHANGED: u32 = 12;
const KCG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFFFFFE;

// CGEventField
const KCG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

// Device-dependent modifier flags (IOKit/IOLLEvent.h NX_DEVICE* series)
// These distinguish left from right modifier keys
const NX_DEVICELSHIFTKEYMASK: u64 = 0x00000002;
const NX_DEVICERSHIFTKEYMASK: u64 = 0x00000004;
const NX_DEVICELCTLKEYMASK: u64 = 0x00000001;
const NX_DEVICERCTLKEYMASK: u64 = 0x00002000;
const NX_DEVICELALTKEYMASK: u64 = 0x00000020;
const NX_DEVICERALTKEYMASK: u64 = 0x00000040;
const NX_DEVICELCMDKEYMASK: u64 = 0x00000008;
const NX_DEVICERCMDKEYMASK: u64 = 0x00000010;
const KCG_EVENT_FLAG_SECONDARY_FN: u64 = 0x00800000;

const ESCAPE_KEYCODE: u16 = 0x35;

/// Map modifier keyCode to its device-dependent flag bit (distinguishes left/right)
fn device_flag_for_keycode(keycode: u16) -> Option<u64> {
    match keycode {
        0x38 => Some(NX_DEVICELSHIFTKEYMASK),
        0x3C => Some(NX_DEVICERSHIFTKEYMASK),
        0x3B => Some(NX_DEVICELCTLKEYMASK),
        0x3E => Some(NX_DEVICERCTLKEYMASK),
        0x3A => Some(NX_DEVICELALTKEYMASK),
        0x3D => Some(NX_DEVICERALTKEYMASK),
        0x37 => Some(NX_DEVICELCMDKEYMASK),
        0x36 => Some(NX_DEVICERCMDKEYMASK),
        0x3F => Some(KCG_EVENT_FLAG_SECONDARY_FN),
        _ => None,
    }
}

// FFI types
type CGEventTapLocation = u32;
type CGEventTapPlacement = u32;
type CGEventTapOptions = u32;
type CGEventMask = u64;
type CGEventRef = *mut std::ffi::c_void;
type CFMachPortRef = *mut std::ffi::c_void;

const KCG_SESSION_EVENT_TAP: CGEventTapLocation = 1;
const KCG_HEAD_INSERT_EVENT_TAP: CGEventTapPlacement = 0;
const KCG_EVENT_TAP_OPTION_LISTEN_ONLY: CGEventTapOptions = 1;

type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: *mut std::ffi::c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut std::ffi::c_void,
) -> CGEventRef;

extern "C" {
    fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOptions,
        events_of_interest: CGEventMask,
        callback: CGEventTapCallBack,
        user_info: *mut std::ffi::c_void,
    ) -> CFMachPortRef;
    fn CFMachPortCreateRunLoopSource(
        allocator: *const std::ffi::c_void,
        port: CFMachPortRef,
        order: i64,
    ) -> core_foundation::runloop::CFRunLoopSourceRef;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
}

/// Context passed to the CGEventTap callback via raw pointer.
///
/// Intentionally leaked (process-level lifetime): the CFRunLoop blocks forever,
/// so this struct lives as long as the process. If stop() support is added later,
/// store the raw pointer and reclaim via Box::from_raw.
struct TapContext {
    target_keycode: Arc<AtomicU16>,
    target_is_modifier: Arc<AtomicBool>,
    on_hotkey: Arc<Mutex<Option<HotkeyCallback>>>,
    on_escape: Arc<Mutex<Option<HotkeyCallback>>>,
    modifier_pressed: AtomicBool,
    tap_ref: CFMachPortRef,
}

// SAFETY: TapContext is only accessed from the event-tap thread's callback,
// which is single-threaded (one CFRunLoop). The Arc fields handle cross-thread sharing.
unsafe impl Send for TapContext {}
unsafe impl Sync for TapContext {}

/// Wrapper to send a raw pointer across thread boundary.
/// SAFETY: The pointee (TapContext) is Send+Sync and lives for the process lifetime.
struct SendPtr(*mut TapContext);
unsafe impl Send for SendPtr {}

unsafe extern "C" fn tap_callback(
    _proxy: *mut std::ffi::c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut std::ffi::c_void,
) -> CGEventRef {
    let ctx = &*(user_info as *const TapContext);

    // Re-enable tap if system disabled it (e.g., callback took too long)
    if event_type == KCG_EVENT_TAP_DISABLED_BY_TIMEOUT
        || event_type > KCG_EVENT_FLAGS_CHANGED
    {
        warn!("[hotkey/macos] event tap disabled by system, re-enabling");
        CGEventTapEnable(ctx.tap_ref, true);
        return event;
    }

    let keycode = CGEventGetIntegerValueField(event, KCG_KEYBOARD_EVENT_KEYCODE) as u16;
    let flags = CGEventGetFlags(event);

    // 1. Check Escape (fires on keyDown)
    if event_type == KCG_EVENT_KEY_DOWN && keycode == ESCAPE_KEYCODE {
        if let Ok(guard) = ctx.on_escape.lock() {
            if let Some(ref cb) = *guard {
                cb();
            }
        }
    }

    // 2. Check main hotkey (Atomic reads, no locks on hot path)
    let target_kc = ctx.target_keycode.load(Ordering::Relaxed);
    if target_kc == NO_KEYCODE {
        return event;
    }

    let is_modifier = ctx.target_is_modifier.load(Ordering::Relaxed);

    if is_modifier
        && event_type == KCG_EVENT_FLAGS_CHANGED
        && keycode == target_kc
    {
        // Modifier key: use device-dependent flags to distinguish left/right
        let flag_bit = device_flag_for_keycode(keycode).unwrap_or(0);
        let is_pressed = (flags & flag_bit) != 0;
        let was_pressed = ctx.modifier_pressed.load(Ordering::Relaxed);

        if is_pressed && !was_pressed {
            ctx.modifier_pressed.store(true, Ordering::Relaxed);
            if let Ok(guard) = ctx.on_hotkey.lock() {
                if let Some(ref cb) = *guard {
                    cb();
                }
            }
        } else if !is_pressed && was_pressed {
            ctx.modifier_pressed.store(false, Ordering::Relaxed);
        }
    } else if !is_modifier
        && event_type == KCG_EVENT_KEY_DOWN
        && keycode == target_kc
    {
        // Normal key: fire on keyDown
        if let Ok(guard) = ctx.on_hotkey.lock() {
            if let Some(ref cb) = *guard {
                cb();
            }
        }
    }

    event
}

/// Start the CGEventTap on a dedicated background thread.
///
/// The tap listens for keyDown, keyUp, and flagsChanged events.
/// It runs in listen-only mode (does not consume/modify events).
/// Requires Accessibility permission.
pub fn start_event_tap(
    target_keycode: Arc<AtomicU16>,
    target_is_modifier: Arc<AtomicBool>,
    on_hotkey: Arc<Mutex<Option<HotkeyCallback>>>,
    on_escape: Arc<Mutex<Option<HotkeyCallback>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Box::new(TapContext {
        target_keycode,
        target_is_modifier,
        on_hotkey,
        on_escape,
        modifier_pressed: AtomicBool::new(false),
        tap_ref: std::ptr::null_mut(), // filled after CGEventTapCreate
    });
    let ctx_ptr = SendPtr(Box::into_raw(ctx));

    std::thread::Builder::new()
        .name("hotkey-event-tap".into())
        .spawn(move || {
            // Force capture of entire SendPtr (Rust 2021 would capture .0 field only)
            let send_ptr = ctx_ptr;
            let ctx_ptr = send_ptr.0;
            unsafe {
                let event_mask: CGEventMask = (1 << KCG_EVENT_KEY_DOWN)
                    | (1 << KCG_EVENT_KEY_UP)
                    | (1 << KCG_EVENT_FLAGS_CHANGED);

                let tap = CGEventTapCreate(
                    KCG_SESSION_EVENT_TAP,
                    KCG_HEAD_INSERT_EVENT_TAP,
                    KCG_EVENT_TAP_OPTION_LISTEN_ONLY,
                    event_mask,
                    tap_callback,
                    ctx_ptr as *mut std::ffi::c_void,
                );

                if tap.is_null() {
                    error!(
                        "[hotkey/macos] CGEventTapCreate failed \
                         — check Accessibility permission"
                    );
                    let _ = Box::from_raw(ctx_ptr);
                    return;
                }

                // Back-fill tap_ref so callback can re-enable if system disables it
                (*ctx_ptr).tap_ref = tap;

                let source_ref = CFMachPortCreateRunLoopSource(
                    std::ptr::null(),
                    tap,
                    0,
                );
                if source_ref.is_null() {
                    error!("[hotkey/macos] CFMachPortCreateRunLoopSource failed");
                    let _ = Box::from_raw(ctx_ptr);
                    return;
                }

                let source =
                    CFRunLoopSource::wrap_under_create_rule(source_ref);

                let run_loop = CFRunLoop::get_current();
                run_loop.add_source(&source, kCFRunLoopCommonModes);

                CGEventTapEnable(tap, true);
                info!("[hotkey/macos] CGEventTap started, entering run loop");
                // CFRunLoop::run_current() blocks forever — TapContext intentionally leaked
                CFRunLoop::run_current();
            }
        })?;

    Ok(())
}
