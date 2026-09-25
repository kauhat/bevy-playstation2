#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;
use core::cell::UnsafeCell;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

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


#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    // platform::ps2::init();
    crate::println!("Starting automated PS2 tests...");

    // Cargo generates this function automatically based on #[test_case]
    test_main();

    0
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::println!("[FAILED]");
    crate::println!("Error: {}", info);
    crate::println!("TESTS FINISHED - FAILED");
    loop {}
}

// Reserve a static 16MB heap block inside the main RAM pool
const HEAP_SIZE: usize = 1024 * 1024 * 16;

#[repr(C, align(16))] // Align to 16 bytes for native PS2 EE alignment requirements
struct PS2StaticArena {
    heap: UnsafeCell<[u8; HEAP_SIZE]>,
    offset: UnsafeCell<usize>,
}

// GlobalAlloc requires the allocator structure to fulfill the `Sync` trait invariant
unsafe impl Sync for PS2StaticArena {}

unsafe impl GlobalAlloc for PS2StaticArena {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let offset = unsafe { &mut *self.offset.get() };
        let size = layout.size();
        let align = layout.align();

        // Ensure accurate alignment offset calculation
        let align_mask = !(align - 1);
        let start = (*offset + align - 1) & align_mask;

        if start + size > HEAP_SIZE {
            ptr::null_mut() // Out Of Memory
        } else {
            *offset = start + size;
            let heap_ptr = self.heap.get() as *mut u8;
            unsafe { heap_ptr.add(start) }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Basic bump allocators don't recover memory individually.
        // For production, consider using a `linked_list_allocator` or `buddy_allocator` crate.
    }
}

#[global_allocator]
static ALLOCATOR: PS2StaticArena = PS2StaticArena {
    heap: UnsafeCell::new([0; HEAP_SIZE]),
    offset: UnsafeCell::new(0),
};

#[test_case]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}
