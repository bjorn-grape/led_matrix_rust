extern crate sdl2;

use std::cell::RefCell;
use std::env;
use std::path::Path;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{TextureQuery, WindowCanvas};
use std::thread;
use std::ptr;
use std::sync::{Arc, Mutex};
use sdl2::sys::SDL_EventType;
use std::time::{Duration, Instant};


const FPS: u32 = 30;
const FRAME_TIME: Duration = Duration::from_micros((1_000_000  / FPS) as u64);

const SCALE_FACTOR: u32 = 10;
static SCREEN_WIDTH: u32 = 64 * 3 * SCALE_FACTOR;
static SCREEN_HEIGHT: u32 = 64 * SCALE_FACTOR;
const CUSTOM_EVENT_TYPE: u32 = SDL_EventType::SDL_USEREVENT as u32 + 1;


// handle the annoying Rect i32
macro_rules! rect (
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

struct CustomEventData {
    // Put your custom data fields here
    message: String,
}

struct DashBoardLines {
    line1: String,
    line2: String,
    line3: String,
    line4: String,
}

impl DashBoardLines {
    fn new() -> DashBoardLines {
        DashBoardLines {
            line1: "empty".to_string(),
            line2: "empty".to_string(),
            line3: "empty".to_string(),
            line4: "empty".to_string(),
        }
    }

    fn update(mut self, line_num: i32, text: String) {
        match line_num {
            1 => { self.line1 = text; }
            2 => { self.line2 = text; }
            3 => { self.line3 = text; }
            4 => { self.line4 = text; }
            _ => { println!("Bad line number! No effect.") }
        }
    }
}

// Scale fonts to a reasonable size when they're too big (though they might look less smooth)
fn get_centered_rect(rect_width: u32, rect_height: u32, cons_width: u32, cons_height: u32) -> Rect {
    let wr = rect_width as f32 / cons_width as f32;
    let hr = rect_height as f32 / cons_height as f32;

    let (w, h) = if wr > 1f32 || hr > 1f32 {
        if wr > hr {
            println!("Scaling down! The text will look worse!");
            let h = (rect_height as f32 / wr) as i32;
            (cons_width as i32, h)
        } else {
            println!("Scaling down! The text will look worse!");
            let w = (rect_width as f32 / hr) as i32;
            (w, cons_height as i32)
        }
    } else {
        (rect_width as i32, rect_height as i32)
    };

    let cx = (SCREEN_WIDTH as i32 - w) / 2;
    let cy = (SCREEN_HEIGHT as i32 - h) / 2;
    rect!(cx, cy, w, h)
}


fn printooo(
    canvas: &mut WindowCanvas,
    lines: &[&str],
    font: &sdl2::ttf::Font,
    co_b: i32,
) {
    canvas.set_draw_color(Color::RGBA(195, 217, co_b as u8, 255));
    canvas.clear();
    let texture_creator = canvas.texture_creator();
    let line_height = 160; // You might want to adjust this value
    for (i, &line) in lines.iter().enumerate() {
        // render a surface, and convert it to a texture bound to the canvas

        let surface = font
            .render(line)
            .blended(Color::RGBA(255, 0, 0, 255))
            .map_err(|e| e.to_string()).unwrap();
        let texture = texture_creator
            .create_texture_from_surface(&surface)
            .map_err(|e| e.to_string()).unwrap();

        let TextureQuery { width, height, .. } = texture.query();

        // If the example text is too big for the screen, downscale it (and center irregardless)
        let padding = 128;
        let target = get_centered_rect(
            width,
            height,
            SCREEN_WIDTH - padding,
            SCREEN_HEIGHT - padding,
        );

        // Offset target rect to the correct line
        let target = Rect::new(0, 0 + i as i32 * line_height, target.width(), target.height());

        let aa = canvas.copy(&texture, None, Some(target));
        if aa.is_err() {
            println!("COPY ERROR");
        }
    }
}


fn update(mut sdl_context: &sdl2::Sdl,
           indexx: &mut i32,
          alive: &mut bool) {

    let mut event_pump = sdl_context.event_pump().unwrap();
    while let Some(event) = event_pump.poll_event() {
        match event {
            Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => { *alive = false; break; },
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => {
                if *indexx < 15 {
                    *indexx = 255;
                }
                *indexx -= 10;
                if *indexx < 15 {
                    *indexx = 255;
                }
            }
            _ => {}
        }
    }
}

// fn push_custom_event(event_pump: &EventPump) {
//     // Create your custom event here, for example, a custom event with type 1234
//     let custom_event = Event::User {
//         timestamp: 0,
//         window_id: 0,
//         type_: 1234, // You can use any custom integer value here
//         code: 0,
//         data1: std::ptr::null_mut(),
//         data2: std::ptr::null_mut(),
//     };
//
//     // Push the custom event into the event queue
//     event_pump.push_event(custom_event).expect("Failed to push custom event");
// }

fn run(font_path: &Path, lines: &[&str]) -> Result<(), String> {

    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let window = video_subsys
        .window("SDL2_TTF Example", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    // Load a font
    let font = ttf_context.load_font(font_path, 160).unwrap();
    // let canvas = Arc::new(Mutex::new(canvas_raw));
    // let font_mutex = Arc::new(Mutex::new(font_raw));
    // font_mutex.lock().unwrap().set_style(sdl2::ttf::FontStyle::BOLD);

    // let canvas_mutex_thread = canvas_mutex.clone();
    // let font_mutex_thread = font_mutex.clone();


    // printooo(&mut canvas,lines, &font, 255);


    // println!("Custom event ID: {}", custom_event_id);


    let event_subsystem = sdl_context.event()?;
    let event_subsystem_arc = Arc::new(Mutex::new(event_subsystem));

//     let handle = thread::spawn(|| {
//     });

    // printooo(&mut canvas, lines, &font,  0);
    // canvas.present();

    let mut indexx = 50;
    let mut last_frame_time = Instant::now();
    let mut alive= true;

    loop {
        if !alive{
            break;
        }
        // Calculate the elapsed time since the last frame
        let elapsed = last_frame_time.elapsed();

        // Update your game logic here...
        update(&sdl_context, &mut indexx, &mut alive);

        if indexx< 10 {
            indexx = 255;
        }
        indexx-=10;

        // Render your game here...
        printooo(&mut canvas, lines, &font, indexx);
        canvas.present();

        // Calculate the remaining time to reach the target frame time
        let remaining_time = FRAME_TIME.checked_sub(elapsed);

        // If there's remaining time, sleep for that duration
        if let Some(remaining) = remaining_time {
            thread::sleep(remaining);
        }
        let aa = 1000  / remaining_time.unwrap().as_millis();
        println!("{aa} FPS");
        // Set the last_frame_time to the current time to measure the next frame duration
        last_frame_time = Instant::now();
    }


    // handle.join().unwrap();

    Ok(())
}

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();

    println!("linked sdl2_ttf: {}", sdl2::ttf::get_linked_version());

    if args.len() < 2 {
        println!("Usage: ./demo font.[ttf|ttc|fon]")
    } else {
        let path: &Path = Path::new(&args[1]);
        let lines = vec![
            "T2 Tief: 10 min",
            "T3: 10 m",
            "89 Altstatten: 10 m",
            "83: 10 m",
        ];
        run(path, &lines)?;
    }

    Ok(())
}