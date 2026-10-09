use alloc::format;
use alloc::string::String;
use bevy::prelude::*;
// use bevy::time::TimePlugin;
// use bevy::utils::Instant;
use bevy::time::TimePlugin;
use bevy_color::{Color, Hsva,ColorToPacked};
use bevy::platform::time::{Instant};
use core::sync::atomic::{AtomicU32, Ordering};
use bevy::time::{Time, Real, Virtual};
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ffi::{c_int, c_uint};
use core::marker::Sync;
use core::panic::PanicInfo;
use core::prelude::rust_2024::global_allocator;
use core::ptr;
use core::time::Duration;
// use prussia_rt::cop0;
use ps2sdk_sys::kernel;
use ps2sdk_sys::gskit;

extern crate alloc;
// extern crate prussia_rt;

pub struct Ps2PlatformPlugin;

impl Plugin for Ps2PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.set_runner(ps2_runner);
        // app.add_systems(Startup, init)
        app.add_systems(Startup, hello_world_system);
        app.add_systems(Update, cycle_background_color_system);
    
        app.insert_resource(BackgroundColor::default());
    }
}

#[derive(Resource)]
pub struct BackgroundColor(pub Color);

impl BackgroundColor {
    /// Converts the Bevy Color into the 64-bit integer format expected by gsKit
    #[inline(always)]
    pub fn to_gs_format(&self) -> u64 {
        let srgba = self.0.to_srgba();
        
        // 1. Clamp to [0.0, 1.0] to prevent math anomalies
        // 2. Cast to u32 FIRST to trigger native PS2 float-to-word instructions
        // 3. Upcast to u64 for bitwise packing
        let r = (srgba.red.clamp(0.0, 1.0) * 255.0) as u32 as u64;
        let g = (srgba.green.clamp(0.0, 1.0) * 255.0) as u32 as u64;
        let b = (srgba.blue.clamp(0.0, 1.0) * 255.0) as u32 as u64;
        
        let a = 0x80_u64; // Standard solid alpha for gsKit
        
        r | (g << 8) | (b << 16) | (a << 24)
    }
}

impl Default for BackgroundColor {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

/// Emotion Engine CPU COP0 Clock frequency (294.912 MHz)
const EE_COP0_CLK_HZ: u64 = 294_912_000;

static LAST_COUNT: AtomicU32 = AtomicU32::new(0);
static ACCUMULATED_TICKS: AtomicU32 = AtomicU32::new(0); // Use AtomicU64 if needed

fn get_ps2_elapsed() -> Duration {
    let current_count = unsafe { kernel::GetCop0(8) }; // or 9 for Count
    
    // Example overflow handling logic (if you were doing this)
    let last = LAST_COUNT.load(Ordering::Relaxed);
    let diff = current_count.wrapping_sub(last);
    LAST_COUNT.store(current_count, Ordering::Relaxed);
    
    let total_ticks = ACCUMULATED_TICKS.fetch_add(diff, Ordering::Relaxed) + diff;
    
    let total_nanos = (total_ticks as u64 * 1_000_000_000) / EE_COP0_CLK_HZ as u64;
    
    Duration::from_nanos(total_nanos)
}

pub fn init() {
    unsafe {
        ps2sdk_sys::common::InitDebug();
        ps2sdk_sys::common::init_scr();
    }

    // Set initial clock...
    unsafe {
        Instant::set_elapsed(|| {
            Duration::ZERO
        });
    }

    println!("Init!");
    println!(
        "Heap: {:?} Offset: {:?}",
        ALLOCATOR.heap.get(),
        ALLOCATOR.offset.get()
    );
}

fn ps2_runner(mut app: App) -> AppExit {
    let gs_global = unsafe {
        let gs = gskit::gsKit_hires_init_global();
        gskit::gsKit_init_screen(gs);
        gs
    };

    loop {
        unsafe {
            Instant::set_elapsed(get_ps2_elapsed);
        }

        // Run schedule updates
        app.update();

        // 3. Fetch the latest background color computed by the app
        let raw_color = app.world().resource::<BackgroundColor>().to_gs_format();

        // println!("{:?}", raw_color);

        unsafe {
            // 4. Clear the screen, queue the execution, and flip the frame buffer
            gskit::gsKit_clear(gs_global, raw_color);
            gskit::gsKit_queue_exec(gs_global);
            gskit::gsKit_sync_flip(gs_global);
        }

        if let Some(exit) = app.should_exit() {
            return exit;
        }
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
            unsafe {
                let res = heap_ptr.add(start);

                // println!("Allocated at: {:p}\0", heap_ptr.add(start));

                res
            }
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
        ps2sdk_sys::common::scr_setbgcolor(0xFF0000FF);
    }

    println!("Panic: {}", info);

    // println!("Panic!\nMessage: {:?}", info.message());

    // if let Some(location) = info.location() {
    //     println!("Location: {}:{}", location.file(), location.line());
    // }

    // Fetch and print the 16 deepest frame addresses from PS2 memory
    let backtrace = get_ps2_backtrace::<16>();
    println!("{}", backtrace);

    loop {
        core::hint::spin_loop();
    }
}

#[inline(always)]
pub fn get_ps2_backtrace<const MAX_DEPTH: usize>() -> String {
    // Array buffer to hold instruction pointers populated by PS2SDK
    let mut stack_buffer: [c_uint; MAX_DEPTH] = [0; MAX_DEPTH];

    // Query the PS2SDK call stack tracer
    unsafe { ps2sdk_sys::common::ps2GetStackTrace(stack_buffer.as_mut_ptr(), MAX_DEPTH as c_int) };

    let count = MAX_DEPTH;

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

fn hello_world_system() {
    unsafe {
        // Set background color to blue.
        ps2sdk_sys::common::scr_setbgcolor(0x0000FFFF);
    }
}

fn cycle_background_color_system(mut hue: Local<f32>, mut bg: ResMut<BackgroundColor>) {
    // Advance hue (0.0 to 360.0 degrees for Bevy's Hsva)
    *hue += 3.6; 
    if *hue >= 360.0 {
        *hue -= 360.0;
    }

    // Let Bevy handle the HSV to RGB conversion internally
    bg.0 = Color::from(Hsva::new(*hue, 1.0, 1.0, 1.0));
}

/// Helper to pack 8-bit RGBA channels into the 64-bit format expected by gsKit
#[inline(always)]
fn gs_color(r: u8, g: u8, b: u8, a: u8) -> u64 {
    (r as u64) | ((g as u64) << 8) | ((b as u64) << 16) | ((a as u64) << 24)
}