#![allow(non_snake_case)]

use std::ffi::CStr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::ptr;
use std::str;
use std::path::Path;
use std::collections::HashMap;
use libc;
use libc::{c_void, c_int, c_long, c_char, size_t, ssize_t, off_t, fpos_t};

use super::{FILES, DIRS, WORKDIR};

static mut IS_INITIALISED: bool = false;

pub fn prep() {
    // Make sure FS is initialised
    assert!(!FS().exists("non_existent_file"));
    unsafe { IS_INITIALISED = true };
}

pub fn atexit() {}

// memfd_create in kernel 3.17

struct FileState {
    // data, offset, (eof)
    fds: Vec<(&'static str, usize)>,
    fps: Vec<(&'static str, usize, bool)>,
    base_fp: usize,
    base_fd: usize,
    // for stat calls
    inodes: HashMap<&'static str, libc::ino_t>,
    base_inode: usize,
}

impl FileState {
    fn exists(&self, fpath: &str) -> bool {
        let workdir = unsafe { WORKDIR };
        let path = Path::new(fpath);
        let relresult = path.relative_from(workdir);
        // Not in workdir?
        if relresult.is_none() {
            return false
        }
        // Filename?
        let relpath_str = relresult.unwrap().to_str().unwrap();
        if FILES.get_key(relpath_str).is_some() {
            true
        } else if DIRS.contains(relpath_str) {
            true
        } else {
            false
        }
    }
    fn is_fd(&self, fd: c_int) -> bool {
        let fd = fd as usize;
        fd >= self.base_fd && fd < self.base_fd + self.fds.len()
    }
    fn is_fp(&self, fp: *mut libc::FILE) -> bool {
        let fp = fp as usize;
        fp >= self.base_fp && fp < self.base_fp + self.fps.len()
    }

    fn get_fp_data(&self, fp: *mut libc::FILE) -> (&'static [u8], usize, bool) {
        let fp = fp as usize;
        let (path, offset, eof) = self.fps[fp - self.base_fp];
        (FILES.get(path).unwrap(), offset, eof)
    }
    fn set_fp_offset(&mut self, fp: *mut libc::FILE, offset: usize) {
        let fp = fp as usize;
        let (path, _, eof) = self.fps[fp - self.base_fp];
        self.fps[fp - self.base_fp] = (path, offset, eof)
    }
    fn set_fp_eof(&mut self, fp: *mut libc::FILE, eof: bool) {
        let fp = fp as usize;
        let (path, offset, _) = self.fps[fp - self.base_fp];
        self.fps[fp - self.base_fp] = (path, offset, eof)
    }

    fn get_fd_data(&self, fd: c_int) -> (&'static [u8], usize) {
        let fd = fd as usize;
        let (path, offset) = self.fds[fd - self.base_fd];
        (FILES.get(path).unwrap(), offset)
    }
    fn set_fd_offset(&mut self, fd: c_int, offset: usize) {
        let fd = fd as usize;
        let (path, _) = self.fds[fd - self.base_fd];
        self.fds[fd - self.base_fd] = (path, offset)
    }

    fn open_as_fp(&mut self, fpath: &str) -> *mut libc::FILE {
        self.fps.push((FILES.get_key(to_relpath(fpath)).unwrap(), 0, false));
        (self.base_fp + self.fps.len() - 1) as *mut libc::FILE
    }
    fn open_as_fd(&mut self, fpath: &str) -> c_int {
        self.fds.push((FILES.get_key(to_relpath(fpath)).unwrap(), 0));
        (self.base_fd + self.fds.len() - 1) as c_int
    }

    fn stat_fd(&mut self, fd: c_int) -> libc::stat {
        let (path, _) = self.fds[fd as usize - self.base_fd];
        self.stat(path)
    }
    // TODO: this method does lookups multiple times
    fn stat(&mut self, fpath: &str) -> libc::stat {
        let fpath = to_relpath(fpath);
        let (isfile, isdir, fpath) = if FILES.contains_key(fpath) {
            (true, false, FILES.get_key(fpath).unwrap())
        } else if DIRS.contains(fpath) {
            (false, true, DIRS.get_key(fpath).unwrap())
        } else {
            unreachable!()
        };
        let inode: libc::ino_t = match self.inodes.get(fpath) {
            Some(&inode) => inode,
            None => {
                let inode = (self.inodes.len() + self.base_inode) as libc::ino_t;
                self.inodes.insert(fpath, inode);
                inode
            },
        };
        let mut stat = libc::stat {
            st_dev: 100000, // arbitrary
            st_ino: inode,
            st_mode: 0o100444, // normal file, read only
            st_nlink: 1,
            st_uid: 1,
            st_gid: 1,
            __pad0: 0, // ???
            st_rdev: 0, // arbitrary
            st_size: 0,
            st_blksize: 4096,
            st_blocks: 0,
            st_atime: 0,
            st_atime_nsec: 0,
            st_mtime: 0,
            st_mtime_nsec: 0,
            st_ctime: 0,
            st_ctime_nsec: 0,
            __unused: [0, 0, 0],
        };
        if isfile {
            let data = FILES.get(fpath).unwrap();
            stat.st_size = data.len() as ssize_t;
            stat.st_blocks = ((data.len() + 1024) / 512) as ssize_t;
        } else if isdir {
            stat.st_mode = 0o040444;
            stat.st_nlink = 100;
            stat.st_size = 1024;
        } else {
            unreachable!();
        }
        stat
    }
}

fn to_relpath(fpath: &str) -> &'static str {
    let workdir = unsafe { WORKDIR };
    let relpath = Path::new(fpath).relative_from(workdir).unwrap().to_str().unwrap();
    let srelpath = FILES.get_key(relpath);
    if srelpath.is_some() {
        srelpath.unwrap()
    } else {
        DIRS.get_key(relpath).unwrap()
    }
}

unsafe fn read_into_ptr(src: &[u8], target_ptr: *mut c_void, offset: usize, count: usize) -> usize {
    let src_ptr = (src.as_ptr() as usize + offset) as *const u8;
    let target_ptr = target_ptr as *mut u8;

    let lenleft = src.len() - offset;
    let num = if count < lenleft { count } else { lenleft };
    ptr::copy_nonoverlapping(src_ptr, target_ptr, num);
    return num
}

lazy_static!{
    static ref FILE_STATE: Arc<Mutex<FileState>> = Arc::new(Mutex::new(FileState {
        fds: vec![],
        fps: vec![],
        base_fp: 1,
        base_fd: 100000,
        inodes: HashMap::new(),
        base_inode: 0,
    }));
}

fn FS<'a>() -> MutexGuard<'a, FileState> {
    FILE_STATE.lock().unwrap()
}

fn INIT() -> bool {
    unsafe { IS_INITIALISED }
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
    if INIT() && FS().is_fp(fp) {
        return 0
    }
    __real_fclose(fp)
}
#[no_mangle]
pub unsafe extern fn __wrap_fopen(path: *const c_char, mode: *const c_char) -> *mut libc::FILE {
    let str_path = str::from_utf8(CStr::from_ptr(path).to_bytes()).unwrap();
    if INIT() && FS().exists(str_path) {
        return FS().open_as_fp(str_path)
    }
    __real_fopen(path, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_fopen64(path: *const c_char, mode: *const c_char) -> *mut libc::FILE {
    __wrap_fopen(path, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_fdopen(fd: c_int, mode: *const c_char) -> *mut libc::FILE {
    if INIT() && FS().is_fd(fd) {
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
    let str_path = str::from_utf8(CStr::from_ptr(path).to_bytes()).unwrap();
    if INIT() && (FS().exists(str_path) || FS().is_fp(stream)) {
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
    if INIT() && FS().is_fp(stream) {
        // TODO: implement better
        let (data, offset, _) = FS().get_fp_data(stream);
        let lenleft = data.len() - offset;
        let wanted = (size * nmemb) as usize;
        let count = if lenleft > wanted { wanted } else { lenleft - (lenleft % size as usize) };
        assert!(read_into_ptr(data, ptr, offset, count) == count);
        FS().set_fp_offset(stream, offset + count);
        return count as size_t / size;
    }
    __real_fread(ptr, size, nmemb, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fread64(ptr: *mut c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t {
    __wrap_fread(ptr, size, nmemb, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fwrite(ptr: *const c_void, size: size_t, nmemb: size_t, stream: *mut libc::FILE) -> size_t {
    if INIT() && FS().is_fp(stream) {
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
    if INIT() && FS().is_fp(stream) {
        panic!("fgetc")
    }
    __real_fgetc(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fgets(s: *mut c_char, size: c_int, stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        panic!("fgets")
    }
    __real_fgets(s, size, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_getc(stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        let (data, offset, _) = FS().get_fp_data(stream);
        if data.len() == offset {
            return libc::EOF
        }
        let chr = data[offset];
        FS().set_fp_offset(stream, offset + 1);
        return chr as c_int
    }
    __real_getc(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap__IO_getc(stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        let (data, offset, _) = FS().get_fp_data(stream);
        if data.len() == offset {
            return libc::EOF
        }
        let chr = data[offset];
        FS().set_fp_offset(stream, offset + 1);
        return chr as c_int
    }
    __real__IO_getc(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ungetc(c: c_int, stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        panic!("ungetc")
    }
    __real_ungetc(c, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fseek(stream: *mut libc::FILE, offset: c_long, whence: c_int) -> c_int {
    if INIT() && FS().is_fp(stream) {
        let (data, cur_offset, _) = FS().get_fp_data(stream);
        let seek_offset = match whence {
            libc::SEEK_SET => offset,
            libc::SEEK_CUR => cur_offset as off_t + offset,
            libc::SEEK_END => data.len() as off_t + offset, // offset is signed!
            w => offset + w as off_t,
        } as usize;
        FS().set_fp_offset(stream, seek_offset);
        FS().set_fp_eof(stream, false);
        return 0
    }
    __real_fseek(stream, offset, whence)
}
#[no_mangle]
pub unsafe extern fn __wrap_fseek64(stream: *mut libc::FILE, offset: c_long, whence: c_int) -> c_int {
    __wrap_fseek(stream, offset, whence)
}
#[no_mangle]
pub unsafe extern fn __wrap_fseeko(stream: *mut libc::FILE, offset: off_t, whence: c_int) -> c_int {
    if INIT() && FS().is_fp(stream) {
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
    if INIT() && FS().is_fp(stream) {
        let (_, offset, _) = FS().get_fp_data(stream);
        return offset as c_long
    }
    __real_ftell(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ftell64(stream: *mut libc::FILE) -> c_long {
    __wrap_ftell(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ftello(stream: *mut libc::FILE) -> off_t {
    if INIT() && FS().is_fp(stream) {
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
    if INIT() && FS().is_fp(stream) {
        panic!("rewind")
    }
    __real_rewind(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fgetpos(stream: *mut libc::FILE, pos: *mut fpos_t) -> c_int {
    if INIT() && FS().is_fp(stream) {
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
    if INIT() && FS().is_fp(stream) {
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
    if INIT() && FS().is_fp(stream) {
        FS().set_fp_eof(stream, false);
        return
    }
    __real_clearerr(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_feof(stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        let (_, _, eof) = FS().get_fp_data(stream);
        return if eof { 1 } else { 0 }
    }
    __real_feof(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ferror(stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        return 0;
    }
    __real_ferror(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fileno(stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        panic!("fileno")
    }
    __real_fileno(stream)
}



#[no_mangle]
pub unsafe extern fn __wrap_dup(oldfd: c_int) -> c_int {
    if INIT() && FS().is_fd(oldfd) { panic!("dup") }
    __real_dup(oldfd)
}
#[no_mangle]
pub unsafe extern fn __wrap_dup2(oldfd: c_int, newfd: c_int) -> c_int {
    if INIT() && FS().is_fd(oldfd) || FS().is_fd(newfd) { panic!("dup2") }
    __real_dup2(oldfd, newfd)
}
#[no_mangle]
pub unsafe extern fn __wrap_dup3(oldfd: c_int, newfd: c_int, flags: c_int) -> c_int {
    if INIT() && FS().is_fd(oldfd) || FS().is_fd(newfd) { panic!("dup3") }
    __real_dup3(oldfd, newfd, flags)
}
#[no_mangle]
pub unsafe extern fn __wrap_open(pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    let str_path = str::from_utf8(CStr::from_ptr(pathname).to_bytes()).unwrap();
    if INIT() && FS().exists(str_path) {
        // This violates the spec by not returning the lowest numbered
        // unused fd
        return FS().open_as_fd(str_path)
    }
    __real_open(pathname, flags, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_open64(pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    __wrap_open(pathname, flags, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t {
    if INIT() && FS().is_fd(fd) {
        let (data, offset) = FS().get_fd_data(fd);
        let actual_count = read_into_ptr(data, buf, offset, count as usize);
        FS().set_fd_offset(fd, offset + actual_count);
        return actual_count as ssize_t
    }
    __real_read(fd, buf, count)
}
#[no_mangle]
pub unsafe extern fn __wrap_pread(fd: c_int, buf: *mut c_void, count: size_t, offset: off_t) -> ssize_t {
    if INIT() && FS().is_fd(fd) {
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
    if INIT() && FS().is_fd(fd) {
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
    if INIT() && FS().is_fd(fd) {
        let (data, cur_offset) = FS().get_fd_data(fd);
        let seek_offset = match whence {
            libc::SEEK_SET => offset,
            libc::SEEK_CUR => cur_offset as off_t + offset,
            libc::SEEK_END => data.len() as off_t + offset, // offset is signed!
            w => offset + w as off_t,
        } as usize;
        FS().set_fd_offset(fd, seek_offset);
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
    let str_path = str::from_utf8(CStr::from_ptr(path).to_bytes()).unwrap();
    if INIT() && FS().exists(str_path) {
        *buf = FS().stat(str_path);
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
    let str_path = str::from_utf8(CStr::from_ptr(path).to_bytes()).unwrap();
    if INIT() && FS().exists(str_path) {
        *buf = FS().stat(str_path);
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
    let str_path = str::from_utf8(CStr::from_ptr(path).to_bytes()).unwrap();
    if INIT() && FS().exists(str_path) {
        *buf = FS().stat(str_path);
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
    let str_path = str::from_utf8(CStr::from_ptr(path).to_bytes()).unwrap();
    if INIT() && FS().exists(str_path) {
        *buf = FS().stat(str_path);
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
    if INIT() && FS().is_fd(fd) {
        *buf = FS().stat_fd(fd);
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
    if INIT() && FS().is_fd(fd) {
        *buf = FS().stat_fd(fd);
        return 0
    }
    __real___fxstat(ver, fd, buf)
}
#[no_mangle]
pub unsafe extern fn __wrap___fxstat64(ver: c_int, fd: c_int, buf: *mut libc::stat) -> c_int {
    __wrap___fxstat(ver, fd, buf)
}
