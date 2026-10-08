#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

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
