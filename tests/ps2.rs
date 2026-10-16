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
//! Two things to know before adding a test:
//!
//! * **Order matters.** `custom_test_frameworks` gives no isolation and the EE has
//!   no unwinding, so tests run in source order and a *hang* aborts nothing - it
//!   just burns `PS2_TEST_TIMEOUT` (default 120s) and reports no verdict. So the
//!   modules below go from pure register/byte manipulation, through things that
//!   touch hardware, to the ones that block on another thread or on the GS. Lines
//!   already printed reach the log before a timeout does, so partial results
//!   survive whatever the last test does.
//! * **Prefer `check!` over `assert!`.** `assert!` panics, which ends the run and
//!   loses every test after it. `check!`/`check_eq!` record a failure against the
//!   owning test and carry on, so one wrong assumption about hardware reports as
//!   one failed line rather than a lost suite. Keep `assert!` for "if this is
//!   wrong the rest of this test cannot run" (a null pointer, an out-of-range
//!   index).

//
// PS2 only...
//

#![cfg_attr(target_vendor = "sony", no_std)]
#![cfg_attr(target_vendor = "sony", no_main)]
#![cfg_attr(target_vendor = "sony", feature(custom_test_frameworks))]
#![cfg_attr(target_vendor = "sony", feature(asm_experimental_arch))]
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
#[macro_use]
mod harness {
    use core::fmt::Arguments;
    use core::panic::PanicInfo;
    use core::sync::atomic::{AtomicUsize, Ordering};

    use ps2sdk_sys::print;
    use ps2sdk_sys::println;

    /// Printed once every test has run, and only if nothing failed. The runner
    /// watches the log for this and reports success to cargo.
    pub const ALL_PASSED: &str = "TESTS FINISHED - ALL PASSED";

    /// Printed when a test failed, or when the panic handler ran. The runner
    /// treats this as a failure.
    pub const FAILED: &str = "TESTS FINISHED - FAILED";

    /// Failures recorded for the test currently running. Reset per test, which is
    /// sound because the runner is single-threaded and sequential.
    static FAILURES: AtomicUsize = AtomicUsize::new(0);

    /// Marks the running test as failed and says why.
    pub fn record_failure(args: Arguments) {
        FAILURES.fetch_add(1, Ordering::Relaxed);
        println!("    check failed: {args}");
    }

    /// Asserts `$cond`, recording a failure rather than panicking.
    macro_rules! check {
        ($cond:expr, $($arg:tt)*) => {
            if !$cond {
                $crate::harness::record_failure(format_args!($($arg)*));
            }
        };
    }

    /// Asserts two values are equal, reporting both on failure.
    macro_rules! check_eq {
        ($left:expr, $right:expr, $($arg:tt)*) => {{
            let (left, right) = ($left, $right);
            if left != right {
                $crate::harness::record_failure(format_args!(
                    "{left:?} != {right:?} - {}",
                    format_args!($($arg)*)
                ));
            }
        }};
    }

    pub trait Testable {
        /// Runs the test and returns how many checks it failed.
        fn run(&self) -> usize;
    }

    impl<T> Testable for T
    where
        T: Fn(),
    {
        fn run(&self) -> usize {
            print!("test {:<58} ...", core::any::type_name::<T>());

            FAILURES.store(0, Ordering::Relaxed);
            self();

            let failures = FAILURES.load(Ordering::Relaxed);
            if failures == 0 {
                println!("ok");
            } else {
                println!(
                    "FAILED ({failures} check{})",
                    if failures == 1 { "" } else { "s" }
                );
            }
            failures
        }
    }

    pub fn test_runner(tests: &[&dyn Testable]) {
        println!("running {} tests", tests.len());

        let mut passed = 0usize;
        let mut failed = 0usize;

        for test in tests {
            if test.run() == 0 {
                passed += 1;
            } else {
                failed += 1;
            }
        }

        println!();
        println!(
            "test result: {}. {passed} passed; {failed} failed",
            if failed == 0 { "ok" } else { "FAILED" }
        );

        if failed == 0 {
            println!("{ALL_PASSED}");
        } else {
            println!("{FAILED}");
        }

        park();
    }

    /// A panic is unrecoverable here (no unwinding), so it ends the run. Tests that
    /// merely *fail* should use `check!`; reaching this handler means something
    /// asserted, or the harness itself broke.
    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        println!("[PANIC] {info}");
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

    /// PCSX2 stays up after the last test prints its marker; the runner is the
    /// one that kills it. Parking here beats returning into crt0.
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
    ps2sdk_sys::println!("--- bevy-ps2 integration tests ---");

    test_main();

    0
}

//
// Smoke test for the pipeline itself: crt0, console, harness, runner. Everything
// below assumes these four work, so it goes first.
//

#[cfg(target_vendor = "sony")]
mod smoke {
    /// If the runner sees this line, printing works and the test ELF booted.
    #[test_case]
    fn console_output_reaches_the_log() {
        ps2sdk_sys::println!("hello from the Emotion Engine");
    }
}

//
// pruSSia: CoP0 registers, interrupt masking, atomic 64-bit access...
//

/// Exercises `prussia_rt`, the crate that replaced the PS2SDK syscall the game's
/// clock used to make. Nothing here can block: every call is a register access or
/// a memory read/write.
#[cfg(target_vendor = "sony")]
mod prussia {
    use core::hint::black_box;

    use prussia_rt::atomic::{read_u64, write_u64};
    use prussia_rt::cop0::{
        self, Cause, Compare, EPC, ErrorEPC, PerfCounter, PerfCounterControl, Status, TimerCount,
    };
    use prussia_rt::interrupts;

    /// Burns a predictable number of instructions, for the performance counter.
    fn work(iterations: u32) -> u32 {
        let mut acc = 0u32;
        for i in 0..iterations {
            acc = acc.wrapping_add(black_box(i));
        }
        acc
    }

    /// `Status::store` is one of the two `mtc0` paths in the crate, and reading
    /// CoP0 at all proves the `mfc0` inline asm settled on a usable register.
    #[test_case]
    fn cop0_status_roundtrips_through_a_store() {
        let first = Status::load();

        // In kernel mode, outside an exception, with CoP0 obviously in use (we just read it).
        check!(
            first.contains(Status::CU0),
            "Status.CU0 clear, yet we read CoP0"
        );
        check!(
            !first.contains(Status::EXL),
            "Status.EXL set outside an exception"
        );
        check!(
            !first.contains(Status::ERL),
            "Status.ERL set outside an error"
        );

        first.store();
        let second = Status::load();
        check_eq!(
            second.bits(),
            first.bits(),
            "Status changed across a store of the value we loaded"
        );
    }

    /// `Cause` is read-only, and its exception code should describe a real
    /// exception rather than garbage.
    #[test_case]
    fn cop0_cause_reports_a_plausible_exception_code() {
        let cause = Cause::load();
        let code = (cause & Cause::EXC_CODE).bits() >> 2;

        // 0..=15 covers every level 1 code the R5900 defines, vendor slots included.
        check!(code <= 15, "Cause.EXC_CODE out of range: {code}");
    }

    /// `Count` is the register the game clock reads, and `TimerCount::store` is the
    /// other `mtc0` path besides `Status`.
    ///
    /// Jumping `Count` *forward* is the safe direction: the clock accumulates
    /// wrapping deltas, so a forward skip loses a few microseconds, where a
    /// backward one would look like a ~14.6s wrap.
    #[test_case]
    fn cop0_count_accepts_a_write() {
        const SKIP: u32 = 0x1000;

        let before = TimerCount::load().value();
        TimerCount::from(before.wrapping_add(SKIP)).store();
        let after = TimerCount::load().value();

        check!(
            after.wrapping_sub(before) >= SKIP,
            "Count moved {:#x} ticks after a +{SKIP:#x} store (before {before:#x}, after {after:#x})",
            after.wrapping_sub(before)
        );
    }

    /// `Compare` is the register a timer interrupt fires on. Round-tripping it
    /// proves the second `mtc0` path, and keeping the value ~7s ahead of `Count`
    /// means the interrupt cannot actually come due while we look.
    ///
    /// The original value is restored before anything is asserted, because the BIOS
    /// may have its own plans for this register.
    #[test_case]
    fn cop0_compare_accepts_a_write_and_restores() {
        let original = Compare::load();
        let count = TimerCount::load().value();
        let far_ahead = count.wrapping_add(0x7FFF_FFFF);

        Compare::from(far_ahead).store();
        let observed = Compare::load().value();

        Compare::from(original.value()).store();
        let restored = Compare::load().value();

        check_eq!(
            observed,
            far_ahead,
            "Compare did not take the written value"
        );
        check_eq!(
            restored,
            original.value(),
            "Compare did not take the restored value"
        );
    }

    /// The R5900 performance-counter opcodes (`mfps`/`mtps`/`mfpc`/`mtpc`) are the
    /// one thing in the crate that cannot be inline asm - LLVM's assembler rejects
    /// them - so they are hand-encoded `.word`s inside `global_asm!`. A wrong
    /// encoding would silently read or write a *neighbouring* CoP0 register, or
    /// alias PCR0 and PCR1 together. Exact round-trips catch both.
    ///
    /// Reaching the assertions at all also proves the six encodings are legal R5900
    /// instructions; a bad opcode raises a Reserved Instruction exception, which
    /// would end the run in the panic handler.
    #[test_case]
    fn perf_counter_opcodes_hit_the_registers_they_claim() {
        let original = PerfCounterControl::load();

        // PCCR: CTE | K0 | EVENT0 = processor cycle.
        let control = PerfCounterControl::CTE
            | PerfCounterControl::K0
            | PerfCounterControl::from(cop0::PCCREvent0::ProcessorCycle);
        control.store();
        let readback = PerfCounterControl::load();

        // PCR0 and PCR1 are distinguished by the low bits of the same opcode, so
        // writing different sentinels and reading them back separately is what
        // proves the selector survived the encoding.
        let sentinel0 = 0x0FFF_0000u32;
        let sentinel1 = 0x0123_4567u32;
        PerfCounter::from(sentinel0).store_pcr0();
        PerfCounter::from(sentinel1).store_pcr1();
        let echoed0 = PerfCounter::load_pcr0().value();
        let echoed1 = PerfCounter::load_pcr1().value();

        // Restore before anything can fail, so a half-configured counter does not
        // poison the rest of the run.
        original.store();

        check_eq!(
            readback.bits(),
            control.bits(),
            "mfps/mtps did not round-trip the PCCR control word"
        );
        check_eq!(echoed0, sentinel0, "mfpc/mtpc did not round-trip PCR0");
        check_eq!(
            echoed1,
            sentinel1,
            "mfpc/mtpc did not round-trip PCR1 (the two counters may be aliased)"
        );

        // Whether the counter *increments* is a separate question from whether the
        // opcodes are right, and it belongs to the emulator rather than this crate:
        // PCSX2 models the counter registers but not the counting, so PCR0 holds
        // whatever was last stored. Reported, not asserted - an emulator that does
        // model it will show a non-zero delta here, while the round-trips above are
        // what keep the encodings honest in the meantime.
        let before = PerfCounter::load_pcr0().value();
        work(20_000);
        let after = PerfCounter::load_pcr0().value();

        ps2sdk_sys::println!();
        ps2sdk_sys::println!(
            "  PCCR wrote/read {control:#010x}/{readback:#010x}; PCR0 {before} -> {after} (counting {})",
            if after != before {
                "modelled"
            } else {
                "not modelled here"
            }
        );
    }

