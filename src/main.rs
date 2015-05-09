extern crate python27_sys as py;

use std::ffi::CString;
use std::ptr;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::env;

const PYTHONLIB: &'static [u8] = include_bytes!("../libpython2.7.zip");
const PYTHONLIBTARGET: &'static str = "/tmp/pyinrs-libpython2.7.zip";

fn main() {
    let path = Path::new(PYTHONLIBTARGET);
    let mut f = File::create(path).unwrap();
    f.write_all(PYTHONLIB).unwrap();

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
