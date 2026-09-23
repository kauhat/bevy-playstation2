use core::panic::PanicInfo;

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        crate::println!("test {} ... ", core::any::type_name::<T>());
        self();
        crate::println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    crate::println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    crate::println!("TESTS FINISHED - ALL PASSED");

    // Exit loop or trigger PCSX2 shutdown via RPC/syscall
    loop {}
}

#[cfg(test)]
extern crate alloc;

#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn main() {
    test_main();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::println!("[FAILED]");
    crate::println!("Error: {}", info);
    crate::println!("TESTS FINISHED - FAILED");
    loop {}
}
