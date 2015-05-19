use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

use super::{PYTHONLIB, PYTHONLIBTARGET};

pub fn prep() {
    let path = Path::new(PYTHONLIBTARGET);
    let mut f = File::create(path).unwrap();
    f.write_all(PYTHONLIB).unwrap();
}
