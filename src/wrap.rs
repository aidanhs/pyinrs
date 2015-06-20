#![allow(non_snake_case)]

use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex, MutexGuard};
use std::ptr;
use std::str;
use std::env;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use libc;
use libc::{c_void, c_int, c_long, c_char, size_t, ssize_t, off_t, fpos_t};

use super::{FILES, DIRS, WORKDIR};

static mut IS_INITIALISED: bool = false;

// Missing defines from libc crate
const AT_FDCWD: c_int = -100;
#[allow(dead_code)]
#[allow(non_camel_case_types)]
struct dirent {
    d_ino: libc::ino_t,
    d_off: off_t,
    d_reclen: libc::c_ushort,
    d_type: libc::c_uchar,
    d_name: [c_char; 256],
}

pub fn prep() {
    // Make sure FS is initialised
    assert!(!FS().exists("non_existent_file"));
    unsafe { IS_INITIALISED = true };
}

pub fn atexit() {}

// memfd_create in kernel 3.17

struct FileState {
    cwd: Option<&'static str>,
    // path, offset
    fds: Vec<(&'static str, usize)>,
    // fd, eof
    fps: Vec<(c_int, bool)>,
    base_fp: usize,
    base_fd: usize,
    // for storing memory for dirent calls
    // next_offset, cur_dirent
    dirents: Vec<(usize, dirent)>,
    // for stat calls
    inodes: HashMap<&'static str, libc::ino_t>,
    base_inode: usize,
}

impl FileState {
    fn exists(&self, fpath: &str) -> bool {
        let workdir = unsafe { WORKDIR };
        let mut path = self.actual_cwd_path();
        path.push(Path::new(fpath));
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

    fn get_fp_data(&self, fp: *mut libc::FILE) -> (&'static [u8], c_int, usize, bool) {
        let fp = fp as usize;
        let (fd, eof) = self.fps[fp - self.base_fp];
        let (data, offset) = self.get_fd_data(fd);
        (data, fd, offset, eof)
    }
    fn set_fp_offset(&mut self, fp: *mut libc::FILE, offset: usize) {
        let fp = fp as usize;
        let (fd, _) = self.fps[fp - self.base_fp];
        self.set_fd_offset(fd, offset)
    }
    fn set_fp_eof(&mut self, fp: *mut libc::FILE, eof: bool) {
        let fp = fp as usize;
        let (fd, _) = self.fps[fp - self.base_fp];
        self.fps[fp - self.base_fp] = (fd, eof)
    }

    fn get_fd_path(&self, fd: c_int) -> &'static str {
        let fd = fd as usize;
        let (path, _) = self.fds[fd - self.base_fd];
        path
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
        let fd = self.open_as_fd(fpath);
        self.fps.push((fd, false));
        (self.base_fp + self.fps.len() - 1) as *mut libc::FILE
    }
    fn open_as_fd(&mut self, fpath: &str) -> c_int {
        let relpath = self.to_relpath(fpath);
        let path = match FILES.get_key(relpath) {
            Some(fpath) => fpath,
            None => DIRS.get_key(relpath).unwrap(),
        };
        self.fds.push((path, 0));
        (self.base_fd + self.fds.len() - 1) as c_int
    }

    fn get_inode(&mut self, fpath: &'static str) -> libc::ino_t {
        match self.inodes.get(fpath) {
            Some(&inode) => inode,
            None => {
                let inode = (self.inodes.len() + self.base_inode) as libc::ino_t;
                self.inodes.insert(fpath, inode);
                inode
            },
        }
    }

    fn stat_fd(&mut self, fd: c_int) -> libc::stat {
        let workdir = unsafe { WORKDIR };
        let (path, _) = self.fds[fd as usize - self.base_fd];
        self.stat(&format!("{}/{}", workdir, path))
    }
    // TODO: this method does lookups multiple times
    fn stat(&mut self, fpath: &str) -> libc::stat {
        let fpath = self.to_relpath(fpath);
        let (isfile, isdir, fpath) = if FILES.contains_key(fpath) {
            (true, false, FILES.get_key(fpath).unwrap())
        } else if DIRS.contains(fpath) {
            (false, true, DIRS.get_key(fpath).unwrap())
        } else {
            unreachable!()
        };
        let mut stat = libc::stat {
            st_dev: 100000, // arbitrary
            st_ino: self.get_inode(fpath),
            st_mode: 0o100555, // normal file, read only
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
            stat.st_mode = 0o040555; // directory, r+x
            stat.st_nlink = 100;
            stat.st_size = 1024;
        } else {
            unreachable!();
        }
        stat
    }