    /// `PerfCounter`'s two masks are the whole API surface for the overflow bit, and
    /// `is_ovfl` was `x & OVFL == 1` upstream - which can never hold for a bit that
    /// lives at position 31. The masks are public constants, so assert the contract
    /// directly rather than inferring it from a counter reading.
    #[test_case]
    fn perf_counter_splits_value_and_overflow() {
        check_eq!(PerfCounter::OVFL, 1u32 << 31, "OVFL is not bit 31");
        check_eq!(
            PerfCounter::VALUE,
            u32::MAX >> 1,
            "VALUE is not the low 31 bits"
        );
        check_eq!(
            PerfCounter::OVFL & PerfCounter::VALUE,
            0,
            "OVFL and VALUE overlap"
        );
        check_eq!(
            PerfCounter::OVFL | PerfCounter::VALUE,
            u32::MAX,
            "OVFL and VALUE do not cover the register"
        );

        // And on a live counter, VALUE must never carry the overflow bit.
        let raw = PerfCounter::load_pcr0();
        check_eq!(raw.value() & PerfCounter::OVFL, 0, "VALUE leaked OVFL");
    }

    /// `ErrorEPC` is only written by error-level exceptions; reading it must not
    /// fault, and it should hold something address-shaped.
    #[test_case]
    fn cop0_error_epc_is_readable() {
        let value = ErrorEPC::load().value();

        // Either never used (0) or an address in the EE's physical/virtual space.
        check!(
            value == 0 || value >= 0x0010_0000,
            "ErrorEPC holds an implausible address: {value:#010x}"
        );
        ps2sdk_sys::println!(
            "  (ErrorEPC = {value:#010x}, EPC = {:#010x})",
            EPC::load().value()
        );
    }

    /// `interrupts::disable`/`enable` manipulate the IE *and* EIE bits (the R5900
    /// `di`/`ei` instructions act on EIE, and using them trips an errata).
    #[test_case]
    fn interrupts_disable_and_enable() {
        let was_enabled = matches!(interrupts::status(), interrupts::Status::Enabled);

        interrupts::disable();
        check!(
            matches!(interrupts::status(), interrupts::Status::Disabled),
            "interrupts::status still reports Enabled after disable()"
        );
        let status = Status::load();
        check!(!status.contains(Status::IE), "CoP0 IE survived disable()");
        check!(!status.contains(Status::EIE), "CoP0 EIE survived disable()");

        interrupts::enable();
        check!(
            matches!(interrupts::status(), interrupts::Status::Enabled),
            "interrupts::status reports Disabled after enable()"
        );

        // Leave the mask as we found it, so a test that assumed a live interrupt
        // system is not surprised by this one.
        if !was_enabled {
            interrupts::disable();
        }
    }

    /// `free` is what `atomic::read_u64` uses to keep a 64-bit access from being
    /// torn by an interrupt, so it has to mask *during* the closure and unmask
    /// after - not merely at one end or the other.
    #[test_case]
    fn interrupts_free_masks_only_for_the_duration() {
        interrupts::enable();

        let mut live_inside = true;
        interrupts::free(|| {
            live_inside = Status::load().contains(Status::IE);
        });

        check!(
            !live_inside,
            "interrupts were still live inside interrupts::free()"
        );
        check!(
            matches!(interrupts::status(), interrupts::Status::Enabled),
            "free() did not restore the mask on the way out"
        );
    }

    /// Regression test for the bug that made `write_u64` worth testing at all: the
    /// hand-written assembly documented `$5 = low half, $6 = high half`, which is
    /// the **o32** convention for passing a `u64`. This target is **n32**, where a
    /// `u64` argument arrives whole in `$5` and `$6` is the *next* parameter - so
    /// the old routine stored a register the caller never set.
    ///
    /// High-half patterns are what catch it; a value that fits in 32 bits survives
    /// either way.
    #[test_case]
    fn atomic_u64_write_keeps_both_halves() {
        let mut storage = 0xDEAD_BEEF_DEAD_BEEFu64;
        let pointer = &raw mut storage;
        let address = pointer as usize;

        for pattern in [
            0xFFFF_FFFF_0000_0000u64,
            0x0000_0000_FFFF_FFFF,
            0xDEAD_BEEF_CAFE_F00D,
            1 << 63,
            1,
            0,
        ] {
            write_u64(address, pattern);

            check_eq!(
                unsafe { pointer.read_volatile() },
                pattern,
                "write_u64({pattern:#018x}) landed wrong"
            );
            check_eq!(
                read_u64(address),
                pattern,
                "read_u64 disagreed after write_u64({pattern:#018x})"
            );
        }
    }

    /// The other direction: `read_u64` has to see what ordinary Rust stored, which
    /// is what `biostation`'s GS IMR writes rely on.
    #[test_case]
    fn atomic_u64_read_sees_plain_rust_writes() {
        let mut storage = 0u64;
        let pointer = &raw mut storage;

        for pattern in [
            0x0000_0000_0000_0001u64,
            0x8000_0000_0000_0000,
            0x1234_5678_9ABC_DEF0,
        ] {
            unsafe { pointer.write_volatile(pattern) };
            check_eq!(
                read_u64(pointer as usize),
                pattern,
                "read_u64 did not see a volatile store of {pattern:#018x}"
            );
        }
    }
}

//
// The vendored re-rac (Ratchet & Clank) format readers, ported to `core + alloc`...
//

/// Drives `rc_formats` - the on-disc format readers from the neighbouring Ratchet & Clank port - on the EE.
///
/// The crate used to assume a hosted environment (`std::fs`, `std::collections`, `thiserror` and the `f32`/`f64`
/// transcendentals). It now compiles for `core + alloc` behind its `std` feature, and this module is the proof: it
/// parses real lumps pulled from `vendor/rerac/extracted/`, inflated in-emulator where the format calls for it, and
/// compares every result against what the host computed from identical bytes at build time.
///
/// The bytes and the answers travel in a generated module (see `build/fixtures.rs`), so there is nothing to type by
/// hand and no game data in the tree. On a checkout without the extracted discs, `PRESENT` is false and each test
/// reports itself skipped instead of pretending to pass.
///
/// Everything here is arithmetic over supplied buffers - no blocking, no hardware - so it belongs in the part of the
/// file that precedes anything that can wait on a device.
#[cfg(target_vendor = "sony")]
mod rc_formats {
    use core::hint::black_box;

    use rc_formats::tfrag_light::ps2;
    use ps2sdk_sys::println;

    /// Lumps and the host's reference answers, emitted by `build.rs`.
    pub mod fixture {
        include!(concat!(env!("OUT_DIR"), "/rc_formats_fixture.rs"));
    }

    /// Prints the reason a fixture-driven test stepped aside, and returns whether the caller should bail.
    fn skip_unless(label: &str, ready: bool) -> bool {
        if !ready {
            println!("    (skipped {label}: no extracted game data at configure time)");
        }
        !ready
    }

    /// A real WAD-compressed lump, inflated on the EE.
    ///
    /// `core_data.bin` for a whole level is ~17 MiB and would not fit beside the test runner's heap, but a HUD bank
    /// is a fair specimen of the same LZ77 stream (literals, near and far matches, padding) at 62 KiB in and
    /// exactly 128 KiB out - and the output is pinned by both length and digest, so a stream the EE walked even one
    /// byte differently lands here rather than sailing through.
    #[test_case]
    fn rc_wad_inflates_a_real_hud_bank() {
        if skip_unless("real WAD lump", fixture::HUD_BANK_WAD_OK) {
            return;
        }

        let inflated = match rc_formats::wad::decompress(fixture::HUD_BANK_WAD) {
            Ok(bytes) => bytes,
            Err(err) => {
                check!(false, "wad::decompress failed on the HUD bank: {err}");
                return;
            }
        };

        check_eq!(
            inflated.len(),
            fixture::HUD_BANK_WAD_INFLATED_LEN,
            "the EE inflated the HUD bank to the wrong size"
        );
        check_eq!(
            fnv1a(&inflated),
            fixture::HUD_BANK_WAD_INFLATED_FNV,
            "the EE's inflated HUD bank has the wrong contents"
        );

        println!(
            "    {}-byte WAD -> {} bytes, digest agreed",
            fixture::HUD_BANK_WAD.len(),
            inflated.len()
        );
    }

    /// The WAD decompressor's fiddlier branches, on hand-built streams.
    ///
    /// The vectors are `wad.rs`'s own unit tests, lifted here because those run on the host: literal runs, short
    /// and long matches, the window-extending far match, the dummy and pad packets, and the rejected
    /// adjacent-literals stream. On the EE they exercise the same code paths the digest test above does not reach.
    #[test_case]
    fn rc_wad_handles_every_packet_kind() {
        // Literal then a short "little match": 4 literals, dd, a run of 8 ds.
        expect_inflate(&[0x01, b'a', b'b', b'c', b'd', 0xC0, 0x00], b"abcdddddddd", "literal + little match");
        // Medium match with a trailing little literal run (`!?`).
        expect_inflate(&[0x01, b'x', b'y', b'z', b'w', 0x21, 0x02, 0x00, b'!', b'?'], b"xyzwwww!?", "medium match + literal");
        // Dummy packet then a pad packet.
        expect_inflate(&[0x01, b'a', b'b', b'c', b'd', 0x11, 0x02, 0x00, b'e', b'f'], b"abcdef", "dummy packet");
        expect_inflate(&[0x01, b'a', b'b', b'c', b'd', 0x12, 0x00, 0x00, 0xEE], b"abcd", "pad packet");

        // Adjacent literal packets are malformed and must be reported, not swallowed.
        let adjacent = wad(&[0x01, b'a', b'b', b'c', b'd', 0x01, b'e', b'f', b'g', b'h']);
        check!(
            rc_formats::wad::decompress(&adjacent).is_err(),
            "adjacent literal packets should be rejected"
        );
    }

