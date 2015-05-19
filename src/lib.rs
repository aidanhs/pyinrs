extern crate libc;

pub const PYTHONLIB: &'static [u8] = include_bytes!("../libpython2.7.zip");
pub const PYTHONLIBTARGET: &'static str = "/tmp/pyinrs-libpython2.7.zip";

#[cfg(feature = "wrap")]
#[path = "wrap.rs"]
pub mod backend;

#[cfg(feature = "dump")]
#[path = "dump.rs"]
pub mod backend;
