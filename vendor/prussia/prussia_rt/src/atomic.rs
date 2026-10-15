//! Atomic reads and writes.

use core::arch::asm;

use crate::interrupts;

/// Read a 64-bit value from an address in a single bus transaction.
///
/// A plain Rust read of a `u64` on this target is permitted to become two 32-bit loads, which
/// would let an interrupt arriving between them see half of a value that was only half written.
/// `ld` is one access, and interrupts are masked around it so that no handler can interleave
/// either.
pub fn read_u64(addr: usize) -> u64 {
    // Initialised rather than declared-then-assigned: `interrupts::free` takes a closure, and a
    // local it writes has to be capturable as mutable. The `ld` overwrites it unconditionally.
    let mut value = 0;

    interrupts::free(|| unsafe {
        asm!("ld {}, 0({})", out(reg) value, in(reg) addr, options(nostack, preserves_flags));
    });

    value
}

/// Write a 64-bit value to an address in a single bus transaction.
///
/// See [`read_u64`] for why this cannot simply be a Rust assignment.
pub fn write_u64(addr: usize, value: u64) {
    interrupts::free(|| unsafe {
        asm!(
            "sd {}, 0({})",
            in(reg) value,
            in(reg) addr,
            options(nostack, preserves_flags),
        );
    });
}