    /// Prepend the 16-byte WAD header to a packet list, filling in the total size, as `wad.rs`'s tests do.
    fn wad(packets: &[u8]) -> alloc::vec::Vec<u8> {
        let mut buf = b"WAD\0\0\0\0TEST\0\0\0\0\0".to_vec();
        buf.extend_from_slice(packets);
        let size = buf.len() as u32;
        buf[3..7].copy_from_slice(&size.to_le_bytes());
        buf
    }

    fn expect_inflate(packets: &[u8], expected: &[u8], what: &str) {
        match rc_formats::wad::decompress(&wad(packets)) {
            Ok(bytes) => check_eq!(bytes.as_slice(), expected, "{what}"),
            Err(err) => check!(false, "{what}: decompress failed: {err}"),
        }
    }

    /// The level core index, parsed on the EE, reproduced against the host.
    ///
    /// `core_index.bin` is the 32 KiB table that names every table and block of a level: the gs_ram descriptors,
    /// the moby/tie/shrub class tables, and (through the boundary rule) each block's extent. Getting the same
    /// counts and the same opening block names on both sides means the whole 0xbc-byte header, the POD tables and
    /// the integer boundary walk all behaved.
    #[test_case]
    fn rc_level_core_index_matches_the_host() {
        if skip_unless("level core index", fixture::CORE_OK) {
            return;
        }

        let core = match rc_formats::level::parse_level_core(fixture::LEVEL_CORE_INDEX, 0) {
            Ok(core) => core,
            Err(err) => {
                check!(false, "parse_level_core failed on the EE: {err}");
                return;
            }
        };

        check_eq!(core.gs_ram.len(), fixture::CORE_GSRAM_COUNT, "gs_ram descriptor count");
        check_eq!(core.moby_classes.len(), fixture::CORE_MOBY_CLASSES, "moby class count");
        check_eq!(core.tie_classes.len(), fixture::CORE_TIE_CLASSES, "tie class count");
        check_eq!(core.shrub_classes.len(), fixture::CORE_SHRUB_CLASSES, "shrub class count");
        check_eq!(core.blocks.len(), fixture::CORE_BLOCKS, "block count");

        // The first few block names, which is what the boundary rule hangs off.
        let count = core.blocks.len().min(fixture::KEY_BLOCK_COUNT);
        for index in 0..count {
            check_eq!(
                core.blocks[index].name.as_str(),
                fixture::KEY_BLOCKS[index],
                "block #{index} name"
            );
        }

        println!(
            "    core: {} gsram, {} moby, {} tie, {} shrub, {} blocks",
            core.gs_ram.len(),
            core.moby_classes.len(),
            core.tie_classes.len(),
            core.shrub_classes.len(),
            core.blocks.len()
        );
    }

    /// The disc's table of contents, parsed on the EE.
    #[test_case]
    fn rc_toc_lists_every_level() {
        if skip_unless("TOC", fixture::TOC_OK) {
            return;
        }

        match rc_formats::toc::level_header_sectors(fixture::TOC) {
            Ok(sectors) => check_eq!(
                sectors.iter().filter(|s| s.is_some()).count(),
                fixture::TOC_LEVELS,
                "populated level-header slots in the TOC"
            ),
            Err(err) => check!(false, "level_header_sectors failed on the EE: {err}"),
        }
    }

    /// The level header, parsed and expanded on the EE.
    ///
    /// Beyond deserialising the 0x2434-byte structure, this expands it into the level's audio and scene lumps
    /// (`toc::level_stream_lumps`), which walks the `bindata`/`music`/`speech` arrays and their scene records and
    /// builds a formatted path for each survivor. The EE's count and opening names must match the host's.
    #[test_case]
    fn rc_level_header_expands_the_same_lumps() {
        if skip_unless("level header", fixture::LVLHDR_OK) {
            return;
        }

        let header = match rc_formats::toc::parse_level_header(fixture::LEVEL_HEADER) {
            Ok(header) => header,
            Err(err) => {
                check!(false, "parse_level_header failed on the EE: {err}");
                return;
            }
        };

        check_eq!(header.id, fixture::LVLHDR_ID, "level header id");

        let lumps = rc_formats::toc::level_stream_lumps(&header, |_sector| {
            Ok::<u64, core::convert::Infallible>(0x800)
        })
        .unwrap_or_default();

        check_eq!(lumps.len(), fixture::LVLHDR_LUMPS, "stream lump count");

        let count = lumps.len().min(fixture::LVLHDR_KEY_LUMPS.len());
        for index in 0..count {
            check_eq!(
                lumps[index].name.as_str(),
                fixture::LVLHDR_KEY_LUMPS[index],
                "stream lump #{index} name"
            );
        }
    }

    /// The occlusion block, parsed on the EE.
    ///
    /// Novalis carries a sparse occlusion tree, so the cell count is often low or zero - the point here is that the
    /// EE walks the block's node table and reaches exactly the number of cells the host did, error-free.
    #[test_case]
    fn rc_occlusion_block_parses_consistently() {
        if skip_unless("occlusion block", fixture::OCCL_OK) {
            return;
        }

        match rc_formats::occlusion::parse_occlusion_block(fixture::OCCLUSION_BLOCK) {
            Ok(occl) => {
                check_eq!(occl.cells.len(), fixture::OCCL_CELLS, "occlusion cell count");

                // The mask array must sit inside the block, and looking up a coordinate must stay in bounds.
                check!(
                    occl.masks_offset >= 0 && (occl.masks_offset as usize) <= fixture::OCCLUSION_BLOCK.len(),
                    "masks_offset {} escaped the block",
                    occl.masks_offset
                );
                let _ = rc_formats::occlusion::lookup_mask(fixture::OCCLUSION_BLOCK, 0, 0, 0);
            }
            Err(err) => check!(false, "parse_occlusion_block failed on the EE: {err}"),
        }

        // The copied block itself has to be the host's bytes, or the above proves nothing.
        check_eq!(
            fnv1a(fixture::OCCLUSION_BLOCK),
            fixture::OCCLUSION_BLOCK_FNV,
            "occlusion block digest"
        );
    }

    /// The HUD header, parsed on the EE.
    #[test_case]
    fn rc_hud_header_matches_the_host() {
        if skip_unless("HUD header", fixture::HUD_OK) {
            return;
        }

        match rc_formats::hud::parse_header(fixture::HUD_HEADER) {
            Ok(header) => {
                check_eq!(header.icon_count as usize, fixture::HUD_ICONS, "HUD icon count");
                check_eq!(header.frame_count as usize, fixture::HUD_FRAMES, "HUD frame count");
            }
            Err(err) => check!(false, "hud::parse_header failed on the EE: {err}"),
        }
    }

    /// The VU0 `sin`/`cos` the whole lighting path leans on, swept on the EE and checked against the host.
    ///
    /// `vu0_sin_cos` reimplements the vector unit's polynomial approximation over *raw float bit patterns*, folding
    /// and clamping in integer-land. It underpins every rotated moby. The samples rake across zeroes, ones, signs,
    /// denormals, infinities and angles spanning the full quadrant, so a disagreement in any arithmetic edge would
    /// show up as a differing word.
    #[test_case]
    fn rc_vu0_sin_cos_is_bit_identical_across_targets() {
        let mut differed = 0usize;
        for (angle, sin_expected, cos_expected) in fixture::VU0_SIN_COS_SAMPLES {
            let (sin_got, cos_got) = rc_formats::moby_light::vu0_sin_cos(black_box(angle));
            if !checks_equal_pair((sin_got, cos_got), (sin_expected, cos_expected), angle) {
                differed += 1;
            }
        }

        check_eq!(differed, 0, "vu0_sin_cos disagreed with the host on {differed} samples");
        println!(
            "    vu0_sin_cos: {} samples agreed bit for bit",
            fixture::VU0_SIN_COS_SAMPLES.len()
        );
    }

    fn checks_equal_pair(got: (u32, u32), expected: (u32, u32), angle: u32) -> bool {
        if got == expected {
            return true;
        }
        check!(
            false,
            "vu0_sin_cos({angle:#010x}) = ({:#010x}, {:#010x}), host says ({:#010x}, {:#010x})",
            got.0,
            got.1,
            expected.0,
            expected.1
        );
        false
    }

    /// The tuple arithmetic every light multiplies through, crossed pairwise on the EE.
    ///
    /// `ps2::{add,sub,mul,div,max}` mimic the emotion engine's FMAC behaviour (mantissa truncation, saturating
    /// sums, flush-to-zero), so they are integer code standing in for floating point. Every operand is paired with
    /// every other, including degenerate patterns no sane number would hold.
    #[test_case]
    fn rc_fmac_tuple_arithmetic_matches_the_host() {
        let mut mismatches = 0usize;
        for ([a, b], results) in fixture::TU_PAIR_SAMPLES {
            let got = [
                ps2::add(black_box(a), black_box(b)),
                ps2::sub(a, b),
                ps2::mul(a, b),
                ps2::div(a, b),
                ps2::max(a, b),
            ];
            for (index, (got, expected)) in got.iter().zip(results).enumerate() {
                if *got != expected {
                    mismatches += 1;
                    check!(
                        false,
                        "ps2 op #{index} of ({a:#010x}, {b:#010x}) = {got:#010x}, host says {expected:#010x}"
                    );
                }
            }
        }

        check_eq!(mismatches, 0, "tuple arithmetic disagreed with the host {mismatches} times");
        println!("    fmac tuples: {} pairs agreed", fixture::TU_PAIR_SAMPLES.len());
    }

