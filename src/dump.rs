use std::fs;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

use super::{PYTHONLIB, PYTHONLIBTARGET, FILES, FILESTARGET};

pub fn prep() {
    let path = Path::new(PYTHONLIBTARGET);
    let mut f = File::create(path).unwrap();
    f.write_all(PYTHONLIB).unwrap();

    if !Path::new(FILESTARGET).is_dir() {
        for (relpath, data) in FILES.entries() {
            let path = Path::new(FILESTARGET).join(relpath);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut f = File::create(path).unwrap();
            f.write_all(data).unwrap();
        }
    }
}

pub fn atexit() {
    fs::remove_file(PYTHONLIBTARGET).unwrap();
    fs::remove_dir_all(FILESTARGET).unwrap();
}
