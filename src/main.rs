#![no_std]
#![no_main]

use core::ffi::c_char;

extern crate panic_halt;

unsafe extern "C" {
    // Initializes the debug screen system
    pub fn init_scr();

    // On-screen formatted printing (C-style varargs)
    pub fn scr_printf(fmt: *const c_char, ...);
}

#[unsafe(no_mangle)]
fn main() -> ! {
    unsafe {
        init_scr();
    }

    loop {
        unsafe {
            scr_printf(b"Hello, world!\n\0".as_ptr() as *const c_char);
        }
    }
}