    /// SHA-1, used to authenticate disc assets, agreeing with the host on a known sentence.
    ///
    /// Independent of the fixtures: works even on a checkout with no game data, so the reader always gets some
    /// exercise.
    #[test_case]
    fn rc_sha1_matches_the_known_digest() {
        const FOX: &[u8] = b"The quick brown fox jumps over the lazy dog";
        let digest = rc_formats::sha1::sha1(FOX);

        check_eq!(digest, fixture::SHA1_FOX_DIGEST, "sha1(\"The quick brown fox…\")");
        check_eq!(FOX.len(), fixture::SHA1_FOX_LEN, "sample length");
    }

    /// The carried lumps arrived intact (their bytes survive the trip into `.rodata`).
    ///
    /// A digest of the *inputs* rather than any parse of them: if this ever fails, every other test in the module
    /// is suspect, which is exactly the sort of thing worth isolating.
    #[test_case]
    fn rc_fixtures_arrived_intact() {
        if skip_unless("fixture blobs", fixture::PRESENT) {
            return;
        }

        check_eq!(fixture::TOC.len(), fixture::TOC_LEN, "TOC length");
        check_eq!(fnv1a(fixture::TOC), fixture::TOC_FNV, "TOC digest");
        check_eq!(fixture::LEVEL_CORE_INDEX.len(), fixture::LEVEL_CORE_INDEX_LEN, "core index length");
        check_eq!(fnv1a(fixture::LEVEL_CORE_INDEX), fixture::LEVEL_CORE_INDEX_FNV, "core index digest");
        check_eq!(fixture::HUD_HEADER.len(), fixture::HUD_HEADER_LEN, "HUD header length");
        check_eq!(fnv1a(fixture::HUD_HEADER), fixture::HUD_HEADER_FNV, "HUD header digest");
        check_eq!(fixture::LEVEL_HEADER.len(), fixture::LEVEL_HEADER_LEN, "level header length");
        check_eq!(fnv1a(fixture::LEVEL_HEADER), fixture::LEVEL_HEADER_FNV, "level header digest");
        check_eq!(fixture::OCCLUSION_BLOCK.len(), fixture::OCCLUSION_BLOCK_LEN, "occlusion length");
        check_eq!(fixture::HUD_BANK_WAD.len(), fixture::HUD_BANK_WAD_RAW_LEN, "HUD bank length");
    }

