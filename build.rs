#![feature(path_ext)]
#![feature(path_relative_from)]

extern crate phf_codegen;
extern crate glob;
extern crate uuid;

use std::fs;
use std::io::prelude::*;

fn main() {
    println!("cargo:rustc-link-search=native={}", "cpython/Modules/zlib");

    let paths: Vec<String> = glob::glob("include/**/*").unwrap()
        .map(|e| e.unwrap())
        .filter(|p| p.is_file())
        .map(|p| String::from(p.relative_from("include").unwrap().to_str().unwrap()))
        .collect();

    let mut file = fs::File::create("include.files").unwrap();
    let mut builder = phf_codegen::Map::new();
    for path in &paths {
        write!(&mut file, "#[allow(non_upper_case_globals)]\n").unwrap();
        let varname = format!("FILE_{}", uuid::Uuid::new_v4().to_simple_string());
        let incstr = format!("include_bytes!(\"../include/{}\")", path);
        write!(&mut file, "const {}: &'static [u8] = {};\n", varname, incstr).unwrap();
        builder.entry(&**path, &varname);
    }
    write!(&mut file, "static FILES: phf::Map<&'static str, &'static [u8]> = ").unwrap();
    builder.build(&mut file).unwrap();
    write!(&mut file, ";\n").unwrap();
}
