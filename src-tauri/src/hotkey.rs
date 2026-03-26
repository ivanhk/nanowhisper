//! Native single-key hotkey monitoring.
//!
//! Supports two modes:
//!   - Toggle: Quick tap (release within trigger_delay_ms) triggers on_tap
//!   - Hold: Hold longer than trigger_delay_ms triggers on_hold_start, release triggers on_hold_end
//!
//! macOS: Uses NSEvent global/local monitors (runs on NSApplication main RunLoop,
//!        immune to App Nap — the OS keeps the app responsive while monitoring).
//! Windows: Uses SetWindowsHookExW low-level keyboard hook.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

/// Debounce interval (ms) to prevent double-fires.
const DEBOUNCE_MS: u64 = 500;

static ON_TAP: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();
static ON_HOLD_START: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();
static ON_HOLD_END: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();
static DEBOUNCE_LAST: AtomicU64 = AtomicU64::new(0);
static PAUSED: AtomicBool = AtomicBool::new(false);
static MONOTONIC_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn trigger_tap() {
    if PAUSED.load(Ordering::SeqCst) {
        return;
    }
    let now = now_ms();
    let last = DEBOUNCE_LAST.load(Ordering::SeqCst);
    if last != 0 && now.saturating_sub(last) < DEBOUNCE_MS {
        return;
    }
    DEBOUNCE_LAST.store(now, Ordering::SeqCst);

    if let Some(cb) = ON_TAP.get() {
        cb();
    }
}

fn trigger_hold_start() {
    if PAUSED.load(Ordering::SeqCst) {
        return;
    }
    if let Some(cb) = ON_HOLD_START.get() {
        cb();
    }
}

fn trigger_hold_end() {
    if PAUSED.load(Ordering::SeqCst) {
        return;
    }
    if let Some(cb) = ON_HOLD_END.get() {
        cb();
    }
}

fn now_ms() -> u64 {
    MONOTONIC_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis() as u64
}

fn get_trigger_delay_ms() -> u64 {
    crate::settings::get_settings().trigger_delay_ms as u64
}

/// Temporarily disable the native hotkey (e.g. while capturing a custom shortcut).
pub fn pause() {
    PAUSED.store(true, Ordering::SeqCst);
}

/// Re-enable the native hotkey.
pub fn resume() {
    PAUSED.store(false, Ordering::SeqCst);
}

/// Start the native hotkey monitor.
/// - on_tap: Called on quick tap (toggle mode)
/// - on_hold_start: Called when held longer than trigger_delay_ms (hold mode)
/// - on_hold_end: Called on release after hold started
pub fn start(
    on_tap: impl Fn() + Send + Sync + 'static,
    on_hold_start: impl Fn() + Send + Sync + 'static,
    on_hold_end: impl Fn() + Send + Sync + 'static,
) {
    let _ = ON_TAP.set(Box::new(on_tap));
    let _ = ON_HOLD_START.set(Box::new(on_hold_start));
    let _ = ON_HOLD_END.set(Box::new(on_hold_end));
    platform::start();
}

// ── macOS: NSEvent global + local monitors ───────────────────────────────────

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use block2::RcBlock;
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use std::ptr::NonNull;

    const NS_FLAGS_CHANGED_MASK: u64 = 1 << 12;
    const NS_KEY_DOWN_MASK: u64 = 1 << 10;
    const K_VK_RIGHT_COMMAND: u16 = 0x36;
    const NS_COMMAND_KEY_MASK: u64 = 1 << 20;

    static KEY_DOWN: AtomicBool = AtomicBool::new(false);
    static KEY_TIME: AtomicU64 = AtomicU64::new(0);
    static OTHER_KEY: AtomicBool = AtomicBool::new(false);
    static HOLD_STARTED: AtomicBool = AtomicBool::new(false);
    static TIMER_CANCELED: AtomicBool = AtomicBool::new(false);

    fn handle_event(event: &AnyObject) {
        let keycode: u16 = unsafe { msg_send![event, keyCode] };
        let flags: u64 = unsafe { msg_send![event, modifierFlags] };

        if keycode == K_VK_RIGHT_COMMAND {
            let cmd_down = (flags & NS_COMMAND_KEY_MASK) != 0;
            if cmd_down {
                if !KEY_DOWN.swap(true, Ordering::SeqCst) {
                    KEY_TIME.store(now_ms(), Ordering::SeqCst);
                    OTHER_KEY.store(false, Ordering::SeqCst);
                    HOLD_STARTED.store(false, Ordering::SeqCst);
                    TIMER_CANCELED.store(false, Ordering::SeqCst);
                    
                    let delay_ms = get_trigger_delay_ms();
                    schedule_hold_check(delay_ms);
                }
            } else if KEY_DOWN.swap(false, Ordering::SeqCst) {
                TIMER_CANCELED.store(true, Ordering::SeqCst);
                
                let held = now_ms().saturating_sub(KEY_TIME.load(Ordering::SeqCst));
                let delay_ms = get_trigger_delay_ms();
                
                if HOLD_STARTED.load(Ordering::SeqCst) {
                    trigger_hold_end();
                } else if !OTHER_KEY.load(Ordering::SeqCst) && held < delay_ms {
                    trigger_tap();
                }
            }
        } else if KEY_DOWN.load(Ordering::SeqCst) {
            OTHER_KEY.store(true, Ordering::SeqCst);
            TIMER_CANCELED.store(true, Ordering::SeqCst);
        }
    }

    fn schedule_hold_check(delay_ms: u64) {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            if TIMER_CANCELED.load(Ordering::SeqCst) {
                return;
            }
            if OTHER_KEY.load(Ordering::SeqCst) {
                return;
            }
            if !KEY_DOWN.load(Ordering::SeqCst) {
                return;
            }
            HOLD_STARTED.store(true, Ordering::SeqCst);
            trigger_hold_start();
        });
    }

    pub fn start() {
        let mask: u64 = NS_FLAGS_CHANGED_MASK | NS_KEY_DOWN_MASK;

        let global_block = RcBlock::new(|event: NonNull<AnyObject>| {
            handle_event(unsafe { event.as_ref() });
        });

        let local_block = RcBlock::new(|event: NonNull<AnyObject>| -> *mut AnyObject {
            handle_event(unsafe { event.as_ref() });
            event.as_ptr()
        });

        unsafe {
            let cls = AnyClass::get(c"NSEvent").expect("NSEvent class not found");

            let _: *mut AnyObject = msg_send![
                cls,
                addGlobalMonitorForEventsMatchingMask: mask,
                handler: &*global_block
            ];

            let _: *mut AnyObject = msg_send![
                cls,
                addLocalMonitorForEventsMatchingMask: mask,
                handler: &*local_block
            ];
        }

        std::mem::forget(global_block);
        std::mem::forget(local_block);

        log::info!("Native hotkey started (Right Command via NSEvent monitors)");
    }
}

