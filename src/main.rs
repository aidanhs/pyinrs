extern crate pyinrs;
extern crate python27_sys as py;

#[link(name = "z")]
extern {}

use std::ffi::CString;
use std::ptr;

fn main() {
    pyinrs::prep();

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
