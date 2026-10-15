use std::{
    env,
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Assembly sources, each assembled to its own object and archived together.
///
/// Listing them explicitly (rather than reading `src/`) is what makes the `rerun-if-changed`
/// directives below precise; a new file that is not added here simply does not get assembled.
///
/// Keeping the bootstrap (`start.S`) separate from the CoP0/atomic helpers matters: an archive
/// member is only pulled in as a whole, so bundling `_start` with the register accessors would
/// drag PruSSia's crt0 - and the `START_OF_BSS`/`END_OF_BSS` linker symbols it needs - into any
/// program that merely wants to read the Count register.
const ASM_SRCS: &[&str] = &["start.S", "cop0.S", "atomic.S"];

/// Assembler flags, read from `asm.flags` so that this and `rebuild-asm-lib.sh` cannot disagree.
///
/// They have to match what the rest of the EE link is built with: the ps2dev target spec passes
/// `-mabi=n32 -EL` and the PS2SDK objects carry the same ELF ABI bits. Get this wrong and ld
/// refuses to mix o32 and n32 objects, which it does late and loudly at link time rather than
/// here.
const ASM_FLAGS: &str = include_str!("asm.flags");

const LIB_NAME: &str = "prussia-rt";

/// Assemblers to try, in order, when `PRUSSIA_RT_AS` is unset.
const ASSEMBLERS: &[&str] = &["mips64r5900el-ps2-elf-as", "mips64el-none-elf-as"];

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=asm.flags");
    for src in ASM_SRCS {
        println!("cargo:rerun-if-changed=src/{src}");
    }
    println!("cargo:rerun-if-env-changed=PRUSSIA_RT_AS");
    println!("cargo:rerun-if-env-changed=PRUSSIA_RT_AR");

    // The link search path has to be known before anything can find the archive.
    println!("cargo:rustc-link-search={}", out_dir.display());

    // Put `linkfile.ld` in the build directory, for programs that boot through PruSSia.
    fs::File::create(out_dir.join("linkfile.ld"))?.write_all(include_bytes!("user-linkfile.ld"))?;

    if let Err(err) = assemble(&manifest_dir, &out_dir) {
        // The archive checked into this crate is a stand-in so that host-only builds (which never
        // link it) still work. It was assembled by whoever last ran rebuild-asm-lib.sh, and
        // nothing guarantees its ABI matches the current link, so falling back to it while
        // building for the EE is how you discover the problem three crates later, as
        // "ABI is incompatible with that of the selected emulation" from ld. Only fall back where
        // the archive cannot be reached.
        if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("mips64") {
            return Err(format!(
                "cannot assemble {LIB_NAME} for the EE: {err}\n\
                 (need a MIPS R5900 assembler on PATH, or set PRUSSIA_RT_AS)"
            )
            .into());
        }

        println!("cargo:rerun-if-changed=lib{LIB_NAME}.a");
        fs::copy(
            manifest_dir.join(format!("lib{LIB_NAME}.a")),
            out_dir.join(format!("lib{LIB_NAME}.a")),
        )?;
    }

    link_flags(&out_dir);

    Ok(())
}

/// The link directives, whichever way the archive was produced.
///
/// The link library is exposed as metadata as well as asked for directly, on purpose:
/// `rustc-link-lib` from a dependency lands in the middle of the linker's library list, which is
/// before the objects that reference these symbols. A program that wants the archive searched
/// late has to re-emit it as a `rustc-link-arg`, which is what the downstream crate's build.rs
/// does with `link_search`/`link_lib`.
fn link_flags(out_dir: &Path) {
    println!("cargo:rustc-link-lib=static={LIB_NAME}");
    println!("cargo:link_search={}", out_dir.display());
    println!(
        "cargo:linkfile_path={}",
        out_dir.join("linkfile.ld").display()
    );
    println!("cargo:link_lib={LIB_NAME}");
}

/// Assembles `ASM_SRCS` into `out_dir` and archives the objects as `lib{LIB_NAME}.a`.
fn assemble(manifest_dir: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let assembler = find_assembler()?;

    let mut objects = Vec::new();
    for src in ASM_SRCS {
        let source = manifest_dir.join("src").join(src);
        let object = out_dir.join(src.replace(".S", ".o"));

        // Capture rather than inherit: cargo hides a build script's stderr unless the script
        // fails, and this one is designed to succeed while falling back. Whatever the assembler
        // has to say therefore has to ride along in the error message.
        let output = Command::new(&assembler)
            .args(ASM_FLAGS.split_whitespace())
            .arg("-o")
            .arg(&object)
            .arg(&source)
            .output()?;

        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "{} failed on {} ({})\n  source: {}\n  {}",
                assembler.display(),
                src,
                output.status,
                source.display(),
                detail.trim(),
            )
            .into());
        }

        objects.push(object);
    }

    // Recreate the archive from scratch, so a stale member can never survive a rename.
    let archive = out_dir.join(format!("lib{LIB_NAME}.a"));
    let _ = fs::remove_file(&archive);

    for archiver in archivers_for(&assembler) {
        let status = Command::new(&archiver)
            .arg("crs")
            .arg(&archive)
            .args(&objects)
            .status();

        match status {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                let _ = fs::remove_file(&archive);
                return Err(format!("{} failed ({status})", archiver.display()).into());
            }
            // Not installed; try the next candidate rather than giving up on the whole build.
            Err(_) => continue,
        }
    }

    Err("no archiver found (set PRUSSIA_RT_AR)".into())
}

/// The first usable MIPS assembler, or the one `PRUSSIA_RT_AS` names.
fn find_assembler() -> Result<PathBuf, Box<dyn Error>> {
    let candidates = env::var("PRUSSIA_RT_AS")
        .ok()
        .map(|explicit| vec![explicit])
        .unwrap_or_else(|| ASSEMBLERS.iter().map(|name| name.to_string()).collect());

    for candidate in &candidates {
        // Probe quietly; a candidate that is not installed, or is the wrong binutils, is simply
        // not the assembler we are going to use.
        let status = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if matches!(status, Ok(status) if status.success()) {
            return Ok(PathBuf::from(candidate));
        }
    }

    Err(format!(
        "no MIPS assembler found (looked for {})",
        candidates.join(", ")
    )
    .into())
}

/// Archivers to try for `assembler`: one named by `PRUSSIA_RT_AR`, the binutils sibling, then
/// whatever `ar` is on the path.
fn archivers_for(assembler: &Path) -> Vec<PathBuf> {
    let mut archivers = Vec::new();

    if let Ok(explicit) = env::var("PRUSSIA_RT_AR") {
        archivers.push(PathBuf::from(explicit));
    }

    let name = assembler
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    if let Some(prefix) = name.strip_suffix("as") {
        // Either a sibling of an absolute path, or a bare name resolved through PATH.
        archivers.push(assembler.with_file_name(format!("{prefix}ar")));
    }

    archivers.push(PathBuf::from("ar"));
    archivers
}
