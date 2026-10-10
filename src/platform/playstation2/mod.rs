#![allow(unused)]

use alloc::format;
use alloc::string::String;
use bevy::MinimalPlugins;
use bevy::app::ScheduleRunnerPlugin;
use bevy::color::{Color, ColorToComponents, ColorToPacked, Hsva};
use bevy::diagnostic::FrameCountPlugin;
use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy::time::TimePlugin;
use bevy::time::{Real, Time, Virtual};
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ffi::{c_int, c_uint};
use core::marker::Sync;
use core::panic::PanicInfo;
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

use ps2sdk_sys::gskit;
use ps2sdk_sys::gskit::GSGLOBAL;
use ps2sdk_sys::kernel;

use crate::shared::{BackgroundColor, GameCamera, GameMesh};

extern crate alloc;

mod components;
pub use components::*;

/// Holds the raw gsKit context handle for the Emotion Engine.
#[derive(Resource)]
pub struct GsContext(pub *mut GSGLOBAL);

// Raw pointers don't implement Send/Sync by default.
// Since PS2 runs single-threaded, it is safe to mark this as Sync.
unsafe impl Send for GsContext {}
unsafe impl Sync for GsContext {}

pub struct Ps2PlatformPlugin;

impl Plugin for Ps2PlatformPlugin {
    fn build(&self, app: &mut App) {
        // Initialize gsKit hardware context
        let gs_global = unsafe {
            let gs = gskit::gsKit_hires_init_global();
            gskit::gsKit_init_screen(gs);

            gs
        };

        app.insert_resource(GsContext(gs_global));

        app.set_runner(ps2_runner);

        app.add_plugins(
            // TaskPoolPlugin immediatly panics, disable for now.
            MinimalPlugins.build().disable::<TaskPoolPlugin>(),
        );

        // NOTE: `hello_world_system` / `render_system` are not defined anywhere
        // in the crate, so they are no longer registered here.
        app.add_systems(Update, (attach_ps2_camera, attach_ps2_mesh));
        app.add_systems(
            Last,
            (
                clear_screen_system,
                draw_meshes_system, // Queries Query<(&Transform, &Ps2Mesh)>
                flip_display_system,
            )
                .chain(),
        );
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

// Move into plugin build function?
pub fn init() {
    unsafe {
        ps2sdk_sys::common::InitDebug();
        ps2sdk_sys::common::init_scr();
    }

    // Set initial clock...
    unsafe {
        Instant::set_elapsed(get_ps2_elapsed);
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

        if let Some(exit) = app.should_exit() {
            return exit;
        }
    }
}

// Reserve a static 16MB heap block inside the main RAM pool
const HEAP_SIZE: usize = 1024 * 1024 * 16;

// TODO: Replace with linked_list_allocator
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

#[inline(never)]
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

#[cfg(target_vendor = "sony")]
fn attach_ps2_camera(mut commands: Commands, query: Query<Entity, Added<GameCamera>>) {
    for entity in query.iter() {
        commands.entity(entity).insert(Ps2Camera::default());
    }
}

#[cfg(target_vendor = "sony")]
fn attach_ps2_mesh(mut commands: Commands, query: Query<Entity, Added<GameMesh>>) {
    for entity in query.iter() {
        commands.entity(entity).insert(Ps2Mesh {});
    }
}

fn clear_screen_system(gs: Res<GsContext>, bg: Res<BackgroundColor>) {
    let raw_color = Ps2Color::from(bg.0);

    unsafe {
        gskit::gsKit_clear(gs.0, raw_color.0);
    }
}

fn draw_meshes_system(query: Query<(&Transform, &Ps2Mesh)>) {
    for entity in query.iter() {
        // TODO: Implement actual mesh drawing logic here.
    }
}

fn flip_display_system(gs: Res<GsContext>) {
    unsafe {
        gskit::gsKit_queue_exec(gs.0);
        gskit::gsKit_sync_flip(gs.0);
    }
}

pub struct Ps2Color(pub u64);

/// Converts the Bevy Color into the 64-bit integer format expected by gsKit
impl From<Color> for Ps2Color {
    #[inline(always)]
    fn from(color: Color) -> Self {
        let srgba = color.to_srgba();

        // 1. Clamp to [0.0, 1.0] to prevent math anomalies
        // 2. Cast to u32 FIRST to trigger native PS2 float-to-word instructions
        // 3. Upcast to u64 for bitwise packing
        let r = (srgba.red.clamp(0.0, 1.0) * 255.0) as u32 as u64;
        let g = (srgba.green.clamp(0.0, 1.0) * 255.0) as u32 as u64;
        let b = (srgba.blue.clamp(0.0, 1.0) * 255.0) as u32 as u64;

        let a = 0x80_u64; // Standard solid alpha for gsKit

        Self(r | (g << 8) | (b << 16) | (a << 24))
    }
}

// /// Helper to pack 8-bit RGBA channels into the 64-bit format expected by gsKit
// #[inline(always)]
// fn gs_color(r: u8, g: u8, b: u8, a: u8) -> u64 {
//     (r as u64) | ((g as u64) << 8) | ((b as u64) << 16) | ((a as u64) << 24)
// }
