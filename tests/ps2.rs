//! Tests that need a PS2 to mean anything, so: PCSX2.
//!
//! These live in the workspace root rather than in `ps2sdk-sys` because the
//! root is the crate that knows how to turn declarations into a bootable ELF
//! (linkfile + PS2SDK library group, see `build.rs`).
//!
//! On the host, everything here is `#[cfg]`'d away and `cargo test` compiles an
//! empty test binary. To actually run them:
//!
//! ```text
//! just ps2-test
//! ```
//!
//! Cargo builds the test as an ELF and hands it to the PS2 target's runner,
//! `scripts/pcsx2-test-runner.rs`, which boots it in PCSX2, streams the EE
//! debug console to stdout and exits non-zero unless the log ends with the pass
//! marker. The harness below prints those markers.
//!
//! Note that `custom_test_frameworks` doesn't parse command line args, so
//! filters (`cargo test some_test`) are ignored - every `#[test_case]` runs.

//
// PS2 only...
//

#![cfg_attr(target_vendor = "sony", no_std)]
#![cfg_attr(target_vendor = "sony", no_main)]
#![cfg_attr(target_vendor = "sony", feature(custom_test_frameworks))]
#![cfg_attr(target_vendor = "sony", test_runner(crate::harness::test_runner))]
#![cfg_attr(target_vendor = "sony", reexport_test_harness_main = "test_main")]

#[cfg(target_vendor = "sony")]
extern crate alloc;

/// Minimal `std`-free test harness.
///
/// `libtest` needs a filesystem, threads and a stack big enough for its own
/// good, none of which the EE has, so the tests are collected by
/// `custom_test_frameworks` and run by hand here. Output goes to the EE debug
/// console through `ps2sdk_sys`'s `print!`/`println!`.
#[cfg(target_vendor = "sony")]
mod harness {
    use core::panic::PanicInfo;

    use ps2sdk_sys::print;
    use ps2sdk_sys::println;

    /// Printed once every test has run. The runner script watches the log for
    /// this and reports success to cargo.
    pub const ALL_PASSED: &str = "TESTS FINISHED - ALL PASSED";

    /// Printed by the panic handler. The runner treats this as a failure.
    pub const FAILED: &str = "TESTS FINISHED - FAILED";

    pub trait Testable {
        fn run(&self);
    }

    impl<T> Testable for T
    where
        T: Fn(),
    {
        fn run(&self) {
            print!("test {:<56} ...", core::any::type_name::<T>());
            self();
            println!(" ok");
        }
    }

    pub fn test_runner(tests: &[&dyn Testable]) {
        println!("running {} tests", tests.len());
        for test in tests {
            test.run();
        }
        println!("{ALL_PASSED}");
        park();
    }

    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        println!("[FAILED] {info}");
        println!("{FAILED}");
        park();
    }

    pub(crate) fn init() {
        // EE text console, which is where `_print` output ends up.
        unsafe {
            ps2sdk_sys::common::init_scr();
        }

        init_heap();
    }

    //
    // Heap...
    //

    // Same 16MB static arena the game uses; the EE has no MMU, so a bump of
    // BSS is as good as it gets.
    const HEAP_SIZE: usize = 1024 * 1024 * 16;

    #[repr(C, align(16))]
    struct HeapBuffer([u8; HEAP_SIZE]);

    #[global_allocator]
    static ALLOCATOR: linked_list_allocator::LockedHeap =
        linked_list_allocator::LockedHeap::empty();

    static HEAP_MEMORY: HeapBuffer = HeapBuffer([0; HEAP_SIZE]);

    fn init_heap() {
        unsafe {
            ALLOCATOR
                .lock()
                .init(HEAP_MEMORY.0.as_ptr() as *mut u8, HEAP_SIZE);
        }
    }

    /// PCSX2 stays up after the last test prints its marker; the runner script
    /// is the one that kills it. Parking here beats returning into crt0.
    fn park() -> ! {
        loop {
            core::hint::spin_loop();
        }
    }
}

/// The PS2SDK crt0 enters through `main`, same as for the game binary.
///
/// `test_main` is injected by `reexport_test_harness_main`, and only resolves
/// from the crate root, which is why `main` lives out here rather than in
/// `harness`.
#[cfg(target_vendor = "sony")]
#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    harness::init();
    ps2sdk_sys::println!("--- ps2 integration tests ---");

    test_main();

    0
}

/// Hardware/SDK tests. Everything in here assumes it is running on an EE.
#[cfg(target_vendor = "sony")]
mod tests {
    use alloc::boxed::Box;
    use alloc::vec;
    use alloc::vec::Vec;

