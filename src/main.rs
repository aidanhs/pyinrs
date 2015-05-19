extern crate libc;
extern crate python27_sys as py;

#[cfg(feature = "wrap")]
#[path = "wrap.rs"]
pub mod backend;

#[cfg(feature = "dump")]
#[path = "dump.rs"]
pub mod backend;

use std::ffi::CString;
use std::ptr;
use std::env;

pub const PYTHONLIB: &'static [u8] = include_bytes!("../libpython2.7.zip");
pub const PYTHONLIBTARGET: &'static str = "/tmp/pyinrs-libpython2.7.zip";

#[cfg(any(feature = "dump", feature = "wrap"))]
const VALIDMODE: bool = true;

fn main() {
    assert!(VALIDMODE);

    backend::prep();

    env::set_var("PYTHONPATH", PYTHONLIBTARGET);
    let pyhome_str = "";
    let pyhome_cstr = CString::new(pyhome_str.as_bytes()).unwrap();
    let cmd_str = "
import sys, base64
print base64.b64decode('SGVsbG8sIHB5dGhvbiE=')
print sys.path
";
    let cmd_cstr = CString::new(cmd_str.as_bytes()).unwrap();

    unsafe {
        py::Py_NoSiteFlag = 1;
        py::Py_NoUserSiteDirectory = 1;
        py::Py_DontWriteBytecodeFlag = 1;
        py::Py_SetPythonHome(pyhome_cstr.as_ptr() as *mut i8);
        py::Py_Initialize();
        //let mut flags = py::PyCompilerFlags { cf_flags: 0 };
        let flags = ptr::null_mut();
        py::PyRun_SimpleStringFlags(cmd_cstr.as_ptr(), flags);
    }
}
