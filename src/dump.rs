use std::fs;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

use super::{FILES, WORKDIR};

pub fn prep() {
    let workdir = unsafe { WORKDIR };
    if !Path::new(workdir).is_dir() {
        for (relpath, data) in FILES.entries() {
            let path = Path::new(workdir).join(relpath);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut f = File::create(path).unwrap();
            f.write_all(data).unwrap();
        }
    }
}

pub fn atexit() {
    let workdir = unsafe { WORKDIR };
    fs::remove_dir_all(workdir).unwrap();
}