    use prussia_rt::cop0::TimerCount;
    use ps2sdk_sys::kernel;
    use ps2sdk_sys::println;

    /// 0xAABBGGRR, the way gsKit wants its colours.
    const fn rgba(r: u64, g: u64, b: u64, a: u64) -> u64 {
        r | (g << 8) | (b << 16) | (a << 24)
    }

    /// Smoke test for the whole pipeline: crt0, the debug console, the harness
    /// and the runner script. If the runner sees this line, printing works.
    #[test_case]
    fn console_output_reaches_the_log() {
        println!("hello from the Emotion Engine");
    }

    /// Exercises a real kernel syscall and gives the log something to look at.
    #[test_case]
    fn kernel_reports_the_ram_size() {
        let bytes = unsafe { kernel::GetMemorySize() };
        println!("GetMemorySize() -> {bytes} bytes");

        // Every PS2 ships with 32MB of RDRAM.
        assert!(bytes >= 32 * 1024 * 1024, "expected at least 32MB");
    }

    /// The COP0 count register is what `bevy_ps2`'s hardware-backed time reads.
    #[test_case]
    fn cop0_count_register_ticks() {
        let start = TimerCount::load().value();

        let mut now = start;
        for _ in 0..1_000_000 {
            now = TimerCount::load().value();
            if now != start {
                break;
            }
        }

        assert_ne!(now, start, "COP0 count never advanced");
    }

    /// Characterises how fast COP0 `Count` runs, so the frequency the game divides by can be
    /// checked against the hardware instead of taken on faith.
    ///
    /// PCSX2 stamps every console line with host wall-clock time, so the gap between the two
    /// timestamps this prints is roughly the wall time `SAMPLE_TICKS` covers. It is only
    /// *roughly*: the lines travel over the emulated debug port, which added ~7% of skew when
    /// this was last read (200000033 ticks in 0.635s, i.e. 315 MHz apparent against 294.912 MHz
    /// nominal). For a trustworthy number, compare the game's own `[Ns] Total entities` lines
    /// against their timestamps over tens of seconds instead.
    #[test_case]
    fn cop0_count_rate_is_measurable() {
        // ~0.68s of emulated time at 294.912 MHz: short enough to keep the suite brisk, long
        // enough that the timestamp resolution is not the limiting factor.
        const SAMPLE_TICKS: u32 = 200_000_000;

        let start = TimerCount::load().value();
        println!("rate probe start: count={start}");

        let mut now = start;
        while now.wrapping_sub(start) < SAMPLE_TICKS {
            now = TimerCount::load().value();
        }

        let elapsed = now.wrapping_sub(start);
        println!("rate probe end: count={now} ({elapsed} ticks)");

        // `Count` is a free runner, so the spin can only overshoot by the handful of ticks the
        // final read took. A large overshoot means the register is not incrementing at all and
        // the loop exited for some other reason.
        assert!(
            elapsed >= SAMPLE_TICKS && elapsed - SAMPLE_TICKS < 1_000_000,
            "expected ~{SAMPLE_TICKS} ticks, measured {elapsed}"
        );
    }

    /// The allocator in `harness` is the only thing between `alloc` and a
    /// null pointer, so push a few megabytes through it.
    #[test_case]
    fn heap_holds_a_few_megabytes() {
        let mut buffer: Vec<u8> = vec![0xAB; 4 * 1024 * 1024];
        let last = buffer.len() - 1;

        buffer[0] = 0x01;
        buffer[last] = 0x02;

        assert_eq!(buffer[0], 0x01);
        assert_eq!(buffer[last], 0x02);
        assert!(
            buffer[1..last].iter().all(|&byte| byte == 0xAB),
            "buffer clobbered"
        );

        // Second allocation, to prove the free list is actually reusable.
        let boxed = Box::new(41u64);
        assert_eq!(*boxed, 41);
    }

    /// Boots a gsKit context and paints the screen burgundy, which is also the
    /// colour to look for in the PCSX2 window to know the GS path works.
    #[test_case]
    fn gskit_paints_the_screen() {
        use ps2sdk_sys::gskit;

        unsafe {
            let gs = gskit::gsKit_hires_init_global();
            assert!(!gs.is_null(), "gsKit_hires_init_global() returned null");

            gskit::gsKit_init_screen(gs);
            gskit::gsKit_clear(gs, rgba(0x80, 0x1A, 0x2B, 0x80));
            gskit::gsKit_queue_exec(gs);
            gskit::gsKit_sync_flip(gs);
            gskit::gsKit_deinit_global(gs);
        }
    }
}
