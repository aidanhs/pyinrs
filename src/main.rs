extern crate python27_sys as py;

use std::ffi::CString;
use std::ptr;

fn main() {
    let cmd_str = "print 'Hello, python!'";
    let cmd_cstr = CString::new(cmd_str.as_bytes()).unwrap();
    unsafe {
        py::Py_Initialize();
        //let mut flags = py::PyCompilerFlags { cf_flags: 0 };
        let flags = ptr::null_mut();
        py::PyRun_SimpleStringFlags(cmd_cstr.as_ptr(), flags);
    }
}
