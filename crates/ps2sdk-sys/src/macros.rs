use core::ffi::c_char;
use core::fmt::{self, Write};

#[doc(hidden)]
pub fn _printdebug(args: core::fmt::Arguments) {
    let mut console = LibcConsole;
    let _ = console.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::macros::_printdebug(core::format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::macros::print!("\r\n");
    };
    ($($arg:tt)*) => {
        $crate::macros::_printdebug(core::format_args!($($arg)*));
        $crate::macros::_printdebug(core::format_args!("\r\n"));
    };
}

pub struct LibcConsole;

impl Write for LibcConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // A 256-byte stack buffer. We leave 1 byte at the end for the null terminator.
        const BUF_LEN: usize = 256;
        let mut buf = [0u8; BUF_LEN];
        let mut i = 0;

        for byte in s.bytes() {
            buf[i] = byte;
            i += 1;

            // If the buffer fills up, flush it to the screen, then reset
            if i == BUF_LEN - 1 {
                buf[i] = 0; // Null terminate
                unsafe {
                    // Write to stdout
                    crate::common::_print(buf.as_ptr() as *const c_char);

                    // Write to screen
                    // crate::common::scr_printf(buf.as_ptr() as *const c_char);
                }
                i = 0;
            }
        }

        // Flush any remaining bytes
        if i > 0 {
            buf[i] = 0; // Null terminate
            unsafe {
                crate::common::_print(buf.as_ptr() as *const c_char);
                // crate::common::scr_printf(buf.as_ptr() as *const c_char);
            }
        }

        Ok(())
    }
}
