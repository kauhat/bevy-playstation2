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
use linked_list_allocator::LockedHeap;
use ps2sdk_sys::gskit;
use ps2sdk_sys::gskit::GSGLOBAL;
use ps2sdk_sys::kernel;

use crate::shared::{BackgroundColor, GameCamera, GameMesh};

extern crate alloc;

mod components;
pub use components::*;

// Reserve a static 16MB heap block inside the main RAM pool
const HEAP_SIZE: usize = 1024 * 1024 * 16;

#[repr(C, align(16))]
struct HeapBuffer([u8; HEAP_SIZE]);

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
static HEAP_MEMORY: HeapBuffer = HeapBuffer([0; HEAP_SIZE]);

fn init_heap() {
    let heap_start = HEAP_MEMORY.0.as_ptr() as *mut u8;
    let heap_size = HEAP_SIZE;
    unsafe {
        ALLOCATOR.lock().init(heap_start, HEAP_SIZE);
    }
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        // TODO: colors can be 32 bit as well?
        Ps2Color::from(Color::srgb(1.0, 0.0, 0.0));

        // Set background color to red.
        ps2sdk_sys::common::scr_setbgcolor(0xFF0000FF);
    }

    println!("Panic: {}", info);

    // Fetch and print the 16 deepest frame addresses from PS2 memory
    let backtrace = get_ps2_backtrace::<16>();

    println!("{}", backtrace);

    loop {
        core::hint::spin_loop();
    }
}

// Move into plugin build function?
pub fn init() {
    unsafe {
        // ps2sdk_sys::common::InitDebug();
        ps2sdk_sys::common::init_scr();
    }

    // Set initial clock...
    unsafe {
        Instant::set_elapsed(get_ps2_elapsed);
    }

    println!("Init!");
    init_heap()
}

/// Holds the raw gsKit context handle for the Emotion Engine.
#[derive(Resource)]
pub struct GsContext(pub *mut GSGLOBAL);
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
            MinimalPlugins.build(),
            // TaskPoolPlugin immediatly panics, disable for now. (mips2 fixed this????)
            // MinimalPlugins.build().disable::<TaskPoolPlugin>(),
        );

        app.add_systems(Update, (attach_ps2_camera, attach_ps2_mesh));

        // Rendering...
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

// TODO: Can use use prussia_rt instead?
fn get_ps2_elapsed() -> Duration {
    // Read the COP0 Count register (Register 9)
    let current_count = unsafe { kernel::GetCop0(9) } as u64;

    let last = LAST_COUNT.swap(current_count as u32, Ordering::Relaxed) as u64;

    // Calculate tick difference using wrapping math for 32-bit COP0 count
    let diff = (current_count as u32).wrapping_sub(last as u32) as u64;

    let accumulated = ACCUMULATED_TICKS.fetch_add(diff as u32, Ordering::Relaxed) as u64 + diff;

    // Perform math strictly in u64 space
    let total_nanos = (accumulated * 1_000_000_000) / EE_COP0_CLK_HZ;

    Duration::from_nanos(total_nanos)
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
