#![feature(path_ext)]
#![feature(std_misc)]

extern crate libc;
extern crate phf;

use std::env;
use std::rt;

pub const PYTHONLIB: &'static [u8] = include_bytes!("../libpython2.7.zip");
pub const PYTHONLIBTARGET: &'static str = "/tmp/pyinrs-libpython2.7.zip";

// Defines static FILES: phf::Map<&'static str, &'static [u8]>
include!("../include.files");
pub const FILESTARGET: &'static str = "/tmp/pyinrs-dump";

#[cfg(any(feature = "dump", feature = "wrap"))]
const VALIDMODE: bool = true;

pub fn prep() {
    assert!(VALIDMODE);

    env::set_var("PYTHONPATH", PYTHONLIBTARGET);
    backend::prep();
    rt::at_exit(backend::atexit).unwrap();
}

#[cfg(feature = "wrap")]
#[path = "wrap.rs"]
pub mod backend;

#[cfg(feature = "dump")]
#[path = "dump.rs"]
pub mod backend;