    fn seek_dirent(&mut self, dirp: *mut libc::DIR, loc: libc::c_long) {
        let diro = dirp as usize - self.base_fp;
        self.dirents[diro].0 = loc as usize
    }
    fn next_dirent(&mut self, dirp: *mut libc::DIR) -> *mut dirent {
        let diro = dirp as usize - self.base_fp;
        while self.dirents.len() <= diro {
            self.dirents.push((0, dirent {
                d_ino: 0, d_off: 0, d_reclen: 0,
                d_type: 0 as libc::c_uchar, d_name: [0; 256],
            }));
        }

        let (dirent_off, _) = self.dirents[diro];
        let (fd, _) = self.fps[diro];
        let dpath_str = self.get_fd_path(fd);
        let dpath = Path::new(dpath_str);
        let mut dirent_count = 0;
        for &subpath_str in DIRS.iter().chain(FILES.keys()) {
            let subpath = Path::new(subpath_str);
            match subpath.parent() {
                Some(parentpath) => if parentpath != dpath { continue },
                None => continue,
            }
            if dirent_off != dirent_count {
                dirent_count += 1;
                continue
            }
            let fpath_str: &str = subpath.file_name().unwrap().to_str().unwrap();
            let fpath_len = fpath_str.len();
            assert!(fpath_len <= 256);
            let mut de = dirent {
                d_ino: self.get_inode(fpath_str),
                d_off: dirent_off as c_long,
                d_reclen: fpath_len as libc::c_ushort,
                d_type: 0 as libc::c_uchar,
                d_name: [0; 256],
            };
            let cbytes = CString::new(fpath_str).unwrap().as_ptr();
            unsafe { ptr::copy(cbytes, de.d_name.as_mut_ptr(), fpath_len) };
            self.dirents[diro] = (dirent_off + 1, de);
            return (&mut self.dirents[diro].1) as *mut dirent
        }
        ptr::null_mut()
    }

    fn set_cwd(&mut self, dir: &str) {
        self.cwd = Some(self.to_relpath(dir))
    }
    fn unset_cwd(&mut self) {
        self.cwd = None
    }
    fn has_cwd(&self) -> bool {
        self.cwd.is_some()
    }
    fn get_cwd(&self) -> Option<&'static str> {
        self.cwd
    }
    fn actual_cwd_path(&self) -> PathBuf {
        match self.cwd {
            Some(pathstr) => PathBuf::from(pathstr),
            None => {
                // cannot use env::current_dir().unwrap() because deadlock
                let cwd_ptr = unsafe { __real_getcwd(ptr::null_mut(), 0) };
                let cwd_cstr = unsafe { CStr::from_ptr(cwd_ptr) };
                let pb = PathBuf::from(cwd_cstr.to_str().unwrap());
                unsafe { libc::free(cwd_ptr as *mut c_void) };
                pb
            }
        }
    }

    fn to_relpath(&self, fpath: &str) -> &'static str {
        let workdir = unsafe { WORKDIR };
        let mut path = self.actual_cwd_path();
        path.push(Path::new(fpath));
        let relpath = path.relative_from(workdir).unwrap().to_str().unwrap();
        let srelpath = FILES.get_key(relpath);
        if srelpath.is_some() {
            srelpath.unwrap()
        } else {
            DIRS.get_key(relpath).unwrap()
        }
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
        cwd: None,
        fds: vec![],
        fps: vec![],
        base_fp: 1,
        base_fd: 100000,
        dirents: vec![],
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
    fn __real_fgets(s: *mut c_char, size: c_int, stream: *mut libc::FILE) -> *mut libc::c_char;
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
    fn __real_flockfile(stream: *mut libc::FILE);
    fn __real_ftrylockfile(stream: *mut libc::FILE) -> c_int;
    fn __real_funlockfile(stream: *mut libc::FILE);

