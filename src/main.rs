extern crate pyinrs;
extern crate python27_sys as py;
extern crate uuid;
#[macro_use]
extern crate lazy_static;

#[link(name = "z")]
extern {}

use std::ffi::CString;
use std::ptr;
use std::env;

pub const PYTHONLIBNAME: &'static str = "libpython2.7.zip";
lazy_static!{
    pub static ref WORKDIR: String =
        format!("/tmp/pyinrs-{}", uuid::Uuid::new_v4().to_simple_string());
}

fn main() {
    pyinrs::prep(&*WORKDIR);
    env::set_var("PYTHONPATH", format!("{}/{}", &*WORKDIR, PYTHONLIBNAME));

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
