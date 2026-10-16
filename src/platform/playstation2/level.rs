//! The level, loaded on the EE. The data and its formats live in `shared::level`; this module supplies the bytes: the
//! host files that `build/fixtures.rs` writes into the extracted tree, read through PCSX2's `host:` device, with the
//! embedded slice as the fallback.

extern crate alloc;

use alloc::vec::Vec;

use crate::shared::level::Level;

/// The embedded slice (`build/fixtures.rs`), used when no host file is there.
mod data {
    include!(concat!(env!("OUT_DIR"), "/rc_formats_fixture.rs"));
}

/// The level's tfrags, written by `build/fixtures.rs` `write_tfrag_level`. `host:` reaches it through the `levels` link
/// the `ps2-build` recipe puts beside the ELF.
const HOST_TFRAGS: &str = "host:levels/00/level0_tfrags.bin";

/// The flat triangle list (`write_full_level`), the fallback when the tfrag file is missing.
const HOST_LEVEL: &str = "host:levels/00/level0_tris.bin";

/// Loads the level: the tfrag file if it opens and parses; otherwise the flat list; otherwise the embedded slice.
/// Returns the reason it fell back, for the caller to report.
pub fn load() -> (Level, Option<&'static str>) {
    let why = match read_host(HOST_TFRAGS) {
        Some(bytes) => match Level::parse_tfrags(&bytes) {
            Some(level) => return (level, None),
            None => "tfrag file is malformed",
        },
        None => "tfrag file did not open",
    };
    if let Some(level) = read_host(HOST_LEVEL).and_then(|b| Level::parse_flat(&b)) {
        return (level, Some(why));
    }
    let embedded = Level::parse_flat(data::LEVEL0_MESH)
        .unwrap_or(Level { base: 0.0, tfrags: Vec::new(), triangles: 0 });
    (embedded, Some(why))
}

/// Reads a whole host file through `fioRead` in 64 KB pieces. `None` if it does not open.
fn read_host(path: &str) -> Option<Vec<u8>> {
    use ps2sdk_sys::fileio::{fioClose, fioInit, fioLseek, fioOpen, fioRead, FIO_O_RDONLY, FIO_SEEK_END, FIO_SEEK_SET};

    const CHUNK: usize = 64 * 1024;
    let mut name = alloc::vec::Vec::with_capacity(path.len() + 1);
    name.extend_from_slice(path.as_bytes());
    name.push(0);

    unsafe {
        fioInit();
        let fd = fioOpen(name.as_ptr().cast(), FIO_O_RDONLY as i32);
        if fd < 0 {
            return None;
        }
        let size = fioLseek(fd, 0, FIO_SEEK_END as i32);
        if size <= 0 || fioLseek(fd, 0, FIO_SEEK_SET as i32) != 0 {
            fioClose(fd);
            return None;
        }
        let mut bytes = alloc::vec![0u8; size as usize];
        let mut done = 0usize;
        while done < bytes.len() {
            let want = (bytes.len() - done).min(CHUNK);
            let got = fioRead(fd, bytes[done..].as_mut_ptr().cast(), want as i32);
            if got <= 0 {
                fioClose(fd);
                return None;
            }
            done += got as usize;
        }
        fioClose(fd);
        Some(bytes)
    }
}
