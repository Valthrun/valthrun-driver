use std::{
    env,
    path::Path,
};

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let include_out_dir = Path::new(&env::var("OUT_DIR").unwrap())
        .join("..")
        .join("..")
        .join("..")
        .join("generated")
        .join("include");

    cbindgen::generate(crate_dir).map_or_else(
        |error| match error {
            cbindgen::Error::ParseSyntaxError { .. } => {}
            e => panic!("{:?}", e),
        },
        |bindings| {
            bindings.write_to_file(include_out_dir.join("libum_ffi.h"));
        },
    );
}
