extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=rt");
    println!("cargo:rustc-link-lib=m");

    println!("OUT_DIR is: {}", env::var("OUT_DIR").unwrap());
    // Compile the C/C++ code
    cc::Build::new()
        .file("3rdparty/rpi-rgb-led-matrix/lib/bdf-font.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/led-matrix-c.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/content-streamer.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/multiplex-mappers.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/framebuffer.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/options-initialize.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/gpio.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/pixel-mapper.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/graphics.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/thread.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/led-matrix.cc")
        .file("3rdparty/rpi-rgb-led-matrix/lib/hardware-mapping.c")
        .include("3rdparty/rpi-rgb-led-matrix/include")
        .compile("librpi_rgb.a");

    // Generate bindings for the C headers
    let bindings = bindgen::Builder::default()
        .header("3rdparty/rpi-rgb-led-matrix/include/content-streamer.h")
        .header("3rdparty/rpi-rgb-led-matrix/include/canvas.h")
        .header("3rdparty/rpi-rgb-led-matrix/include/graphics.h")
        .header("3rdparty/rpi-rgb-led-matrix/include/pixel-mapper.h")
        .header("3rdparty/rpi-rgb-led-matrix/include/led-matrix.h")
        .header("3rdparty/rpi-rgb-led-matrix/include/thread.h")
        .header("3rdparty/rpi-rgb-led-matrix/include/led-matrix-c.h")
        .header("3rdparty/rpi-rgb-led-matrix/include/threaded-canvas-manipulator.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
