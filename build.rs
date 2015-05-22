
fn main() {
    println!("cargo:rustc-link-search=native={}", "cpython/Modules/zlib");
}
