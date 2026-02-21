use crate::types::{BreakpointRange, LocationInfo, WorkerOut};
use js_sys::SharedArrayBuffer;
use std::cell::RefCell;

// Thread-local storage for the global debugger instance
thread_local! {
    static DEBUGGER: RefCell<Option<Debugger>> = RefCell::new(None);
}

/// Debugger state that manages breakpoint locations and their enable/disable state.
///
/// The breakpoint buffer is a SharedArrayBuffer with two purposes:
///
/// **Index 0 — Pause/Resume Signal (Sentinel)**
/// - When a breakpoint is hit, the Rust side calls `Atomics.wait()` on index 0
/// - This blocks execution until TypeScript calls `Atomics.notify()` on index 0
/// - Value doesn't matter; we just wait for the notification
///
/// **Index 1..N — Breakpoint Enable/Disable Flags**
/// - Index N corresponds to `locations[N-1]`
/// - Value 0 = breakpoint disabled
/// - Value 1 = breakpoint enabled
pub struct Debugger {
    locations: Vec<LocationInfo>,
    ranges: Vec<BreakpointRange>,
    files: Vec<String>,
    buffer: SharedArrayBuffer,
}

impl Debugger {
    pub fn new(locations: Vec<LocationInfo>, ranges: Vec<BreakpointRange>, files: Vec<String>) -> Self {
        let max_bkpt = ranges.iter().map(|r| r.bkpt).max().unwrap_or(0);
        let slots = max_bkpt + 1; // slot 0 sentinel, 1..N breakpoints
        let buffer_size = slots * 4; // Int32Array backing store
        let buffer = SharedArrayBuffer::new(buffer_size);

        Self {
            locations,
            ranges,
            files,
            buffer,
        }
    }

    pub fn buffer(&self) -> &SharedArrayBuffer {
        &self.buffer
    }

    pub fn locations(&self) -> &[LocationInfo] {
        &self.locations
    }

    pub fn files(&self) -> &[String] {
        &self.files
    }

    pub fn send_debug_info(&self) {
        WorkerOut::Debug {
            locations: self.locations.clone(),
            ranges: self.ranges.clone(),
            files: self.files.clone(),
            breakpoint_buffer: self.buffer.clone(),
        }
        .send();
    }

    /// Check if a breakpoint at the given index is enabled.
    ///
    /// This reads from the SharedArrayBuffer using atomic operations.
    /// Returns false for index 0 (sentinel) or out-of-bounds indices.
    pub fn bkpt_enabled(&self, index: u32) -> bool {
        let max_bkpt = self.ranges.iter().map(|r| r.bkpt).max().unwrap_or(0);
        if index == 0 || index > max_bkpt {
            return false;
        }

        let view = js_sys::Int32Array::new(&self.buffer);
        let value = js_sys::Atomics::load(&view, index).unwrap_or(0);
        value != 0
    }

    /// Called when a breakpoint is hit. Blocks until TypeScript signals resume.
    ///
    /// This waits on index 0 (the sentinel) using `Atomics.wait()`.
    /// TypeScript will call `Atomics.notify()` on index 0 when the user
    /// wants to resume execution.
    ///
    /// The `expected_value` parameter is the current value at index 0.
    /// If TypeScript has already changed it, the wait returns immediately.
    pub fn wait_for_resume(&self) {
        let view = js_sys::Int32Array::new(&self.buffer);
        let current = js_sys::Atomics::load(&view, 0).unwrap_or(0);
        // Wait until TypeScript notifies us
        let _ = js_sys::Atomics::wait(&view, 0, current);
    }

    /// Check if breakpoint is enabled, and if so, wait for resume.
    ///
    /// This is the main entry point called from instrumented WASM code.
    pub fn bkpt(&self, index: u32) -> bool {
        if !self.bkpt_enabled(index) {
            return false;
        }

        // TODO: Send BreakpointHit message to TypeScript with stack info

        // TODO: remove after debugging
        web_sys::console::log_1(&format!("Breakpoint hit at index {}", index).into());

        self.wait_for_resume();
        true
    }

    /// Set the global debugger instance.
    /// Call this before running instrumented code.
    pub fn set_global(debugger: Debugger) {
        DEBUGGER.with(|d| *d.borrow_mut() = Some(debugger));
    }

    /// Handle a breakpoint hit from WASM import.
    /// This is the function provided as the "debug"."bkpt" import.
    pub fn handle_bkpt(index: i32) {
        DEBUGGER.with(|d| {
            if let Some(debugger) = d.borrow().as_ref() {
                debugger.bkpt(index as u32);
            }
        });
    }
}
