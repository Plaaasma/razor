//! Runtime-tunable search parameters, exposed as UCI spin options so an external
//! SPSA driver can vary them without rebuilding. Defaults equal the previously
//! hardcoded constants, so behaviour (and `bench`) is identical out of the box.
//!
//! Set via `setoption name <id> value <n>` (ids below, case-insensitive). The
//! LMR table reads its params once at first search (UCI sets options before the
//! first `go`, so an SPSA run that sets options at launch gets the right table);
//! the margin params are read live per node.

use std::sync::atomic::{AtomicI32, Ordering};

/// LMR reduction: r = LMR_BASE/100 + ln(d)*ln(mc) / (LMR_DIV/100)
pub static LMR_BASE: AtomicI32 = AtomicI32::new(75); // 0.75
pub static LMR_DIV: AtomicI32 = AtomicI32::new(225); // 2.25
/// reverse-futility margin per remaining ply (cp)
pub static RFP_MARGIN: AtomicI32 = AtomicI32::new(80);
/// futility pruning margin: FUT_BASE + FUT_SCALE * depth (cp)
pub static FUT_BASE: AtomicI32 = AtomicI32::new(80);
pub static FUT_SCALE: AtomicI32 = AtomicI32::new(120);
/// singular-extension margin: sbeta = tt_score - SE_MARGIN/100 * depth (cp/ply)
pub static SE_MARGIN: AtomicI32 = AtomicI32::new(200); // 2.00 * depth

#[inline(always)]
pub fn get(a: &AtomicI32) -> i32 {
    a.load(Ordering::Relaxed)
}

/// Apply a tunable by lowercase UCI id. Returns true if the id matched.
pub fn set(name: &str, v: i32) -> bool {
    let a = match name {
        "lmrbase" => &LMR_BASE,
        "lmrdiv" => &LMR_DIV,
        "rfpmargin" => &RFP_MARGIN,
        "futbase" => &FUT_BASE,
        "futscale" => &FUT_SCALE,
        "semargin" => &SE_MARGIN,
        _ => return false,
    };
    a.store(v, Ordering::Relaxed);
    true
}
