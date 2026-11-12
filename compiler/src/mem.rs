//! The memory budget. Running out of memory is the one resource fault
//! Rust cannot hand back as a value — a failed allocation aborts the
//! process, and an agent is left with `memory allocation of 384 bytes
//! failed` and no coordinates. So the runtime counts what the process
//! holds and the VM checks the count between instructions: a grid that
//! grows past its budget glitches where it stands, like any other fault.
//!
//! `MLANG_MEM_LIMIT` sets the budget in MiB (default 4096; 0 lifts it).
//! The count is the bytes requested from the allocator, so the same
//! program crosses the same budget at the same instruction on every run
//! of the sequential scheduler. One instruction that asks for more than
//! the machine has in a single allocation can still abort — the check
//! sits between instructions, and the per-op limits (⍸, ^) are what
//! bound those.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

pub struct Counting;

static USED: AtomicUsize = AtomicUsize::new(0);
/// 0 = no budget. Set once at startup, before any strand runs.
static LIMIT: AtomicUsize = AtomicUsize::new(0);

pub const DEFAULT_LIMIT_MIB: usize = 4096;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            USED.fetch_add(layout.size(), Relaxed);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            USED.fetch_add(layout.size(), Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        USED.fetch_sub(layout.size(), Relaxed);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            if new_size >= layout.size() {
                USED.fetch_add(new_size - layout.size(), Relaxed);
            } else {
                USED.fetch_sub(layout.size() - new_size, Relaxed);
            }
        }
        p
    }
}

/// Set the budget in MiB; 0 lifts it.
pub fn set_limit_mib(mib: usize) {
    LIMIT.store(mib.saturating_mul(1024 * 1024), Relaxed);
}

/// Read `MLANG_MEM_LIMIT` (MiB) and set the budget from it. An unset
/// variable means the default; one that is not a number is an error the
/// caller reports — a typo must not silently lift the limit.
pub fn limit_from_env() -> Result<(), String> {
    match std::env::var("MLANG_MEM_LIMIT") {
        Err(_) => set_limit_mib(DEFAULT_LIMIT_MIB),
        Ok(s) => match s.trim().parse::<usize>() {
            Ok(n) => set_limit_mib(n),
            Err(_) => {
                return Err(format!(
                    "MLANG_MEM_LIMIT must be a whole number of MiB (0 lifts the limit), got '{s}'"
                ))
            }
        },
    }
    Ok(())
}

/// The budget in MiB when the process holds more than it, else None.
#[inline]
pub fn over_budget() -> Option<usize> {
    let limit = LIMIT.load(Relaxed);
    if limit != 0 && USED.load(Relaxed) > limit {
        Some(limit / (1024 * 1024))
    } else {
        None
    }
}
