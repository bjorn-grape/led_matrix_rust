use std::env;
use std::path::PathBuf;
use bindgen::Builder;
use bindgen::CargoCallbacks;
use cc::Build;


fn main() {
       Build::new()
        .cpp(true)  // Compile a C++ library
        .flag("-std=c++11")
        .include("./3rdparty/rpi-rgb-led-matrix/include")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/bdf-font.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/led-matrix-c.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/content-streamer.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/multiplex-mappers.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/framebuffer.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/options-initialize.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/gpio.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/pixel-mapper.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/graphics.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/thread.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/led-matrix.cc")
        .file("./3rdparty/rpi-rgb-led-matrix/lib/hardware-mapping.c")
        .compile("rpi_rgb_led_matrix");



    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=rt");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=static=rpi_rgb_led_matrix");

    let bindings = Builder::default()
    .header("src/wrapper.h")
        // Add functions to the allowlist
    .allowlist_function("led_matrix_create_from_options")
    .allowlist_function("led_matrix_create_from_options_and_rt_options")
    .allowlist_function("led_matrix_print_flags")
    .allowlist_function("led_matrix_create")
    .allowlist_function("led_matrix_delete")
    .allowlist_function("led_matrix_get_canvas")
    .allowlist_function("led_canvas_get_size")
    .allowlist_function("led_canvas_set_pixel")
    .allowlist_function("led_canvas_set_pixels")
    .allowlist_function("led_canvas_clear")
    .allowlist_function("led_canvas_fill")
    .allowlist_function("led_matrix_create_offscreen_canvas")
    .allowlist_function("led_matrix_swap_on_vsync")
    .allowlist_function("led_matrix_get_brightness")
    .allowlist_function("led_matrix_set_brightness")
    // Add structs to the allowlist
    .allowlist_type("RGBLedMatrix")
    .allowlist_type("LedCanvas")
    .allowlist_type("LedFont")
    .allowlist_type("RGBLedMatrixOptions")
    .allowlist_type("RGBLedRuntimeOptions")
    .allowlist_type("Color")
    .parse_callbacks(Box::new(CargoCallbacks))
    .generate()
    .expect("Failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