    fn __real_opendir(name: *const c_char) -> *mut libc::DIR;
    fn __real_fdopendir(fd: c_int) -> *mut libc::DIR;
    fn __real_closedir(dirp: *mut libc::DIR) -> c_int;
    fn __real_readdir(dirp: *mut libc::DIR) -> *mut libc::dirent_t;
    fn __real_readdir64(dirp: *mut libc::DIR) -> *mut libc::dirent_t;
    fn __real_readdir_r(dirp: *mut libc::DIR, entry: *mut libc::DIR, result: *mut *mut libc::DIR) -> c_int;
    fn __real_readdir_r64(dirp: *mut libc::DIR, entry: *mut libc::DIR, result: *mut *mut libc::DIR) -> c_int;
    fn __real_rewinddir(dirp: *mut libc::DIR);
    fn __real_seekdir(dirp: *mut libc::DIR, loc: c_long);
    fn __real_telldir(dirp: *mut libc::DIR) -> c_long;

    fn __real_dup(oldfd: c_int) -> c_int;
    fn __real_dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn __real_dup3(oldfd: c_int, newfd: c_int, flags: c_int) -> c_int;
    fn __real_read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    fn __real_open(pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int;
    fn __real_open64(pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int;
    fn __real_openat(dirfd: c_int, pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int;
    fn __real_openat64(dirfd: c_int, pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int;
    fn __real_creat(pathname: *const c_char, mode: libc::mode_t) -> c_int;
    fn __real_creat64(pathname: *const c_char, mode: libc::mode_t) -> c_int;
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
    fn __real_access(pathname: *const c_char, mode: c_int) -> c_int;

    fn __real_chdir(path: *const c_char) -> c_int;
    fn __real_fchdir(fd: c_int) -> c_int;
    fn __real_getcwd(buf: *mut c_char, size: size_t) -> *mut c_char;
    fn __real_getwd(buf: *mut c_char) -> *mut c_char;
    fn __real_get_current_dir_name() -> *mut c_char;
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
        let (data, _, offset, _) = FS().get_fp_data(stream);
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
pub unsafe extern fn __wrap_fgets(s: *mut c_char, size: c_int, stream: *mut libc::FILE) -> *mut libc::c_char {
    if INIT() && FS().is_fp(stream) {
        let size = size as usize;
        let (data, _, offset, _) = FS().get_fp_data(stream);
        let size = if size + offset > data.len() { data.len() - offset } else { size };
        let slice = &data[offset..offset+size];
        let numtaken = match slice.position_elem(&('\n' as u8)) {
            Some(numtaken) => numtaken + 1,
            None => size,
        };
        ptr::copy(data[offset..].as_ptr() as *mut u8, s as *mut u8, numtaken);
        let nul: &[u8] = &[0];
        ptr::copy(nul.as_ptr(), (s as usize + numtaken) as *mut u8, 1);
        let newoffset = offset + numtaken;
        if newoffset == data.len() {
            FS().set_fp_eof(stream, true)
        }
        FS().set_fp_offset(stream, newoffset);
        return if numtaken == 0 { ptr::null_mut() } else { s }
    }
    __real_fgets(s, size, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_getc(stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        let (data, _, offset, _) = FS().get_fp_data(stream);
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
        let (data, _, offset, _) = FS().get_fp_data(stream);
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
        let (data, _, offset, _) = FS().get_fp_data(stream);
        assert!(data[offset - 1] == c as u8, "cannot unget mismatched char");
        FS().set_fp_offset(stream, offset - 1);
        return c
    }
    __real_ungetc(c, stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_fseek(stream: *mut libc::FILE, offset: c_long, whence: c_int) -> c_int {
    if INIT() && FS().is_fp(stream) {
        let (data, _, cur_offset, _) = FS().get_fp_data(stream);
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
        let (_, _, offset, _) = FS().get_fp_data(stream);
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
        let (_, _, _, eof) = FS().get_fp_data(stream);
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
        let (_, fd, _, _) = FS().get_fp_data(stream);
        return fd
    }
    __real_fileno(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_flockfile(stream: *mut libc::FILE) {
    if INIT() && FS().is_fp(stream) {
        panic!("flockfile");
    }
    __real_flockfile(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_ftrylockfile(stream: *mut libc::FILE) -> c_int {
    if INIT() && FS().is_fp(stream) {
        panic!("ftrylockfile");
    }
    __real_ftrylockfile(stream)
}
#[no_mangle]
pub unsafe extern fn __wrap_funlockfile(stream: *mut libc::FILE) {
    if INIT() && FS().is_fp(stream) {
        panic!("funlockfile");
    }
    __real_funlockfile(stream)
}



#[no_mangle]
pub unsafe extern fn __wrap_opendir(name: *const c_char) -> *mut libc::DIR {
    let str_path = str::from_utf8(CStr::from_ptr(name).to_bytes()).unwrap();
    let path = Path::new(str_path);
    let cur_path = env::current_dir().unwrap();
    let abs_path = cur_path.join(path);
    let abs_path_str = abs_path.to_str().unwrap();
    if INIT() && FS().exists(abs_path_str) {
        let ret = FS().open_as_fp(abs_path_str) as *mut libc::DIR;
        return ret
    }
    __real_opendir(name)
}
#[no_mangle]
pub unsafe extern fn __wrap_fdopendir(fd: c_int) -> *mut libc::DIR {
    if INIT() && FS().is_fd(fd) {
        panic!("fdopendir");
    }
    __real_fdopendir(fd)
}
#[no_mangle]
pub unsafe extern fn __wrap_closedir(dirp: *mut libc::DIR) -> c_int {
    let fp = dirp as *mut libc::FILE;
    if INIT() && FS().is_fp(fp) {
        __wrap_seekdir(dirp, 0);
        return 0
    }
    __real_closedir(dirp)
}
#[no_mangle]
pub unsafe extern fn __wrap_readdir(dirp: *mut libc::DIR) -> *mut libc::dirent_t {
    let fp = dirp as *mut libc::FILE;
    if INIT() && FS().is_fp(fp) {
        return FS().next_dirent(dirp) as *mut libc::dirent_t
    }
    __real_readdir(dirp)
}
#[no_mangle]
pub unsafe extern fn __wrap_readdir64(dirp: *mut libc::DIR) -> *mut libc::dirent_t {
    __wrap_readdir(dirp)
}
#[no_mangle]
pub unsafe extern fn __wrap_readdir_r(dirp: *mut libc::DIR, entry: *mut libc::DIR, result: *mut *mut libc::DIR) -> c_int {
    let fp = dirp as *mut libc::FILE;
    if INIT() && FS().is_fp(fp) {
        panic!("readdir_r");
    }
    __real_readdir_r(dirp, entry, result)
}
#[no_mangle]
pub unsafe extern fn __wrap_readdir_r64(dirp: *mut libc::DIR, entry: *mut libc::DIR, result: *mut *mut libc::DIR) -> c_int {
    __wrap_readdir_r(dirp, entry, result)
}
#[no_mangle]
pub unsafe extern fn __wrap_rewinddir(dirp: *mut libc::DIR) {
    let fp = dirp as *mut libc::FILE;
    if INIT() && FS().is_fp(fp) {
        panic!("rewinddir");
    }
    __real_rewinddir(dirp)
}
#[no_mangle]
pub unsafe extern fn __wrap_seekdir(dirp: *mut libc::DIR, loc: c_long) {
    let fp = dirp as *mut libc::FILE;
    if INIT() && FS().is_fp(fp) {
        FS().seek_dirent(dirp, loc);
        return
    }
    __real_seekdir(dirp, loc)
}
#[no_mangle]
pub unsafe extern fn __wrap_telldir(dirp: *mut libc::DIR) -> c_long {
    let fp = dirp as *mut libc::FILE;
    if INIT() && FS().is_fp(fp) {
        panic!("telldir");
    }
    __real_telldir(dirp)
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
    let path = Path::new(str_path);
    let cur_path = env::current_dir().unwrap();
    let abs_path = cur_path.join(path);
    let abs_path_str = abs_path.to_str().unwrap();
    if INIT() && FS().exists(abs_path_str) {
        let allowed = 0 as c_int;
        if flags | allowed != allowed {
            panic!("open: invalid flag {}", flags);
        }
        if mode as c_int != libc::O_RDONLY {
            panic!("open: invalid mode {}", mode);
        }
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
pub unsafe extern fn __wrap_openat(dirfd: c_int, pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    let str_path = str::from_utf8(CStr::from_ptr(pathname).to_bytes()).unwrap();
    let path = Path::new(str_path);
    let mut abs_path = PathBuf::new();
    let isvirt = if !INIT() {
        false
    } else if path.is_absolute() {
        abs_path.push(path);
        FS().exists(str_path)
    } else if dirfd != AT_FDCWD && !FS().is_fd(dirfd) {
        false
    } else {
        let prefixpath = match dirfd {
            AT_FDCWD => FS().actual_cwd_path(),
            dfd => PathBuf::from(FS().get_fd_path(dfd)),
        };
        let path = prefixpath.join(path);
        abs_path.push(&path);
        FS().exists(path.to_str().unwrap())
    };
    if isvirt {
        let abs_str_path = abs_path.to_str().unwrap();
        return __wrap_open(CString::new(abs_str_path).unwrap().as_ptr(), flags, mode)
    }
    __real_openat(dirfd, pathname, flags, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_openat64(dirfd: c_int, pathname: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    __wrap_openat(dirfd, pathname, flags, mode)
}
#[no_mangle]
pub unsafe extern fn __wrap_creat(pathname: *const c_char, mode: libc::mode_t) -> c_int {
    let str_path = str::from_utf8(CStr::from_ptr(pathname).to_bytes()).unwrap();
    let path = Path::new(str_path);
    if path.starts_with(Path::new(WORKDIR)) {
        panic!("creat");
    }
    __real_creat(pathname, mode)
}
pub unsafe extern fn __wrap_creat64(pathname: *const c_char, mode: libc::mode_t) -> c_int {
    __wrap_creat(pathname, mode)
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
#[no_mangle]
pub unsafe extern fn __wrap_access(pathname: *const c_char, mode: c_int) -> c_int {
    let str_path = str::from_utf8(CStr::from_ptr(pathname).to_bytes()).unwrap();
    if INIT() && FS().exists(str_path) {
        if mode == libc::F_OK {
            return 0
        }
        if mode | libc::R_OK | libc::X_OK == libc::R_OK | libc::X_OK {
            return 0
        }
        // deliberate fall-through to access denied
    }
    __real_access(pathname, mode)
}

#[no_mangle]
pub unsafe extern fn __wrap_chdir(path: *const c_char) -> c_int {
    let str_path = str::from_utf8(CStr::from_ptr(path).to_bytes()).unwrap();
    if INIT() && FS().exists(str_path) {
        FS().set_cwd(str_path);
        return 0
    }
    if INIT() { FS().unset_cwd(); }
    __real_chdir(path)
}
#[no_mangle]
pub unsafe extern fn __wrap_fchdir(fd: c_int) -> c_int {
    if INIT() && FS().is_fd(fd) {
        let str_path = FS().get_fd_path(fd);
        FS().set_cwd(str_path);
        return 0
    }
    if INIT() { FS().unset_cwd(); }
    __real_fchdir(fd)
}
#[no_mangle]
pub unsafe extern fn __wrap_getcwd(buf: *mut c_char, size: size_t) -> *mut c_char {
    if INIT() && FS().has_cwd() {
        if buf == ptr::null_mut() {
            panic!("getcwd: null");
        }
        let size = size as usize;
        let reldir_str = FS().get_cwd().unwrap();
        let mut dir = PathBuf::from(WORKDIR);
        if reldir_str != "" {
            dir.push(reldir_str);
        }
        let dir_str = dir.to_str().unwrap();
        if dir_str.len() > size {
            panic!("getcwd: too big")
        }
        ptr::copy_nonoverlapping(dir_str.as_ptr(), buf as *mut libc::c_uchar, size);
        return buf
    }
    __real_getcwd(buf, size)
}
#[no_mangle]
pub unsafe extern fn __wrap_getwd(buf: *mut c_char) -> *mut c_char {
    if INIT() && FS().has_cwd() {
        panic!("getcwd")
    }
    __real_getwd(buf)
}
#[no_mangle]
pub unsafe extern fn __wrap_get_current_dir_name() -> *mut c_char {
    if INIT() && FS().has_cwd() {
        panic!("get_current_dir_name")
    }
    __real_get_current_dir_name()
}
