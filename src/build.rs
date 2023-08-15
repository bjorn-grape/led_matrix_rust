use std::env;
use std::path::PathBuf;
use bindgen::Builder;
use bindgen::CargoCallbacks;

fn main() {
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=rt");
    println!("cargo:rustc-link-lib=m");

    let bindings = Builder::default()
    .clang_arg("--std=c++11") 
    .header("src/wrapper.hpp")
        .allowlist_type("rgb_matrix::RGBMatrix")
        .allowlist_type("rgb_matrix::FrameCanvas")
        .allowlist_type("rgb_matrix::RGBMatrix::Options")
        .allowlist_type("RuntimeOptions")
        .allowlist_function("rgb_matrix::RGBMatrix::CreateFromOptions")
        .allowlist_function("rgb_matrix::RGBMatrix::CreateFromFlags")
        .allowlist_function("rgb_matrix::RGBMatrix::width")
        .allowlist_function("rgb_matrix::RGBMatrix::height")
        .allowlist_function("rgb_matrix::RGBMatrix::SetPixel")
        .allowlist_function("rgb_matrix::RGBMatrix::Clear")
        .allowlist_function("rgb_matrix::RGBMatrix::Fill")
        .allowlist_function("rgb_matrix::RGBMatrix::CreateFrameCanvas")
        .allowlist_function("rgb_matrix::RGBMatrix::SwapOnVSync")
        .allowlist_function("rgb_matrix::RGBMatrix::ApplyPixelMapper")
        .allowlist_function("rgb_matrix::RGBMatrix::SetPWMBits")
        .allowlist_function("rgb_matrix::RGBMatrix::pwmbits")
        .allowlist_function("rgb_matrix::RGBMatrix::set_luminance_correct")
        .allowlist_function("rgb_matrix::RGBMatrix::luminance_correct")
    .parse_callbacks(Box::new(CargoCallbacks))
    .generate()
    .expect("Failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
