#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
// Bindgen walked into libc headers (gsKit pulls in `<stdio.h>` and friends), so the
// generated modules redeclare several of the C library's string/memory routines. Seven
// of them shadow symbols `core` also predeclares, which is what the lint suppressed
// below flags. Silencing it is the sanctioned remedy - the lint's own guidance offers
// "allow this lint if the signature is compatible". These declarations are loose ends:
// nothing in the wrapper headers calls `memcpy` from Rust, and a reference that does
// appear resolves to ps2sdk's own implementation (a weak definition in `libkernel.a`),
// so the mistyped `c_ulong` size never governs a real call.
#![allow(suspicious_runtime_symbol_definitions)]

pub mod common {
    include!("bindings/common.rs");
}

pub mod kernel {
    // use super::common::*;
    include!("bindings/kernel.rs");
}

pub mod gskit {
    // use super::common::*;
    include!("bindings/gskit.rs");
}

pub mod draw {
    // use super::common::*;
    include!("bindings/draw.rs");
}

// #[macro_use]
pub mod macros;
