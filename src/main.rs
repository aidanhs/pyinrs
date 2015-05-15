extern crate libc;
extern crate python27_sys as py;

use std::ffi::{CString, CStr};
use std::ptr;
use std::mem;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::env;
use libc::{c_void, c_int, c_long, c_char, size_t, ssize_t, off_t, fpos_t};

const PYTHONLIB: &'static [u8] = include_bytes!("../libpython2.7.zip");
const PYTHONLIBTARGET: &'static str = "/tmp/pyinrs-libpython2.7.zip";

fn main() {
    let path = Path::new(PYTHONLIBTARGET);
    // TODO: make this an option
    //let mut f = File::create(path).unwrap();
    //f.write_all(PYTHONLIB).unwrap();

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

// ================

// memfd_create in kernel 3.17

const PYTHONLIB_FD: c_int = 100000;
const PYTHONLIB_FILE: *mut libc::FILE = 1 as *mut libc::FILE;
static mut PYTHONLIB_FILE_EOF: bool = false;
static mut PYTHONLIB_OFF: usize = 0;
// TODO: no function if .len() was compile-time permissible
fn get_pythonlib_stat_struct() -> libc::stat {
let PYTHONLIB_STAT: libc::stat = libc::stat {
    st_dev: 100000, // arbitrary
    st_ino: 1,
    st_mode: 0o100444, // normal file, read only
    st_nlink: 1,
    st_uid: 1,
    st_gid: 1,
    __pad0: 0, // ???
    st_rdev: 0, // arbitrary
    st_size: PYTHONLIB.len() as ssize_t,
    st_blksize: 4096,
    st_blocks: ((PYTHONLIB.len() + 1024) / 512) as ssize_t,
    st_atime: 0,
    st_atime_nsec: 0,
    st_mtime: 0,
    st_mtime_nsec: 0,
    st_ctime: 0,
    st_ctime_nsec: 0,
    __unused: [0, 0, 0],
};
PYTHONLIB_STAT
}

extern {
    fn __real_fclose(fp: *mut libc::FILE) -> c_int;
    fn __real_fopen(path: *const c_char, mode: *const c_char) -> *mut libc::FILE;
    fn __real_fopen64(path: *const c_char, mode: *const c_char) -> *mut libc::FILE;
    fn __real_fdopen(fd: c_int, mode: *const c_char) -> *mut libc::FILE;
    fn __real_fdopen64(fd: c_int, mode: *const c_char) -> *mut libc::FILE;
    fn __real_freopen(path: *const c_char, mode: *const c_char, stream: *mut libc::FILE) -> *mut libc::FILE;
    fn __real_freopen64(path: *const c_char, mode: *const c_char, stream: *mut libc::FILE) -> *mut libc::FILE;
    fn __real_fread(ptr: *mut c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t;
    fn __real_fread64(ptr: *mut c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t;
    fn __real_fwrite(ptr: *const c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t;
    fn __real_fwrite64(ptr: *const c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t;
    fn __real_fgetc(stream: *mut libc::FILE) -> c_int;
    fn __real_fgets(s: *mut c_char, size: c_int, stream: *mut libc::FILE) -> c_int;
    fn __real_getc(stream: *mut libc::FILE) -> c_int;
    fn __real__IO_getc(stream: *mut libc::FILE) -> c_int;
    // getchar - stdin only
    // gets - stdin only
    fn __real_ungetc(c: c_int, stream: *mut libc::FILE) -> c_int;
    fn __real_fseek(stream: *mut libc::FILE, offset: c_long, whence: c_int) -> c_int;
    fn __real_fseek64(stream: *mut libc::FILE, offset: c_long, whence: c_int) -> c_int;
    fn __real_fseeko(stream: *mut libc::FILE, offset: off_t, whence: c_int) -> c_int;
    fn __real_fseeko64(stream: *mut libc::FILE, offset: off_t, whence: c_int) -> c_int;
    fn __real_ftell(stream: *mut libc::FILE) -> c_long;
    fn __real_ftell64(stream: *mut libc::FILE) -> c_long;
    fn __real_ftello(stream: *mut libc::FILE) -> off_t;
    fn __real_ftello64(stream: *mut libc::FILE) -> off_t;
    fn __real_rewind(stream: *mut libc::FILE);
    fn __real_fgetpos(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int;
    fn __real_fgetpos64(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int;
    fn __real_fsetpos(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int;
    fn __real_fsetpos64(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int;
    fn __real_clearerr(stream: *mut libc::FILE);
    fn __real_feof(stream: *mut libc::FILE) -> c_int;
    fn __real_ferror(stream: *mut libc::FILE) -> c_int;
    fn __real_fileno(stream: *mut libc::FILE) -> c_int;

    fn __real_dup(oldfd: c_int) -> c_int;
    fn __real_dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn __real_dup3(oldfd: c_int, newfd: c_int, flags: c_int) -> c_int;
    fn __real_read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    fn __real_open(pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int;
    fn __real_open64(pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int;
    fn __real_pread(fd: c_int, buf: *mut c_void, count: size_t, offset: off_t) -> ssize_t;
    fn __real_pread64(fd: c_int, buf: *mut c_void, count: size_t, offset: off_t) -> ssize_t;
    fn __real_pwrite(fd: c_int, buf: *const c_void, count: size_t, offset: off_t) -> ssize_t;
    fn __real_pwrite64(fd: c_int, buf: *const c_void, count: size_t, offset: off_t) -> ssize_t;
    fn __real_lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    fn __real_lseek64(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    fn __real_stat(path: *const c_char, buf: *mut libc::stat) -> c_int;
    fn __real_stat64(path: *const c_char, buf: *mut libc::stat) -> c_int;
    fn __real___xstat(ver: c_int, path: *const c_char, buf: *mut libc::stat) -> c_int;
    fn __real___xstat64(ver: c_int, path: *const c_char, buf: *mut libc::stat) -> c_int;
    fn __real_lstat(path: *const c_char, buf: *mut libc::stat) -> c_int;
    fn __real_lstat64(path: *const c_char, buf: *mut libc::stat) -> c_int;
    fn __real___lxstat(ver: c_int, path: *const c_char, buf: *mut libc::stat) -> c_int;
    fn __real___lxstat64(ver: c_int, path: *const c_char, buf: *mut libc::stat) -> c_int;
    fn __real_fstat(fd: c_int, buf: *mut libc::stat) -> c_int;
    fn __real_fstat64(fd: c_int, buf: *mut libc::stat) -> c_int;
    fn __real___fxstat(ver: c_int, fd: c_int, buf: *mut libc::stat) -> c_int;
    fn __real___fxstat64(ver: c_int, fd: c_int, buf: *mut libc::stat) -> c_int;
}

#[no_mangle]
pub unsafe extern fn __wrap_fclose(fp: *mut libc::FILE) -> c_int {
    if fp == PYTHONLIB_FILE {
        return 0
    }
    __real_fclose(fp)
}
#[no_mangle]
pub unsafe extern fn __wrap_fopen(path: *const c_char, mode: *const c_char) -> *mut libc::FILE {
    if CStr::from_ptr(path).to_bytes() == PYTHONLIBTARGET.as_bytes() {
        return PYTHONLIB_FILE
    }
    __real_fopen(path, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_fopen64(path: *const c_char, mode: *const c_char) -> *mut libc::FILE {
    __wrap_fopen(path, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_fdopen(fd: c_int, mode: *const c_char) -> *mut libc::FILE {
    if fd == PYTHONLIB_FD {
        panic!("fdopen")
    }
    __real_fdopen(fd, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_fdopen64(fd: c_int, mode: *const c_char) -> *mut libc::FILE {
    __wrap_fdopen(fd, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_freopen(path: *const c_char, mode: *const c_char, stream: *mut libc::FILE) -> *mut libc::FILE {
    if CStr::from_ptr(path).to_bytes() == PYTHONLIBTARGET.as_bytes() || stream == PYTHONLIB_FILE {
        panic!("freopen")
    }
    __real_freopen(path, mode, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_freopen64(path: *const c_char, mode: *const c_char, stream: *mut libc::FILE) -> *mut libc::FILE {
    __wrap_freopen(path, mode, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fread(ptr: *mut c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t {
    if stream == PYTHONLIB_FILE {
        // TODO: implement better
        let lenleft = PYTHONLIB.len() - PYTHONLIB_OFF;
        let wanted = (size * nmemb) as usize;
        let toread = if lenleft > wanted { wanted } else { lenleft - (lenleft % size as usize) };
        assert!(__wrap_read(PYTHONLIB_FD, ptr, toread as size_t) as usize == toread);
        return toread as size_t / size;
    }
    __real_fread(ptr, size, nmemb, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fread64(ptr: *mut c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t {
    __wrap_fread(ptr, size, nmemb, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fwrite(ptr: *const c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t {
    if stream == PYTHONLIB_FILE {
        panic!("fwrite")
    }
    __real_fwrite(ptr, size, nmemb, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fwrite64(ptr: *const c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t {
    __wrap_fwrite(ptr, size, nmemb, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fgetc(stream: *mut libc::FILE) -> c_int {
    if stream == PYTHONLIB_FILE {
        panic!("fgetc")
    }
    __real_fgetc(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fgets(s: *mut c_char, size: c_int, stream: *mut libc::FILE) -> c_int {
    if stream == PYTHONLIB_FILE {
        panic!("fgets")
    }
    __real_fgets(s, size, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_getc(stream: *mut libc::FILE) -> c_int {
    if stream == PYTHONLIB_FILE {
        if PYTHONLIB_OFF == PYTHONLIB.len() {
            return libc::EOF
        }
        let chr = PYTHONLIB[PYTHONLIB_OFF];
        PYTHONLIB_OFF += 1;
        return chr as c_int
    }
    __real_getc(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap__IO_getc(stream: *mut libc::FILE) -> c_int {
    if stream == PYTHONLIB_FILE {
        if PYTHONLIB_OFF == PYTHONLIB.len() {
            return libc::EOF
        }
        let chr = PYTHONLIB[PYTHONLIB_OFF];
        PYTHONLIB_OFF += 1;
        return chr as c_int
    }
    __real__IO_getc(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ungetc(c: c_int, stream: *mut libc::FILE) -> c_int {
    if stream == PYTHONLIB_FILE {
        panic!("ungetc")
    }
    __real_ungetc(c, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fseek(stream: *mut libc::FILE, offset: c_long, whence: c_int) -> c_int {
    if stream == PYTHONLIB_FILE {
        PYTHONLIB_FILE_EOF = false;
        return __wrap_lseek(PYTHONLIB_FD, offset as off_t, whence) as c_int
    }
    __real_fseek(stream, offset, whence)
}
#[no_mangle]
pub unsafe extern fn __wrap_fseek64(stream: *mut libc::FILE, offset: c_long, whence: c_int) -> c_int {
    __wrap_fseek(stream, offset, whence)
}
#[no_mangle]
pub unsafe extern fn __wrap_fseeko(stream: *mut libc::FILE, offset: off_t, whence: c_int) -> c_int {
    if stream == PYTHONLIB_FILE {
        panic!("fseeko")
    }
    __real_fseeko(stream, offset, whence)
}
#[no_mangle]
pub unsafe extern fn __wrap_fseeko64(stream: *mut libc::FILE, offset: off_t, whence: c_int) -> c_int {
    __wrap_fseeko(stream, offset, whence)
}
#[no_mangle]
pub unsafe extern fn __wrap_ftell(stream: *mut libc::FILE) -> c_long {
    if stream == PYTHONLIB_FILE {
        return PYTHONLIB_OFF as c_long
    }
    __real_ftell(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ftell64(stream: *mut libc::FILE) -> c_long {
    __wrap_ftell(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ftello(stream: *mut libc::FILE) -> off_t {
    if stream == PYTHONLIB_FILE {
        panic!("ftello")
    }
    __real_ftello(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ftello64(stream: *mut libc::FILE) -> off_t {
    __wrap_ftello(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_rewind(stream: *mut libc::FILE) {
    if stream == PYTHONLIB_FILE {
        panic!("rewind")
    }
    __real_rewind(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fgetpos(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int {
    if stream == PYTHONLIB_FILE {
        panic!("fgetpos")
    }
    __real_fgetpos(stream, pos)
}
#[no_mangle]
pub unsafe extern fn __wrap_fgetpos64(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int {
    __wrap_fgetpos(stream, pos)
}
#[no_mangle]
pub unsafe extern fn __wrap_fsetpos(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int {
    if stream == PYTHONLIB_FILE {
        panic!("fsetpos")
    }
    __real_fsetpos(stream, pos)
}
#[no_mangle]
pub unsafe extern fn __wrap_fsetpos64(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int {
    __wrap_fsetpos(stream, pos)
}
#[no_mangle]
pub unsafe extern fn __wrap_clearerr(stream: *mut libc::FILE) {
    if stream == PYTHONLIB_FILE {
        PYTHONLIB_FILE_EOF = false;
        return
    }
    __real_clearerr(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_feof(stream: *mut libc::FILE) -> c_int {
    if stream == PYTHONLIB_FILE {
        return if PYTHONLIB_FILE_EOF { 1 } else { 0 }
    }
    __real_feof(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ferror(stream: *mut libc::FILE) -> c_int {
    if stream == PYTHONLIB_FILE {
        return 0;
    }
    __real_ferror(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fileno(stream: *mut libc::FILE) -> c_int {
    if stream == PYTHONLIB_FILE {
        return PYTHONLIB_FD
    }
    __real_fileno(stream)
}



#[no_mangle]
pub unsafe extern fn __wrap_dup(oldfd: c_int) -> c_int {
    if oldfd == PYTHONLIB_FD { panic!("dup") }
    __real_dup(oldfd)
}
#[no_mangle]
pub unsafe extern fn __wrap_dup2(oldfd: c_int, newfd: c_int) -> c_int {
    if oldfd == PYTHONLIB_FD || newfd == PYTHONLIB_FD { panic!("dup2") }
    __real_dup2(oldfd, newfd)
}
#[no_mangle]
pub unsafe extern fn __wrap_dup3(oldfd: c_int, newfd: c_int, flags: c_int) -> c_int {
    if oldfd == PYTHONLIB_FD || newfd == PYTHONLIB_FD { panic!("dup3") }
    __real_dup3(oldfd, newfd, flags)
}
#[no_mangle]
pub unsafe extern fn __wrap_open(pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    if CStr::from_ptr(pathname).to_bytes() == PYTHONLIBTARGET.as_bytes() {
        // This violates the spec by not returning the lowest numbered
        // unused fd
        return PYTHONLIB_FD
    }
    __real_open(pathname, flags, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_open64(pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    __wrap_open(pathname, flags, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t {
    if fd == PYTHONLIB_FD {
        let buf = buf as *mut u8;
        let count = count as usize;
        let lenleft = PYTHONLIB.len() - PYTHONLIB_OFF;
        let num = if count < lenleft { count } else { lenleft };
        let pythonlib_offptr = (PYTHONLIB.as_ptr() as usize + PYTHONLIB_OFF) as *const u8;
        ptr::copy_nonoverlapping(pythonlib_offptr, buf, num);
        PYTHONLIB_OFF = PYTHONLIB_OFF + num;
        return num as ssize_t
    }
    __real_read(fd, buf, count)
}
#[no_mangle]
pub unsafe extern fn __wrap_pread(fd: c_int, buf: *mut c_void, count: size_t, offset: off_t) -> ssize_t {
    if fd == PYTHONLIB_FD {
        panic!("pread");
    }
    __real_pread(fd, buf, count, offset)
}
#[no_mangle]
pub unsafe extern fn __wrap_pread64(fd: c_int, buf: *mut c_void, count: size_t, offset: off_t) -> ssize_t {
    __wrap_pread(fd, buf, count, offset)
}
#[no_mangle]
pub unsafe extern fn __wrap_pwrite(fd: c_int, buf: *const c_void, count: size_t, offset: off_t) -> ssize_t {
    if fd == PYTHONLIB_FD {
        panic!("pwrite");
    }
    __real_pwrite(fd, buf, count, offset)
}
#[no_mangle]
pub unsafe extern fn __wrap_pwrite64(fd: c_int, buf: *const c_void, count: size_t, offset: off_t) -> ssize_t {
    __wrap_pwrite(fd, buf, count, offset)
}
#[no_mangle]
pub unsafe extern fn __wrap_lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    if fd == PYTHONLIB_FD {
        PYTHONLIB_OFF = match whence {
            libc::SEEK_SET => offset,
            libc::SEEK_CUR => PYTHONLIB_OFF as off_t + offset,
            libc::SEEK_END => PYTHONLIB.len() as off_t + offset, // offset is signed!
            w => offset + w as off_t,
        } as usize;
        return 0
    }
    __real_lseek(fd, offset, whence)
}
#[no_mangle]
pub unsafe extern fn __wrap_lseek64(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    __wrap_lseek(fd, offset, whence)
}
#[no_mangle]
pub unsafe extern fn __wrap_stat(path: *const c_char, buf: *mut libc::stat) -> c_int {
    if CStr::from_ptr(path).to_bytes() == PYTHONLIBTARGET.as_bytes() {
        *buf = get_pythonlib_stat_struct();
        return 0
    }
    __real_stat(path, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap_stat64(path: *const c_char, buf: *mut libc::stat) -> c_int {
    __wrap_stat(path, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap___xstat(ver: c_int, path: *const c_char, buf: *mut libc::stat) -> c_int {
    if CStr::from_ptr(path).to_bytes() == PYTHONLIBTARGET.as_bytes() {
        *buf = get_pythonlib_stat_struct();
        return 0
    }
    __real___xstat(ver, path, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap___xstat64(ver: c_int, path: *const c_char, buf: *mut libc::stat) -> c_int {
    __wrap___xstat(ver, path, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap_lstat(path: *const c_char, buf: *mut libc::stat) -> c_int {
    if CStr::from_ptr(path).to_bytes() == PYTHONLIBTARGET.as_bytes() {
        *buf = get_pythonlib_stat_struct();
        return 0
    }
    __real_lstat(path, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap_lstat64(path: *const c_char, buf: *mut libc::stat) -> c_int {
    __wrap_lstat(path, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap___lxstat(ver: c_int, path: *const c_char, buf: *mut libc::stat) -> c_int {
    if CStr::from_ptr(path).to_bytes() == PYTHONLIBTARGET.as_bytes() {
        *buf = get_pythonlib_stat_struct();
        return 0
    }
    __real___lxstat(ver, path, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap___lxstat64(ver: c_int, path: *const c_char, buf: *mut libc::stat) -> c_int {
    __wrap___lxstat(ver, path, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap_fstat(fd: c_int, buf: *mut libc::stat) -> c_int {
    if fd == PYTHONLIB_FD {
        *buf = get_pythonlib_stat_struct();
        return 0
    }
    __real_fstat(fd, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap_fstat64(fd: c_int, buf: *mut libc::stat) -> c_int {
    __wrap_fstat(fd, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap___fxstat(ver: c_int, fd: c_int, buf: *mut libc::stat) -> c_int {
    if fd == PYTHONLIB_FD {
        *buf = get_pythonlib_stat_struct();
        return 0
    }
    __real___fxstat(ver, fd, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap___fxstat64(ver: c_int, fd: c_int, buf: *mut libc::stat) -> c_int {
    __wrap___fxstat(ver, fd, buf)
}