// ── Windows: Low-level keyboard hook ─────────────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::ffi::c_void;
    use std::sync::atomic::AtomicPtr;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
        WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    const VK_RCONTROL: u32 = 0xA3;

    static KEY_DOWN: AtomicBool = AtomicBool::new(false);
    static KEY_TIME: AtomicU64 = AtomicU64::new(0);
    static OTHER_KEY: AtomicBool = AtomicBool::new(false);
    static HOLD_STARTED: AtomicBool = AtomicBool::new(false);
    static TIMER_CANCELED: AtomicBool = AtomicBool::new(false);
    static HOOK: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    unsafe extern "system" fn hook_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        if code >= 0 {
            let kbd = *(l_param as *const KBDLLHOOKSTRUCT);
            let vk = kbd.vkCode;
            let is_down = w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize;
            let is_up = w_param == WM_KEYUP as usize || w_param == WM_SYSKEYUP as usize;

            if vk == VK_RCONTROL {
                if is_down && !KEY_DOWN.load(Ordering::SeqCst) {
                    KEY_DOWN.store(true, Ordering::SeqCst);
                    KEY_TIME.store(now_ms(), Ordering::SeqCst);
                    OTHER_KEY.store(false, Ordering::SeqCst);
                    HOLD_STARTED.store(false, Ordering::SeqCst);
                    TIMER_CANCELED.store(false, Ordering::SeqCst);
                    
                    let delay_ms = get_trigger_delay_ms();
                    schedule_hold_check(delay_ms);
                } else if is_up && KEY_DOWN.swap(false, Ordering::SeqCst) {
                    TIMER_CANCELED.store(true, Ordering::SeqCst);
                    
                    let held = now_ms().saturating_sub(KEY_TIME.load(Ordering::SeqCst));
                    let delay_ms = get_trigger_delay_ms();
                    
                    if HOLD_STARTED.load(Ordering::SeqCst) {
                        trigger_hold_end();
                    } else if !OTHER_KEY.load(Ordering::SeqCst) && held < delay_ms {
                        trigger_tap();
                    }
                }
            } else if is_down && KEY_DOWN.load(Ordering::SeqCst) {
                OTHER_KEY.store(true, Ordering::SeqCst);
                TIMER_CANCELED.store(true, Ordering::SeqCst);
            }
        }

        let h = HOOK.load(Ordering::SeqCst);
        unsafe { CallNextHookEx(h, code, w_param, l_param) }
    }

    fn schedule_hold_check(delay_ms: u64) {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            if TIMER_CANCELED.load(Ordering::SeqCst) {
                return;
            }
            if OTHER_KEY.load(Ordering::SeqCst) {
                return;
            }
            if !KEY_DOWN.load(Ordering::SeqCst) {
                return;
            }
            HOLD_STARTED.store(true, Ordering::SeqCst);
            trigger_hold_start();
        });
    }

    pub fn start() {
        std::thread::spawn(|| unsafe {
            let hmod = GetModuleHandleW(std::ptr::null());
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(hook_proc),
                hmod,
                0,
            );
            if hook.is_null() {
                log::error!("Failed to install keyboard hook");
                return;
            }
            HOOK.store(hook, Ordering::SeqCst);
            log::info!("Native hotkey started (Right Control)");

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
        });
    }
}

// ── Linux: no-op (use global_shortcut fallback) ──────────────────────────────

#[cfg(target_os = "linux")]
mod platform {
    pub fn start() {
        log::info!("Native hotkey not available on Linux; use global shortcut");
    }
}