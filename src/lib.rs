#![feature(path_relative_from)]
#![feature(std_misc)]

extern crate libc;
extern crate phf;
#[macro_use]
extern crate lazy_static;

use std::rt;

// Defines static FILES: phf::Map<&'static str, &'static [u8]>
//         static DIRS:  phf::Set<&'static str>
include!("../include.files");

#[cfg(any(feature = "dump", feature = "wrap"))]
const VALIDMODE: bool = true;

static mut WORKDIR: &'static str = "";

pub fn prep(workdir: &'static str) {
    assert!(VALIDMODE);
    unsafe { WORKDIR = workdir };

    backend::prep();
    rt::at_exit(backend::atexit).unwrap();
}

#[cfg(feature = "wrap")]
#[path = "wrap.rs"]
pub mod backend;

#[cfg(feature = "dump")]
#[path = "dump.rs"]
pub mod backend;
