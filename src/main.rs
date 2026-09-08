#![no_std]
#![no_main]

extern crate alloc;
use bevy::prelude::*;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::panic::PanicInfo;
use core::ptr;

// Reserve a static 1MB heap block inside the main RAM pool
const HEAP_SIZE: usize = 1024 * 1024;

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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        // Direct MMIO: Set the Graphics Synthesizer background color to bright blue
        // R=0x00, G=0x00, B=0xFF
        GS_BGCOLOR.write_volatile(0xFF00_0000);
    }

    loop {}
}

// Memory-Mapped IO addresses for the Emotion Engine / GS
const GS_BGCOLOR: *mut u64 = 0x1200_00E0 as *mut u64;

#[unsafe(no_mangle)]
pub extern "C" fn __start() -> ! {
    // println!("Hello, PS2 World!");

    App::new()
        .add_systems(Startup, hello_world_system)
        .add_systems(Update, cycle_background_color_system)
        .run();

    loop {}
}

fn hello_world_system() {
    unsafe {
        GS_BGCOLOR.write_volatile(u64::MAX);
    }
}

fn cycle_background_color_system(mut hue: Local<f32>) {
    // Advance the hue one step per frame (0.01 ≈ full cycle every 1.7s @60fps)
    *hue += 0.01;
    if *hue >= 1.0 {
        *hue -= 1.0;
    }

    // HSV -> RGB at full saturation/value
    let h = *hue * 6.0;
    let sector = (h as u32) % 6;
    let f = h - (h as u32 as f32); // fractional part
    let q = 1.0 - f;

    let (r, g, b) = match sector {
        0 => (1.0, f, 0.0),
        1 => (q, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, q, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, q),
    };

    // Scale to the GS BGCOLOR 6-bit channels:
    //   B -> bits 16..22, G -> bits 8..14, R -> bits 0..6
    let r6 = (r * 63.0) as u32 & 0x3F;
    let g6 = (g * 63.0) as u32 & 0x3F;
    let b6 = (b * 63.0) as u32 & 0x3F;
    let color = (b6 << 16) | (g6 << 8) | r6;

    unsafe {
        GS_BGCOLOR.write_volatile(color as u64);
    }
}
