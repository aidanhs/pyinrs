extern crate pyinrs;
extern crate python27_sys as py;
extern crate uuid;
extern crate libc;
#[macro_use]
extern crate lazy_static;

#[link(name = "z")]
extern {}

use std::ffi::CString;
use std::ptr;
use std::env;
use libc::{c_char, c_int};

pub const PYTHONLIBNAME: &'static str = "libpython2.7.zip";
lazy_static!{
    pub static ref WORKDIR: String =
        format!("/tmp/pyinrs-{}", uuid::Uuid::new_v4().to_simple_string());
}

extern {
    fn PySys_SetArgvEx(argc: c_int, argv: *mut *mut c_char, updatepath: c_int);
}

fn main() {
    pyinrs::prep(&*WORKDIR);
    env::set_var("PYTHONPATH", format!("{}/{}", &*WORKDIR, PYTHONLIBNAME));
    let args: Vec<String> = env::args().collect();

    let pyhome_str = "";
    let pyhome_cstr = CString::new(pyhome_str.as_bytes()).unwrap();
    let cmd_str = format!("
import sys
workdir = '{}'
#import socket
#import _socket
#print socket.gethostname()
#print _socket.gethostname()
#import wave
#import base64
#sys.exit(1)
sys.path.insert(0, workdir + '/dep')
sys.path.insert(0, workdir) # shutit uses sys.path[0] as file location
import shutit_util
shutit_util._default_cnf = shutit_util._default_cnf.replace(
    'shutit_module_path:',
    'shutit_module_path:' + workdir + '/library:'
) # such a hack
import shutit_main
shutit_main.main()
", *WORKDIR);
    let cmd_cstr = CString::new(cmd_str.as_bytes()).unwrap();

    let mut cstr_args: Vec<CString> = vec![];
    for arg in args.iter() {
        cstr_args.push(CString::new(arg.as_bytes()).unwrap());
    }
    let mut ptr_args: Vec<*const c_char> = vec![];
    for arg in cstr_args.iter() {
        ptr_args.push(arg.as_ptr());
    }

    unsafe {
        py::Py_NoSiteFlag = 1;
        py::Py_NoUserSiteDirectory = 1;
        py::Py_DontWriteBytecodeFlag = 1;
        py::Py_SetPythonHome(pyhome_cstr.as_ptr() as *mut i8);
        py::Py_Initialize();
        PySys_SetArgvEx(ptr_args.len() as c_int, ptr_args.as_ptr() as *mut *mut c_char, 0);
        //let mut flags = py::PyCompilerFlags { cf_flags: 0 };
        let flags = ptr::null_mut();
        py::PyRun_SimpleStringFlags(cmd_cstr.as_ptr(), flags);
    }
}
