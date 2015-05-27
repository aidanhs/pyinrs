#![feature(path_ext)]
#![feature(path_relative_from)]

extern crate phf_codegen;
extern crate glob;
extern crate uuid;

use std::fs;
use std::io::prelude::*;
use std::path::Path;
use std::collections::HashSet;

fn main() {
    println!("cargo:rustc-link-search=native={}", "cpython/Modules/zlib");

    let paths: Vec<String> = glob::glob("include/**/*").unwrap()
        .map(|e| e.unwrap())
        .filter(|p| p.is_file())
        .map(|p| String::from(p.relative_from("include").unwrap().to_str().unwrap()))
        .collect();

    // TODO: make files and dirs an enum in the same map?
    let mut file = fs::File::create("include.files").unwrap();
    let mut filebuilder = phf_codegen::Map::new();
    let mut dirset = HashSet::new();

    for path in &paths {
        write!(&mut file, "#[allow(non_upper_case_globals)]\n").unwrap();
        let varname = format!("FILE_{}", uuid::Uuid::new_v4().to_simple_string());
        let incstr = format!("include_bytes!(\"../include/{}\")", path);
        write!(&mut file, "const {}: &'static [u8] = {};\n", varname, incstr).unwrap();
        filebuilder.entry(&**path, &varname);

        let mut parent = Path::new(path).parent().unwrap();
        while parent.to_str().unwrap() != "" {
            dirset.insert(parent.to_str().unwrap());
            parent = parent.parent().unwrap();
        }
    }

    let mut dirbuilder = phf_codegen::Set::new();
    dirbuilder.entry("");
    for dir in dirset.iter() {
        dirbuilder.entry(dir);
    }

    write!(&mut file, "static FILES: phf::Map<&'static str, &'static [u8]> = ").unwrap();
    filebuilder.build(&mut file).unwrap();
    write!(&mut file, ";\n").unwrap();

    write!(&mut file, "static DIRS: phf::Set<&'static str> = ").unwrap();
    dirbuilder.build(&mut file).unwrap();
    write!(&mut file, ";\n").unwrap();
}
