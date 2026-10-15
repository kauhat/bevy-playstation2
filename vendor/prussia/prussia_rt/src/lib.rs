//! Startup / runtime for the Sony PlayStation 2.
//!
//! This crate provides routines to bootstrap a Rust environment for the PS2.
//!
//! Your main function needs to have the following declaration:
//! ```text
//! #[no_mangle]
//! fn main() -> !;
//! ```
//!
//! # Assembly
//!
//! Everything this crate needs from assembly is either a CoP0 access (see [`cop0`]) or a 64-bit
//! load/store (see [`atomic`]), and both live inline next to the code that uses them. There is no
//! build script and no pre-assembled archive: the objects are produced by the same compiler
//! invocation as the rest of the crate, so they always carry this target's ABI flags.

#![no_std]
#![deny(missing_docs)]
#![feature(asm_experimental_arch)]

pub mod atomic;
pub mod cop0;
pub mod interrupts;

// The bootstrap. LLVM only needs a stack pointer, and the rest we can do in Rust.
//
// `_start` is only reached by images that link with `user-linkfile.ld`, which names it as ENTRY.
// An image booted through somebody else's crt0 (PS2SDK's `__start`, say) never references it, so
// `--gc-sections` discards it along with `_rust_start` and the BSS symbols below.
//
// Unlike the delay-slot trick this replaced, the stack pointer is set *before* the jump, so the
// ordering does not depend on the assembler honouring `.set noreorder`.
core::arch::global_asm!(
    "
    .global _start
_start:
    lui   $sp, 0x20
    j     _rust_start
    nop
"
);

// Static data initialised to zero goes in the .bss segment, which is essentially a pointer to
// uninitialised memory. We need to zero .bss before the main program runs.
//
// This could be done in assembly (and is in PS2SDK), but I figured this was not speed-critical.
unsafe fn zero_bss() {
    // Due to a quirk of the linker, these symbols are given addresses, but there is no space
    // allocated for them. Instead the addresses *are* the values, in this case referring to the
    // start and end of .bss.
    extern "C" {
        static mut START_OF_BSS: u8;
        static mut END_OF_BSS: u8;
    }

    // `addr_of_mut!` rather than `&mut`, because taking a reference to one of these would imply a
    // real object exists at that address with the size of the type.
    let start = &raw mut START_OF_BSS as usize;
    let end = &raw mut END_OF_BSS as usize;

    // SAFETY: for an image linked with `user-linkfile.ld` the linker guarantees
    // [START_OF_BSS, END_OF_BSS) is one contiguous writable region of that many bytes, and by
    // definition it holds nothing but zeroes so far.
    unsafe {
        core::ptr::write_bytes(start as *mut u8, 0, end - start);
    }
}

// A PS2 program's execution flow begins in _start, which sets up a stack and then calls
// _rust_start. Our job here is to provide a safe Rust startup and then call main().
#[doc(hidden)]
#[no_mangle]
pub unsafe extern "C" fn _rust_start() -> ! {
    extern "Rust" {
        fn main() -> !;
    }

    zero_bss();

    main()
}
