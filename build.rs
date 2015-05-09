
fn main() {
    println!("cargo:rustc-link-search=native={}", "cpython/Modules/zlib");
    println!("cargo:rustc-link-lib=static={}", "z");
}