    /// FNV-1a, 64-bit - the digest the fixture generator stamped its reference bytes with. Recomputed here so the
    /// comparison is apples to apples without dragging in a hash crate.
    fn fnv1a(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for &byte in bytes {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// Ratchet & Clank random number generator stream (newlib-based LCG on the EE).
    #[test_case]
    fn rc_game_rng_stream_matches_reference() {
        use rc_game::rng::{Rng, LEVEL_SEED};

        let mut rng = Rng::new();
        rng.srand(LEVEL_SEED);

        for (i, &expected) in fixture::RNG_LEVEL_SEED_SAMPLES.iter().enumerate() {
            let val = rng.rand();
            check_eq!(val, expected, "RNG step #{i} mismatch");
        }
    }

    /// Ratchet & Clank spline cursor advancement and curve tracking on the EE.
    #[test_case]
    fn rc_game_spline_advance_matches_reference() {
        use rc_game::spline::{self, Cursor};

        let pts = [[0.0f32, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0], [0.0, 10.0, 5.0]];
        let spline_pts = spline::with_lengths(&pts);
        let mut cursor = Cursor::default();

        let steps = [2.5f32, 5.0, 5.0, 5.0, 5.0, 5.0];
        for (i, (&step, &(expected_pos, expected_end, expected_seg, expected_t))) in
            steps.iter().zip(&fixture::SPLINE_EVAL_SAMPLES).enumerate()
        {
            let (pos, end) = spline::advance(&spline_pts, false, step, &mut cursor);
            check_eq!(pos[0].to_bits(), expected_pos[0], "spline step #{i} pos.x");
            check_eq!(pos[1].to_bits(), expected_pos[1], "spline step #{i} pos.y");
            check_eq!(pos[2].to_bits(), expected_pos[2], "spline step #{i} pos.z");
            check_eq!(end, expected_end, "spline step #{i} end flag");
            check_eq!(cursor.seg, expected_seg, "spline step #{i} cursor seg");
            check_eq!(cursor.t.to_bits(), expected_t, "spline step #{i} cursor t");
        }
    }

    /// Ratchet & Clank VU0 vector arithmetic operations (vadd, vsub, vmul, dot_x, cross).
    #[test_case]
    fn rc_game_ps2v_vector_math() {
        use rc_game::ps2v;

        let v_a = [0x3f80_0000u32, 0x4000_0000, 0x4040_0000]; // [1.0, 2.0, 3.0]
        let v_b = [0x4080_0000u32, 0x40a0_0000, 0x40c0_0000]; // [4.0, 5.0, 6.0]

        let v_add = ps2v::vadd(v_a, v_b);
        check_eq!(v_add, fixture::PS2V_VADD_SAMPLE, "ps2v::vadd");

        let v_sub = ps2v::vsub(v_a, v_b);
        check_eq!(v_sub, fixture::PS2V_VSUB_SAMPLE, "ps2v::vsub");

        let v_mul = ps2v::vmul(v_a, v_b);
        check_eq!(v_mul, fixture::PS2V_VMUL_SAMPLE, "ps2v::vmul");

        let dot = ps2v::dot_x(v_a, v_b);
        check_eq!(dot, fixture::PS2V_DOT_SAMPLE, "ps2v::dot_x");

        let cross = ps2v::cross(v_a, v_b);
        check_eq!(cross, fixture::PS2V_CROSS_SAMPLE, "ps2v::cross");
    }

    /// Basic moby runtime table and instance creation.
    #[test_case]
    fn rc_game_moby_runtime_instance_lifecycle() {
        use alloc::vec::Vec;
        use rc_game::moby_runtime::{MobyTable, ClassInfo, state};

        let mut table = MobyTable::new(Vec::new(), 4);
        check_eq!(table.mobys.len(), 4, "initial dynamic table size");

        let class_info = ClassInfo {
            slot: 1,
            scale: 1.0,
            ..Default::default()
        };

        let id = table.create(10, Some(&class_info), 1);
        check!(id.is_some(), "create moby returned Some");
        let id = id.unwrap();
        check_eq!(table.mobys[id].o_class, 10, "moby class matches");
        check!(table.mobys[id].state < state::DELETED_STATIC, "moby starts in non-deleted state");

        table.delete(id, 1);
        check!(table.mobys[id].is_deleted(), "moby is deleted after delete()");
    }

    /// Controller record (`PadState`) and input processing on the EE.
    #[test_case]
    fn rc_game_pad_processing() {
        use rc_game::pad::{PadState, PadInput, button};

        let mut pad = PadState::default();
        let input = PadInput::default()
            .press(button::CROSS | button::SQUARE)
            .stick(0.0, 1.0);
        let bytes = input.bytes();

        pad.update(Some(&bytes), false);

        check_eq!(pad.held & button::CROSS, button::CROSS, "Cross button should be down");
        check_eq!(pad.held & button::SQUARE, button::SQUARE, "Square button should be down");
        check_eq!(pad.held & button::CIRCLE, 0, "Circle button should not be down");
        check!(pad.pressed_within(button::CROSS, 1).is_some(), "Cross was just pressed");
    }

    /// Dynamic point light bank allocation and release.
    #[test_case]
    fn rc_game_point_lights_bank() {
        use rc_game::point_lights::{PointLight, PointLights};

        let mut bank = PointLights::default();
        let light = PointLight {
            color: [1.0, 0.5, 0.25],
            intensity: 0.8,
            pos: [10.0, 20.0, 30.0],
            radius: 5.0,
        };

        let slot = bank.alloc(light, 0.5);
        check!(slot.is_some(), "Point light allocation succeeded under load threshold");
        let slot = slot.unwrap();
        check_eq!(bank.slots[slot], Some(light), "Slot holds stored light");

        bank.free(slot);
        check_eq!(bank.slots[slot], None, "Slot is cleared after free");
    }

    /// After-image motion blur ring buffer history.
    #[test_case]
    fn rc_game_afterimage_trail() {
        use rc_game::afterimage::Trail;

        let mut trail = Trail::default();
        trail.start(1);
        check!(trail.active, "Trail is active after start");

        // Add a ghost 2 ticks back with alpha 0x40
        trail.add(0x40, 2);
        check_eq!(trail.ghosts.len(), 1, "One ghost registered");
        check_eq!(trail.ghosts[0].alpha, 0x40, "Ghost alpha matches");

        trail.kill();
        check!(!trail.active, "Trail is inactive after kill");
        check_eq!(trail.ghosts.len(), 0, "Ghosts cleared after kill");
    }
}

//
// The allocator, `alloc`, and soft-float arithmetic...
//

/// Pure software, so nothing here can hang on hardware. It goes early because a
/// broken heap or a broken soft-float routine invalidates the interpretation of
/// everything after it.
#[cfg(target_vendor = "sony")]
mod runtime {
    use alloc::borrow::ToOwned;
    use alloc::boxed::Box;
    use alloc::collections::BTreeMap;
    use alloc::format;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use core::alloc::Layout;
    use core::hint::black_box;

    use ps2sdk_sys::println;

    /// The bump-allocator era is over, but the heap is still the only thing between
    /// `alloc` and a null pointer, so push a few megabytes through it.
    #[test_case]
    fn heap_holds_a_few_megabytes() {
        let mut buffer: Vec<u8> = vec![0xAB; 4 * 1024 * 1024];
        let last = buffer.len() - 1;

        buffer[0] = 0x01;
        buffer[last] = 0x02;

        check_eq!(buffer[0], 0x01, "first byte");
        check_eq!(buffer[last], 0x02, "last byte");
        check!(
            buffer[1..last].iter().all(|&byte| byte == 0xAB),
            "4MB buffer was clobbered in the middle"
        );

        // A second allocation, to prove the free list is reusable rather than a
        // one-shot bump.
        let boxed = Box::new(41u64);
        check_eq!(*boxed, 41, "small allocation after the big one");
    }

    /// A linked-list allocator that ignores alignment would hand back pointers that
    /// fault on 64- or 128-bit accesses, and the EE is fussy about that. Ask the
    /// global allocator for each alignment directly and check the pointer, rather
    /// than inferring it from a `Vec<T>` whose alignment is fixed by `T`.
    #[test_case]
    fn allocator_honours_requested_alignment() {
        for align in [1usize, 2, 4, 8, 16] {
            let layout = Layout::from_size_align(64, align).expect("valid layout");

            let pointer = unsafe { alloc::alloc::alloc(layout) };
            check!(!pointer.is_null(), "alloc(64, align {align}) returned null");
            if pointer.is_null() {
                continue;
            }

            check_eq!(
                pointer as usize % align,
                0,
                "allocation requested align {align} and got {pointer:p}"
            );

            // Write through it: an aligned-but-unmapped pointer would fault here.
            unsafe {
                pointer.write_bytes(0xA5, 64);
                check_eq!(*pointer, 0xA5, "byte 0 of the align-{align} allocation");
                alloc::alloc::dealloc(pointer, layout);
            }
        }

        // 16 bytes is the widest the GIF packets need, so a 128-bit allocation has
        // to come back 16-byte aligned.
        let wide: Vec<u128> = vec![0u128; 8];
        check_eq!(
            wide.as_ptr() as usize % 16,
            0,
            "Vec<u128> is not 16-byte aligned"
        );
    }

    /// Dropping a large allocation and then asking for the same size again is the
    /// free-list path, and the one a bump allocator would silently fail.
    #[test_case]
    fn freed_memory_is_reusable() {
        let first: Vec<u8> = vec![0x11; 2 * 1024 * 1024];
        let first_address = first.as_ptr() as usize;
        drop(first);

        let second: Vec<u8> = vec![0x22; 2 * 1024 * 1024];
        let second_address = second.as_ptr() as usize;

        check!(
            second.iter().all(|&byte| byte == 0x22),
            "second 2MB allocation came back with the wrong contents"
        );
        check!(
            second_address >= first_address.saturating_sub(4 * 1024 * 1024),
            "reuse looked nothing like the freed block"
        );

        // Both alive at once, to prove the allocator is not returning overlapping
        // blocks.
        let a: Vec<u8> = vec![0xAA; 1024 * 1024];
        let b: Vec<u8> = vec![0xBB; 1024 * 1024];
        let (a_start, a_end) = (a.as_ptr() as usize, a.as_ptr() as usize + a.len());
        let (b_start, b_end) = (b.as_ptr() as usize, b.as_ptr() as usize + b.len());

        check!(
            a_end <= b_start || b_end <= a_start,
            "two live 1MB allocations overlap ({a_start:#x}..{a_end:#x} and {b_start:#x}..{b_end:#x})"
        );
        check!(a.iter().all(|&byte| byte == 0xAA), "a was clobbered by b");
        check!(b.iter().all(|&byte| byte == 0xBB), "b was clobbered by a");
    }

    /// Growth means reallocation, which is where a broken allocator or a broken
    /// `memcpy` shows up as silently duplicated or lost elements.
    #[test_case]
    fn vec_growth_preserves_contents() {
        const COUNT: u32 = 10_000;

        let mut values: Vec<u32> = Vec::new();
        for index in 0..COUNT {
            values.push(index);
        }

        check_eq!(values.len(), COUNT as usize, "length after growth");
        let expected_sum: u64 = (COUNT as u64 * (COUNT as u64 - 1)) / 2;
        check_eq!(
            values.iter().map(|&value| value as u64).sum::<u64>(),
            expected_sum,
            "sum after growth"
        );
        check_eq!(
            values.first().copied(),
            Some(0),
            "first element after growth"
        );
        check_eq!(
            values.last().copied(),
            Some(COUNT - 1),
            "last element after growth"
        );

        // Shrink, then grow again, to exercise the free list in both directions.
        values.truncate(10);
        check_eq!(values.len(), 10, "length after truncate");
        for index in 0..COUNT {
            values.push(index);
        }
        check_eq!(values.len(), (COUNT + 10) as usize, "length after regrowth");
        check_eq!(values[10], 0, "first regrown element");
    }

    /// `alloc::collections` over the same heap, with reallocation and drop
    /// interleaved - the pattern Bevy's ECS uses constantly.
    #[test_case]
    fn btreemap_insert_overwrite_and_remove() {
        let mut map: BTreeMap<u32, String> = BTreeMap::new();

        for key in 0..500u32 {
            map.insert(key, format!("entity {key}"));
        }
        check_eq!(map.len(), 500, "length after 500 inserts");

        map.insert(7, "overwritten".to_owned());
        check_eq!(map.len(), 500, "overwrite changed the length");
        check_eq!(
            map.get(&7).map(String::as_str),
            Some("overwritten"),
            "value after overwrite"
        );

        for key in 0..250u32 {
            map.remove(&key);
        }
        check_eq!(map.len(), 250, "length after 250 removals");
        check_eq!(map.get(&0), None, "removed key still present");
        check_eq!(
            map.keys().next().copied(),
            Some(250),
            "iteration does not start at the lowest surviving key"
        );

        let total: usize = map.values().map(String::len).sum();
        check!(total > 0, "surviving values are all empty");
    }

    /// `format!` is where `core::fmt` meets the allocator, and the game leans on it
    /// for its panic backtrace strings.
    #[test_case]
    fn format_handles_padding_precision_and_debug() {
        check_eq!(format!("{}", 1234), "1234", "plain integer");
        check_eq!(format!("{:>8}", 42), "      42", "right align");
        check_eq!(format!("{:<8}|", 42), "42      |", "left align");
        check_eq!(format!("{:0>5}", 7), "00007", "zero fill");
        check_eq!(format!("{:#x}", 255), "0xff", "hex alternate");
        check_eq!(format!("{:#010x}", 255), "0x000000ff", "hex with width");
        check_eq!(format!("{:.3}", 1.5f64), "1.500", "float precision");
        check_eq!(format!("{:?}", [1u8, 2, 3]), "[1, 2, 3]", "slice debug");

        let owned = String::from("heap");
        check_eq!(owned.len(), 4, "String::from length");
        check_eq!(
            format!("{owned} and {0:?}", "positional"),
            "heap and \"positional\"",
            "mixed named and positional arguments"
        );
    }

    /// Newton's method, so the soft-float divide, add and compare all get used
    /// without depending on a `sqrt` symbol that may or may not be linked.
    fn sqrt_newton(value: f64) -> f64 {
        let mut guess = value / 2.0;
        for _ in 0..40 {
            guess = black_box((guess + black_box(value) / guess) / 2.0);
        }
        guess
    }

    /// The target is `+soft-float`, and the linker has been seen complaining about
    /// hard-float objects arriving from `libatomic`. These are the arithmetic paths
    /// the renderer actually uses: `Ps2Color::from` clamps and casts floats to
    /// integers, and the camera rotates by `elapsed_secs() as f32`.
    #[test_case]
    fn soft_float_f32_matches_the_renderer_colour_path() {
        let half = black_box(0.5f32);

        check_eq!(
            black_box(half * half),
            0.25f32,
            "f32 multiply of exact halves"
        );
        check_eq!(black_box(half + half), 1.0f32, "f32 add");
        check_eq!(
            black_box(1.0f32 / half),
            2.0f32,
            "f32 divide by an exact half"
        );
        check_eq!(black_box(-half * 4.0f32), -2.0f32, "f32 signed multiply");

        // Exactly what Ps2Color::from does.
        for (input, expected) in [
            (0.0f32, 0u32),
            (1.0, 255),
            (0.5, 127),
            (2.0, 255), // clamped high
            (-1.0, 0),  // clamped low
        ] {
            let clamped = black_box(input).clamp(0.0, 1.0);
            let packed = (clamped * 255.0) as u32;
            check_eq!(packed, expected, "colour channel for input {input}");
        }

        // A division that is not exactly representable still has to be close, which
        // is what proves the soft-float divide is not returning garbage.
        let third = black_box(1.0f32) / black_box(3.0f32);
        check!(
            (third - 0.333_333_34).abs() < 1e-6,
            "f32 1/3 came back as {third}"
        );
    }

    /// 64-bit soft float, including the compare-and-subtract a transcendental would
    /// be built from.
    #[test_case]
    fn soft_float_f64_division_and_root() {
        let ten = black_box(10.0f64);
        let four = black_box(4.0f64);

        check_eq!(ten / four, 2.5f64, "f64 divide");
        check_eq!(ten * ten, 100.0f64, "f64 multiply");
        check_eq!(
            black_box(1.0f64) / black_box(3.0f64) * 3.0,
            1.0f64,
            "f64 round trip"
        );

        let root = sqrt_newton(black_box(6.25));
        check!((root - 2.5).abs() < 1e-9, "sqrt(6.25) converged to {root}");

        let root = sqrt_newton(black_box(2.0));
        check!(
            (root - 1.414_213_562).abs() < 1e-6,
            "sqrt(2) converged to {root}"
        );
    }

    /// 64-bit integer arithmetic across the clock conversion, since the game turns
    /// COP0 ticks into a `Duration` this way.
    #[test_case]
    fn u64_tick_conversion_does_not_overflow() {
        const EE_COP0_CLK_HZ: u64 = 294_912_000;

        let ticks_to_parts = |ticks: u64| {
            let seconds = ticks / EE_COP0_CLK_HZ;
            let sub = ticks % EE_COP0_CLK_HZ;
            (seconds, sub * 1_000_000_000 / EE_COP0_CLK_HZ)
        };

        let (seconds, nanos) = ticks_to_parts(EE_COP0_CLK_HZ);
        check_eq!(seconds, 1, "one second of ticks");
        check_eq!(nanos, 0, "one second of ticks leaves a remainder");

        // The old code multiplied the whole tick count by 1e9 up front, which wraps
        // u64 at ~62s of uptime. Ten minutes has to still count up correctly.
        let ten_minutes = EE_COP0_CLK_HZ * 600;
        let (seconds, _) = ticks_to_parts(ten_minutes);
        check_eq!(seconds, 600, "ten minutes of ticks");

        let (_, nanos) = ticks_to_parts(EE_COP0_CLK_HZ / 2);
        check!(
            nanos < 1_000_000_000,
            "sub-second remainder {nanos} exceeded a second"
        );
        check!(
            (400_000_000..600_000_000).contains(&nanos),
            "half a second came out as {nanos}ns"
        );

        println!("tick math spot-check: {ten_minutes} ticks -> {seconds}s");
    }
}

//
// PS2SDK kernel: memory, cache, console...
//

#[cfg(target_vendor = "sony")]
mod memory {
    use core::ffi::c_int;

    use alloc::vec::Vec;

    use ps2sdk_sys::common::{
        CPU_DATA_CACHE, CPU_INSTRUCTION_CACHE, ps2GetStackTrace, scr_getX, scr_getY, scr_setXY,
    };
    use ps2sdk_sys::kernel::{FlushCache, GetMemorySize};

    /// The EE has no MMU, so `FlushCache` is about coherence with DMA and the
    /// write-back cache rather than protection. Whatever it does, it must not lose
    /// data we just wrote.
    #[test_case]
    fn flush_cache_preserves_dirty_data() {
        const LEN: usize = 64;

        let pattern = |index: usize| 0x5A00_0000u32 | index as u32;

        let mut buffer = [0u32; LEN];
        for (index, slot) in buffer.iter_mut().enumerate() {
            *slot = pattern(index);
        }

        unsafe {
            FlushCache(CPU_DATA_CACHE as c_int);
        }

        let first_mismatch = (0..LEN).find(|&index| buffer[index] != pattern(index));

        check_eq!(
            first_mismatch,
            None,
            "FlushCache lost or reordered written data"
        );

        // Flushing the instruction cache must not fault either; nothing here
        // modifies code, so it is a no-op that still has to return.
        unsafe {
            FlushCache(CPU_INSTRUCTION_CACHE as c_int);
        }
    }

    /// 32MB of RDRAM is the contract every PS2 ships against, and the game's 16MB
    /// heap sits inside it.
    #[test_case]
    fn memory_size_matches_the_32mb_contract() {
        let bytes = unsafe { GetMemorySize() };

        check!(
            bytes >= 32 * 1024 * 1024,
            "GetMemorySize reported {bytes} bytes, expected at least 32MB"
        );
    }

    /// The EE's scratchpad lives in KSeg1: `0x7000_0000` cached, `0xBF80_0000`
    /// uncached. Only the first 8KB is guaranteed on every model, so this stays
    /// well inside that, and the uncached alias needs no flush.
    ///
    /// Nothing in this image claims a `.spad` section (checked with readelf), so the
    /// region is free to pound on.
    #[test_case]
    fn scratchpad_roundtrips_through_the_uncached_alias() {
        const SCRATCH_UNCACHED: usize = 0xBF80_0000;
        const LEN: usize = 2048;

        let pattern = |index: usize| ((index & 0xFF) as u8) ^ 0x5A;

        unsafe {
            let base = SCRATCH_UNCACHED as *mut u8;

            for index in 0..LEN {
                base.add(index).write_volatile(pattern(index));
            }

            let mismatch =
                (0..LEN).find(|&index| base.add(index).read_volatile() != pattern(index));

            check_eq!(
                mismatch,
                None,
                "scratchpad did not read back at offset {:#x}",
                mismatch.unwrap_or(0)
            );
        }
    }

    /// The debug console's cursor is real state in the scratchpad-backed text
    /// buffer, so `scr_setXY` has to stick.
    #[test_case]
    fn console_cursor_roundtrips() {
        let original = (unsafe { scr_getX() }, unsafe { scr_getY() });

        unsafe { scr_setXY(5, 7) };
        check_eq!(unsafe { scr_getX() }, 5, "scr_getX after scr_setXY(5, 7)");
        check_eq!(unsafe { scr_getY() }, 7, "scr_getY after scr_setXY(5, 7)");

        unsafe { scr_setXY(original.0, original.1) };
    }

    /// Regression test for `ps2sdk_sys`'s print macros, which used to hand their own
    /// buffer to `_print` as the *format* string. A stray `%` in a panic message or
    /// a path would then consume varargs that were never passed - usually garbling
    /// output, occasionally faulting. The buffer is now an argument to `"%s"`, so
    /// percent signs have to arrive literal.
    #[test_case]
    fn console_prints_percent_signs_literally() {
        ps2sdk_sys::println!("literal 100% done  %s %d %n %c %% %p %08x");

        // Surviving the call is most of the assertion; the cursor check proves the
        // console state machine is still coherent afterwards.
        check!(
            unsafe { scr_getX() } >= 0,
            "console X went negative after a '%' payload"
        );
        check!(
            unsafe { scr_getY() } >= 0,
            "console Y went negative after a '%' payload"
        );
    }

    /// `ps2GetStackTrace` is what the game's panic handler uses to print a backtrace,
    /// so its output has to be addresses that can actually be symbolized.
    #[test_case]
    fn stack_trace_returns_frames_inside_text() {
        const DEPTH: usize = 16;
        let mut frames = [0u32; DEPTH];

        unsafe { ps2GetStackTrace(frames.as_mut_ptr(), DEPTH as c_int) };

        let filled: Vec<u32> = frames.iter().copied().filter(|&frame| frame != 0).collect();
        check!(!filled.is_empty(), "ps2GetStackTrace returned no frames");

        let outside = filled
            .iter()
            .position(|&frame| frame < 0x0010_0000 || frame >= 0x0200_0000);

        check_eq!(
            outside,
            None,
            "a stack frame lies outside user text: {:#010x}",
            filled.get(outside.unwrap_or(0)).copied().unwrap_or(0)
        );
    }
}

//
// gsKit and the draw library...
//

/// Ordered inside the module the same way as the file: the packet builders are pure
/// memory manipulation and cannot hang, the context tests touch the GS but do not
/// wait on it, and `vsync_wait` - which blocks - is last.
#[cfg(target_vendor = "sony")]
mod graphics {
    use core::ffi::c_int;
    use core::ptr::addr_of;

    use prussia_rt::cop0::TimerCount;
    use ps2sdk_sys::draw;
    use ps2sdk_sys::gskit::{
        self, GS_MODE_NTSC, GS_MODE_PAL, GS_PSM_CT16, GS_PSM_CT24, GS_PSM_CT32,
    };

    // The register numbers and field packing below come from disassembling
    // `libdraw.a`'s `draw_framebuffer`/`draw_zbuffer` (mips64r5900el-ps2-elf-objdump
    // on draw_environment.o), not from a spec I remembered:
    //
    //     li    v0, 14          ; GIF register DATA
    //     addiu a1, a1, 76      ; GS register FRAME_1 (+2 per context... see below)
    //     srl   v0, v0, 0xb     ; address >> 11
    //     andi  v0, v0, 0x1ff   ; ...masked to the 9-bit block pointer
    //
    // Each call writes one 16-byte GIF "packed" pair: (tag, GIF register) then
    // (register value, GS register number).
    const GIF_TAG: u64 = 0x1000_0000_0000_0001;
    const GIF_DATA: u64 = 14;
    const GS_FRAME: u64 = 76;
    const GS_ZBUF: u64 = 78;

    /// 0xAABBGGRR, the way gsKit wants its colours.
    const fn rgba(r: u64, g: u64, b: u64, a: u64) -> u64 {
        r | (g << 8) | (b << 16) | (a << 24)
    }

    /// The 9-bit block pointer libdraw packs out of a byte address.
    const fn block_pointer(address: u32) -> u64 {
        ((address >> 11) & 0x1FF) as u64
    }

    const ZERO_QWORD: draw::qword_t = draw::qword_t { qw: 0 };

    fn zeroed_qwords<const N: usize>() -> [draw::qword_t; N] {
        [ZERO_QWORD; N]
    }

    /// How many qwords a builder consumed.
    fn advance(start: *const draw::qword_t, end: *const draw::qword_t) -> usize {
        (end as usize - start as usize) / size_of::<draw::qword_t>()
    }

    fn framebuffer(address: u32, width: u32, psm: u32) -> draw::framebuffer_t {
        draw::framebuffer_t {
            address,
            width,
            height: 224,
            psm,
            mask: 0,
        }
    }

    /// Reads one half of a qword, which is the only part of the union libdraw
    /// populates.
    fn half(qword: &draw::qword_t, index: usize) -> u64 {
        unsafe { qword.dw[index] }
    }

    /// `draw_framebuffer` emits a single GIF packed pair naming the FRAME register
    /// for the requested context, with the address reduced to its block pointer.
    #[test_case]
    fn draw_framebuffer_packs_the_frame_register() {
        // Addresses chosen so the 9-bit field is exercised rather than masked away:
        // 0x0800_0000 (what this test originally used) is past GS VRAM entirely and
        // collapses to block 0, which asserts nothing.
        for (address, context) in [
            (0x0000_0800u32, 0),
            (0x0001_0000, 1),
            (0x0008_0000, 1),
            (0x000F_F800, 1),
        ] {
            let mut buffer = zeroed_qwords::<4>();
            let mut frame = framebuffer(address, 64, GS_PSM_CT32);

            let start = buffer.as_mut_ptr();
            let end = unsafe { draw::draw_framebuffer(start, context, &raw mut frame) };

            let written = advance(start, end);
            check_eq!(
                written,
                2,
                "draw_framebuffer({address:#x}) wrote {written} qwords"
            );
            if written != 2 {
                continue;
            }

            check_eq!(half(&buffer[0], 0), GIF_TAG, "GIF tag of the packed pair");
            check_eq!(
                half(&buffer[0], 1),
                GIF_DATA,
                "GIF register of the packed pair"
            );
            check_eq!(
                half(&buffer[1], 1),
                GS_FRAME + context as u64,
                "GS register for context {context}"
            );

            let value = half(&buffer[1], 0);
            check_eq!(
                value & 0x1FF,
                block_pointer(address),
                "block pointer for address {address:#x}"
            );
        }
    }

    /// The width and pixel-format fields have to land in the register too, or the
    /// GS would read the wrong raster out of VRAM.
    #[test_case]
    fn draw_framebuffer_packs_width_and_format() {
        for (width, psm) in [(64u32, GS_PSM_CT32), (128, GS_PSM_CT16), (256, GS_PSM_CT24)] {
            let mut buffer = zeroed_qwords::<4>();
            let mut frame = framebuffer(0x0000_0800, width, psm);

            let start = buffer.as_mut_ptr();
            let end = unsafe { draw::draw_framebuffer(start, 1, &raw mut frame) };
            if advance(start, end) != 2 {
                check!(
                    false,
                    "draw_framebuffer(width {width}) did not write one pair"
                );
                continue;
            }

            let value = half(&buffer[1], 0);

            // libdraw: (width >> 6) << 16, masked to six bits.
            let wanted_width = ((width >> 6) as u64) & 0x3F;
            check_eq!(
                (value >> 16) & 0x3F,
                wanted_width,
                "width field for width {width}"
            );

            // libdraw: psm << 24, masked to six bits.
            check_eq!(
                (value >> 24) & 0x3F,
                psm as u64 & 0x3F,
                "pixel-format field for psm {psm}"
            );
        }
    }

    /// `draw_zbuffer` is the same shape against a different register, which is what
    /// makes the pair layout above a library convention rather than a fluke.
    #[test_case]
    fn draw_zbuffer_packs_the_zbuf_register() {
        let mut buffer = zeroed_qwords::<4>();
        let mut depth = draw::zbuffer_t {
            enable: 1,
            method: 0,
            address: 0x0002_0000,
            zsm: 0,
            mask: 0,
        };

        let start = buffer.as_mut_ptr();
        let end = unsafe { draw::draw_zbuffer(start, 1, &raw mut depth) };

        check_eq!(
            advance(start, end),
            2,
            "draw_zbuffer did not write one packed pair"
        );
        if advance(start, end) != 2 {
            return;
        }

        check_eq!(
            half(&buffer[0], 1),
            GIF_DATA,
            "GIF register of the zbuffer pair"
        );
        check_eq!(
            half(&buffer[1], 1),
            GS_ZBUF + 1,
            "GS register for zbuffer context 1"
        );
        check_eq!(
            half(&buffer[1], 0) & 0x1FF,
            block_pointer(depth.address),
            "zbuffer block pointer"
        );
    }

    /// The builders are functions of their arguments, so building the same packet
    /// twice has to produce identical bytes. A difference would mean the library
    /// read uninitialised memory or kept hidden state between calls.
    #[test_case]
    fn draw_packet_builders_are_deterministic() {
        let mut frame = framebuffer(0x0001_0000, 32, GS_PSM_CT16);

        let mut first = zeroed_qwords::<4>();
        let mut second = zeroed_qwords::<4>();

        let first_end = unsafe { draw::draw_framebuffer(first.as_mut_ptr(), 2, &raw mut frame) };
        let second_end = unsafe { draw::draw_framebuffer(second.as_mut_ptr(), 2, &raw mut frame) };

        let first_len = advance(first.as_ptr(), first_end);
        let second_len = advance(second.as_ptr(), second_end);

        check_eq!(
            first_len,
            second_len,
            "identical calls advanced differently"
        );
        check!(first_len > 0, "draw_framebuffer emitted nothing");

        let differs = (0..first_len).find(|&index| {
            half(&first[index], 0) != half(&second[index], 0)
                || half(&first[index], 1) != half(&second[index], 1)
        });

        check_eq!(differs, None, "identical calls produced different qwords");
    }

    /// Builders chain: each returns where the next starts, so a frame followed by a
    /// z-buffer has to produce both pairs, back to back, without overlap.
    #[test_case]
    fn draw_packet_builders_chain_without_overlapping() {
        let mut buffer = zeroed_qwords::<8>();

        let mut frame = framebuffer(0x0001_0000, 64, GS_PSM_CT32);
        let mut depth = draw::zbuffer_t {
            enable: 1,
            method: 0,
            address: 0x0002_0000,
            zsm: 0,
            mask: 0,
        };

        let start = buffer.as_mut_ptr();
        let end = unsafe {
            let next = draw::draw_framebuffer(start, 1, &raw mut frame);
            draw::draw_zbuffer(next, 1, &raw mut depth)
        };

        let written = advance(start, end);
        check_eq!(written, 4, "two chained builders wrote {written} qwords");
        if written != 4 {
            return;
        }

        // The second pair has to start where the first ended, which is the whole
        // contract of the returned pointer.
        check_eq!(
            half(&buffer[2], 1),
            GIF_DATA,
            "chained zbuffer pair lost its GIF register"
        );
        check_eq!(half(&buffer[1], 1), GS_FRAME + 1, "first pair is not FRAME");
        check_eq!(half(&buffer[3], 1), GS_ZBUF + 1, "second pair is not ZBUF");

        check_eq!(
            half(&buffer[3], 0) & 0x1FF,
            block_pointer(depth.address),
            "zbuffer address did not survive the chain"
        );
        check_eq!(
            half(&buffer[0], 0),
            GIF_TAG,
            "chaining disturbed the first qword"
        );
    }

    /// Reading a `gsGlobal` field. gsKit allocates the struct with C `malloc`, but
    /// bindgen gave the Rust type `#[repr(align(64))]` (a GS cache line), so the
    /// pointer is *less* aligned than the type claims and a plain `(*ctx).Field`
    /// deref trips rustc's debug misaligned-deref check - which is a real FFI
    /// finding, not a false alarm. `addr_of!` + `read_unaligned` never forms a
    /// reference to the struct, so it stays honest.
    #[test_case]
    fn gskit_hires_context_describes_itself() {
        unsafe {
            let context = gskit::gsKit_hires_init_global();
            assert!(
                !context.is_null(),
                "gsKit_hires_init_global() returned null"
            );
            gskit::gsKit_init_screen(context);

            let width = addr_of!((*context).Width).read_unaligned();
            let height = addr_of!((*context).Height).read_unaligned();
            let mode = addr_of!((*context).Mode).read_unaligned();
            let psm = addr_of!((*context).PSM).read_unaligned();

            check!(width > 0, "hires context reported width {width}");
            check!(height > 0, "hires context reported height {height}");
            check!(
                mode == GS_MODE_NTSC as i16 || mode == GS_MODE_PAL as i16,
                "context mode {mode:#x} is neither NTSC nor PAL"
            );
            check!(
                psm == GS_PSM_CT32 as c_int
                    || psm == GS_PSM_CT24 as c_int
                    || psm == GS_PSM_CT16 as c_int,
                "unexpected pixel format {psm}"
            );
            check!(
                width >= 640,
                "hires width {width} is below standard resolution"
            );

            ps2sdk_sys::println!("  hires context: {width}x{height} mode={mode:#x} psm={psm}");

            gskit::gsKit_deinit_global(context);
        }
    }

    /// The GS ROM check decides the display mode gsKit picks, so it has to return
    /// something the emulator's BIOS actually claims.
    #[test_case]
    fn gskit_reports_a_console_region() {
        let rom = unsafe { gskit::gsKit_check_rom() };

        // gsKit's PS2_MODEL values are small non-negative enumerations; a negative
        // or absurd number means the ROM version string was not recognised.
        check!(rom >= 0, "gsKit_check_rom returned {rom}");
        check!(rom < 64, "gsKit_check_rom returned implausible model {rom}");
        ps2sdk_sys::println!("  gsKit ROM model: {rom}");
    }

    /// Boots a gsKit context and paints the screen burgundy, which is also the
    /// colour to look for in the PCSX2 window to know the GS path works.
    #[test_case]
    fn gskit_paints_the_screen() {
        unsafe {
            let context = gskit::gsKit_hires_init_global();
            assert!(
                !context.is_null(),
                "gsKit_hires_init_global() returned null"
            );

            gskit::gsKit_init_screen(context);
            gskit::gsKit_clear(context, rgba(0x80, 0x1A, 0x2B, 0x80));
            gskit::gsKit_queue_exec(context);
            gskit::gsKit_sync_flip(context);
            gskit::gsKit_deinit_global(context);
        }
    }

    /// Clearing and flipping repeatedly is the game's inner loop; a context that
    /// survives one frame proves very little.
    #[test_case]
    fn gskit_clear_and_flip_is_repeatable() {
        unsafe {
            let context = gskit::gsKit_hires_init_global();
            assert!(
                !context.is_null(),
                "gsKit_hires_init_global() returned null"
            );
            gskit::gsKit_init_screen(context);

            for frame in 0..4u64 {
                gskit::gsKit_clear(context, rgba(0x20 * frame, 0x40, 0x60, 0x80));
                gskit::gsKit_queue_exec(context);
                gskit::gsKit_sync_flip(context);
            }

            gskit::gsKit_deinit_global(context);
        }
    }

    /// Blocking on vertical blank is how the game paces itself. This is the first
    /// test that waits on hardware, so it sits after everything that cannot block.
    ///
    /// If the GS never interrupts, this hangs and the runner's timeout is what
    /// reports it.
    #[test_case]
    fn vsync_wait_spends_a_frame() {
        const TICKS_PER_SECOND: u64 = 294_912_000;
        // A 60Hz frame is ~4.9M ticks, a 50Hz PAL frame ~5.9M. Accept anything over
        // a quarter of a 60Hz frame so a mid-frame start is not mistaken for a
        // no-op return.
        const AT_LEAST: u64 = TICKS_PER_SECOND / 240;

        let before = TimerCount::load().value();
        unsafe { gskit::gsKit_vsync_wait() };
        let after = TimerCount::load().value();

        let ticks = after.wrapping_sub(before) as u64;
        check!(
            ticks > AT_LEAST,
            "gsKit_vsync_wait returned after {ticks} ticks, expected more than {AT_LEAST}"
        );
    }
}

//
// EE kernel threading...
//

/// EE kernel threads. These are cooperative: a new thread only runs when the
/// current one blocks, so `SleepThread`/`WaitSema` are load-bearing rather than
/// incidental. That also makes this the module where a bug costs the run its
/// timeout instead of failing a line, which is why it is last.
#[cfg(target_vendor = "sony")]
mod threads {
    use core::ffi::c_void;
    use core::hint::spin_loop;
    use core::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

    use ps2sdk_sys::kernel::{
        CreateSema, CreateThread, DeleteSema, DeleteThread, ExitThread, GetThreadId, PollSema,
        SignalSema, StartThread, WaitSema, ee_sema_t, ee_thread_t, iSignalSema,
    };

    /// 16KB, plenty for a thread that does arithmetic.
    const STACK_SIZE: usize = 16 * 1024;

    #[repr(C, align(16))]
    struct Stack([u8; STACK_SIZE]);

    static CHILD_STACK: Stack = Stack([0; STACK_SIZE]);
    static PING_STACK: Stack = Stack([0; STACK_SIZE]);
    static PONG_STACK: Stack = Stack([0; STACK_SIZE]);

    /// The kernel saves and restores `$28` per thread, so a thread started with the
    /// wrong GP would address its statics through the wrong data area. Passing the
    /// current GP is correct whether or not this image uses GP-relative addressing.
    fn current_gp() -> *mut c_void {
        let gp: usize;
        unsafe {
            core::arch::asm!(
                "move {}, $28",
                out(reg) gp,
                options(nomem, nostack, preserves_flags)
            );
        }
        gp as *mut c_void
    }

    fn thread_struct(
        func: extern "C" fn(*mut c_void) -> i32,
        stack: &'static Stack,
    ) -> ee_thread_t {
        ee_thread_t {
            status: 0,
            func: func as usize as *mut c_void,
            stack: stack.0.as_ptr() as *mut c_void,
            stack_size: STACK_SIZE as i32,
            gp_reg: current_gp(),
            initial_priority: 100,
            current_priority: 100,
            attr: 0,
            option: 0,
        }
    }

    fn create_sema(initial: i32, max: i32) -> i32 {
        let mut sema = ee_sema_t {
            count: initial,
            max_count: max,
            init_count: initial,
            wait_threads: 0,
            attr: 0,
            option: 0,
        };

        unsafe { CreateSema(&raw mut sema) }
    }

    /// A thread that never runs should still be creatable and deletable; this is the
    /// cheap half of the threading contract, and it cannot block.
    #[test_case]
    fn thread_can_be_created_and_deleted_without_running() {
        extern "C" fn never(_arg: *mut c_void) -> i32 {
            0
        }

        let mut thread = thread_struct(never, &CHILD_STACK);
        let id = unsafe { CreateThread(&raw mut thread) };
        check!(id >= 0, "CreateThread returned {id}");

        if id >= 0 {
            check!(
                unsafe { DeleteThread(id) } >= 0,
                "DeleteThread({id}) failed"
            );
        }
    }

    /// `GetThreadId` is how a thread identifies itself to the kernel; it must be
    /// stable, and the main thread is not thread 0.
    #[test_case]
    fn main_thread_has_a_stable_id() {
        let first = unsafe { GetThreadId() };
        let second = unsafe { GetThreadId() };

        check!(
            first >= 0,
            "GetThreadId returned {first} for the main thread"
        );
        check_eq!(second, first, "GetThreadId disagreed with itself");
    }

    /// A semaphore created empty must refuse a `PollSema` until someone signals it,
    /// and must grant exactly one take per signal. `PollSema` is the only
    /// non-blocking probe, which is what makes both directions safe to assert.
    #[test_case]
    fn semaphore_takes_exactly_what_was_signalled() {
        let sema = create_sema(0, 4);
        check!(sema >= 0, "CreateSema returned {sema}");
        if sema < 0 {
            return;
        }

        // PollSema is syscall 0x45 and kernel.h declares it without documenting the
        // return value, so the contract asserted here is the observed one: a
        // negative result refuses, a non-negative one takes the token. Assuming 0
        // for "unavailable" is what this test originally did and it is wrong.
        let empty = unsafe { PollSema(sema) };
        check!(
            empty < 0,
            "empty semaphore granted a take (PollSema -> {empty})"
        );

        unsafe { SignalSema(sema) };

        let first = unsafe { PollSema(sema) };
        check!(
            first >= 0,
            "signalled semaphore refused a take (PollSema -> {first})"
        );

        let second = unsafe { PollSema(sema) };
        check!(
            second < 0,
            "semaphore allowed two takes from one signal (PollSema -> {second})"
        );

        ps2sdk_sys::println!();
        ps2sdk_sys::println!("  PollSema: refuse={empty}, grant={first}, refuse-again={second}");

        unsafe { DeleteSema(sema) };
    }

    /// `iSignalSema` is the variant callable from an interrupt handler; from thread
    /// context it must behave identically.
    #[test_case]
    fn semaphore_accepts_the_interrupt_variant_signal() {
        let sema = create_sema(0, 4);
        check!(sema >= 0, "CreateSema returned {sema}");
        if sema < 0 {
            return;
        }

        unsafe { iSignalSema(sema) };
        check!(
            unsafe { PollSema(sema) } >= 0,
            "iSignalSema did not make a token available"
        );

        unsafe { DeleteSema(sema) };
    }

    static JOIN_SEMA: AtomicI32 = AtomicI32::new(-1);
    static CHILD_SAW_ARG: AtomicUsize = AtomicUsize::new(0);
    static CHILD_RAN: AtomicBool = AtomicBool::new(false);

    extern "C" fn joiner(arg: *mut c_void) -> i32 {
        CHILD_SAW_ARG.store(arg as usize, Ordering::Release);
        CHILD_RAN.store(true, Ordering::Release);

        unsafe { SignalSema(JOIN_SEMA.load(Ordering::Relaxed)) };

        // ps2sdk threads do not return; ExitThread retires the thread. If it ever
        // did return, spinning is safer than falling off the end.
        unsafe { ExitThread() };
        loop {
            spin_loop();
        }
    }

    /// The full handshake: start a thread, block until it signals, check it saw its
    /// argument. This is the smallest test that proves the EE kernel scheduler runs
    /// at all.
    #[test_case]
    fn thread_runs_and_joins_via_semaphore() {
        const MARKER: usize = 0xCAFE;

        let sema = create_sema(0, 1);
        check!(sema >= 0, "CreateSema returned {sema}");
        if sema < 0 {
            return;
        }
        JOIN_SEMA.store(sema, Ordering::Relaxed);

        let mut thread = thread_struct(joiner, &CHILD_STACK);
        let id = unsafe { CreateThread(&raw mut thread) };
        check!(id >= 0, "CreateThread returned {id}");
        if id < 0 {
            unsafe { DeleteSema(sema) };
            return;
        }

        let started = unsafe { StartThread(id, MARKER as *mut c_void) };
        check!(started >= 0, "StartThread returned {started}");
        if started < 0 {
            unsafe {
                DeleteThread(id);
                DeleteSema(sema);
            }
            return;
        }

        // Cooperative scheduler: the child only runs because we block here.
        unsafe { WaitSema(sema) };

        check!(CHILD_RAN.load(Ordering::Acquire), "child thread never ran");
        check_eq!(
            CHILD_SAW_ARG.load(Ordering::Acquire),
            MARKER,
            "the argument did not survive StartThread"
        );

        unsafe {
            DeleteThread(id);
            DeleteSema(sema);
        }
    }

    static PING_SEMA: AtomicI32 = AtomicI32::new(-1);
    static PONG_SEMA: AtomicI32 = AtomicI32::new(-1);
    static DONE_SEMA: AtomicI32 = AtomicI32::new(-1);
    static PING_COUNT: AtomicUsize = AtomicUsize::new(0);
    static PONG_COUNT: AtomicUsize = AtomicUsize::new(0);
    const ROUNDS: usize = 50;

    /// One of two threads; `replies` picks which end of the token ring it sits at.
    extern "C" fn ponker(replies: bool) -> i32 {
        for _ in 0..ROUNDS {
            if replies {
                unsafe { WaitSema(PING_SEMA.load(Ordering::Relaxed)) };
                PONG_COUNT.fetch_add(1, Ordering::Relaxed);
                unsafe { SignalSema(PONG_SEMA.load(Ordering::Relaxed)) };
            } else {
                PING_COUNT.fetch_add(1, Ordering::Relaxed);
                unsafe { SignalSema(PING_SEMA.load(Ordering::Relaxed)) };
                unsafe { WaitSema(PONG_SEMA.load(Ordering::Relaxed)) };
            }
        }

        unsafe { SignalSema(DONE_SEMA.load(Ordering::Relaxed)) };
        unsafe { ExitThread() };
        loop {
            spin_loop();
        }
    }

    extern "C" fn ping(_arg: *mut c_void) -> i32 {
        ponker(false)
    }

    extern "C" fn pong(_arg: *mut c_void) -> i32 {
        ponker(true)
    }

    /// Two threads passing a token back and forth. This is the test that would
    /// notice a scheduler that loses wakeups or mis-orders ready threads, which a
    /// single round-trip cannot reveal.
    ///
    /// Last in the file, and deliberately so: if a wakeup is ever lost this blocks,
    /// and the runner's timeout is what reports it. Everything before it has already
    /// been printed by then.
    #[test_case]
    fn two_threads_exchange_a_token() {
        let ping_sema = create_sema(0, 1);
        let pong_sema = create_sema(1, 1);
        let done_sema = create_sema(0, 2);

        check!(
            ping_sema >= 0 && pong_sema >= 0 && done_sema >= 0,
            "CreateSema failed: ping={ping_sema} pong={pong_sema} done={done_sema}"
        );
        if ping_sema < 0 || pong_sema < 0 || done_sema < 0 {
            return;
        }

        PING_SEMA.store(ping_sema, Ordering::Relaxed);
        PONG_SEMA.store(pong_sema, Ordering::Relaxed);
        DONE_SEMA.store(done_sema, Ordering::Relaxed);
        PING_COUNT.store(0, Ordering::Relaxed);
        PONG_COUNT.store(0, Ordering::Relaxed);

        let mut ping_thread = thread_struct(ping, &PING_STACK);
        let mut pong_thread = thread_struct(pong, &PONG_STACK);

        let ping_id = unsafe { CreateThread(&raw mut ping_thread) };
        let pong_id = unsafe { CreateThread(&raw mut pong_thread) };
        check!(
            ping_id >= 0 && pong_id >= 0,
            "CreateThread: ping={ping_id} pong={pong_id}"
        );
        if ping_id < 0 || pong_id < 0 {
            return;
        }

        // Pong starts holding the token (init_count 1), so both threads can begin.
        unsafe {
            StartThread(ping_id, core::ptr::null_mut());
            StartThread(pong_id, core::ptr::null_mut());
        }

        // One completion per thread.
        unsafe {
            WaitSema(done_sema);
            WaitSema(done_sema);
        }

        check_eq!(
            PING_COUNT.load(Ordering::Acquire),
            ROUNDS,
            "ping thread iteration count"
        );
        check_eq!(
            PONG_COUNT.load(Ordering::Acquire),
            ROUNDS,
            "pong thread iteration count"
        );

        unsafe {
            DeleteThread(ping_id);
            DeleteThread(pong_id);
            DeleteSema(ping_sema);
            DeleteSema(pong_sema);
            DeleteSema(done_sema);
        }
    }
}
