use cbindgen::Config;
use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("failed to find Cargo.toml directory");
    let config = Config::from_file("./cbindgen.toml").expect("failed to find cbindgen.toml");

    cbindgen::generate_with_config(crate_dir, config)
        .expect("failed to generate c api bindings")
        .write_to_file("lasso.h");
}
