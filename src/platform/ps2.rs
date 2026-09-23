use bevy::prelude::*;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ffi::{c_int, c_uint};
use core::marker::Sync;
use core::panic::PanicInfo;
use core::prelude::rust_2024::global_allocator;
use core::ptr;
use alloc::format;
use alloc::string::String;
use ps2sdk_sys::*;

extern crate alloc;

pub struct Ps2PlatformPlugin;

impl Plugin for Ps2PlatformPlugin {
    fn build(&self, app: &mut App) {
        // app.add_systems(Startup, init)
        app.add_systems(Update, count_entities_system);
    }
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

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        // Set background color to red.
        ps2sdk_sys::scr_setbgcolor(0xFF0000FF);
    }

    println!("Panic!\n{:?}", info);
    println!("Panic!\nMessage: {:?}", info.message());

    if let Some(location) = info.location() {
        println!("Location: {}:{}", location.file(), location.line());
    }

    // Fetch and print the 16 deepest frame addresses from PS2 memory
    let backtrace = get_ps2_backtrace::<16>();
    println!("{}", backtrace);

    loop {
        core::hint::spin_loop();
    }
}

pub fn get_ps2_backtrace<const MAX_DEPTH: usize>() -> String {
    // Array buffer to hold instruction pointers populated by PS2SDK
    let mut stack_buffer: [c_uint; MAX_DEPTH] = [0; MAX_DEPTH];

    // Query the PS2SDK call stack tracer
    unsafe {
        ps2sdk_sys::ps2GetStackTrace(
            stack_buffer.as_mut_ptr(),
            MAX_DEPTH as c_int,
        )
    };

    // let count = if frames_found < 0 {
    //     0
    // } else {
    //     (frames_found as usize).min(MAX_DEPTH)
    // };

    let count = MAX_DEPTH as usize;

    // Format the results into a readable trace
    let mut backtrace_str = String::from("PS2 EE Backtrace:\n");
    for (idx, &pc_addr) in stack_buffer[..count].iter().enumerate() {
        // Address format compatible with addr2line or PCSX2 ELF symbol maps
        let line = format!("  {:2}: {:#010x}\n", idx, pc_addr);
        backtrace_str.push_str(&line);
    }

    if count == 0 {
        backtrace_str.push_str("  <no stack frames found>\n");
    }

    backtrace_str
}

// Memory-Mapped IO addresses for the Emotion Engine / GS
// const GS_PMODE: *mut u64 = 0x1200_0000 as *mut u64;
// const GS_SMODE2: *mut u64 = 0x1200_0020 as *mut u64;
// const GS_DISPFB2: *mut u64 = 0x1200_0090 as *mut u64;
// const GS_DISPLAY2: *mut u64 = 0x1200_00A0 as *mut u64;
// const GS_CSR: *mut u64 = 0x1200_1000 as *mut u64;
// const GS_BGCOLOR: *mut u64 = 0x1200_00E0 as *mut u64;

// #[unsafe(no_mangle)]
// pub extern "C" fn __start() -> ! {
//     init_gs();
//     // println!("Hello, PS2 World!");

//     let mut app = App::new();
//     app.add_systems(Startup, hello_world_system);
//     app.add_systems(Update, cycle_background_color_system);

//     // Run Startup systems
//     app.update();

//     loop {
//         // Wait for the CRT beam to reset (Lock to 60Hz NTSC)
//         wait_vsync();

//         // Run the Bevy Update schedule
//         app.update();
//     }
// }

pub fn init() {
    unsafe {
        ps2sdk_sys::init_scr();
    }

    println!("Init!");
}

// pub fn wait_vsync() {
//     unsafe {
//         // Clear the VSync flag by writing 1 to bit 3
//         GS_CSR.write_volatile(1 << 3);

//         // Spin-lock until the hardware sets the VSync flag back to 1 (every ~16.6ms)
//         while (GS_CSR.read_volatile() & (1 << 3)) == 0 {
//             core::hint::spin_loop();
//         }
//     }
// }

// fn init_ps2_hardware() {
//     unsafe {
//         // Minimal GS initialization for NTSC display output
//         core::ptr::write_volatile(0x1200_0020 as *mut u64, 0x02); // SMODE2
//         core::ptr::write_volatile(0x1200_0000 as *mut u64, 0x66); // PMODE
//     }
// }

// fn init_gs() {
//     unsafe {
//         // 1. Reset the Graphics Synthesizer (Write 1 to bit 9 of CSR)
//         GS_CSR.write_volatile(1 << 9);

//         // 2. Enable Read Circuit 1 & 2 in PMODE (Bits 0 and 1)
//         // This tells the GS to actually output a visual signal to the screen.
//         GS_PMODE.write_volatile(0x0000_0000_0000_0066);
//     }
// }

// fn init_video() {
//     unsafe {
//         // 1. Set Video Mode to NTSC (Interlaced)
//         GS_SMODE2.write_volatile(0x02);

//         // 2. Configure Display Buffer 2 (Format: PSMCT32, Width: 10, Base: 0)
//         GS_DISPFB2.write_volatile(0x0000_0000_0900_0000);

//         // 3. Configure Display Area 2 (Magic numbers for standard 640x448 NTSC)
//         GS_DISPLAY2.write_volatile(0x0009_4260_001A_09FF);

//         // 4. Enable Read Circuit 1 & 2 to push pixels to the screen
//         GS_PMODE.write_volatile(0x0000_0000_0000_0066);
//     }
// }

fn hello_world_system() {
    unsafe {
        // Set background color to blue.
        ps2sdk_sys::scr_setbgcolor(0x0000FFFF);
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
        // Set background color to red.
        ps2sdk_sys::scr_setbgcolor(color);
    }
}

pub fn count_entities_system(entities: Query<Entity>) {
    let total_entities = entities.iter().count();
    print!("Total entities in world: {}", total_entities);
}

//
//
//
