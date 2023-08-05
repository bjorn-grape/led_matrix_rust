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
use std::io::{self, Write};
use std::string::ToString;
use reqwest::Error;
use sdl2::libc::time;
use serde::{Deserialize, Serialize};

const FPS: u32 = 30;
const FRAME_TIME: Duration = Duration::from_micros((1_000_000 / FPS) as u64);

const SCALE_FACTOR: u32 = 10;
static SCREEN_WIDTH: u32 = 64 * 3 * SCALE_FACTOR;
static SCREEN_HEIGHT: u32 = 64 * SCALE_FACTOR;
const CUSTOM_EVENT_TYPE: u32 = SDL_EventType::SDL_USEREVENT as u32 + 1;


#[derive(Serialize, Deserialize, Debug)]
struct Connection {
    from: Departure,
}

#[derive(Serialize, Deserialize, Debug)]
struct Departure {
    departure: DateTime<FixedOffset>,
    delay: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Connections {
    connections: Vec<Connection>,
}

// handle the annoying Rect i32
macro_rules! rect (
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

trait Printable {
    fn get_text(&self) -> String;
    fn get_color(&self) -> [u8; 3];
}

struct CustomEventData {
    // Put your custom data fields here
    message: String,
}

const URL_SBB: &str = "http://transport.opendata.ch/v1/connections?";

struct URLRequest {
    begin_station: String,
    end_station: String,
    fields: Vec<String>,
    limit: u32,
}

struct URLResult {
    timestamp: Instant,
    delay: Duration
}

// from=Lausanne&to=Gen%C3%A8ve&fields[]=connections/from/departure&fields[]=connections/from/delay&limit=5
struct DashBoardLine {
    line: String,
    color: [u8; 3],
}

struct DashBoardBusLine {
    request_content: URLRequest,
    result_list: Vec<URLResult>,
    last_update: Instant,
    color: [u8; 3],
}


impl DashBoardBusLine{
    fn new(begin_station:String, end_station:String ) -> DashBoardBusLine{
        DashBoardBusLine{
            request_content: URLRequest{
                begin_station,
                end_station,
                fields: vec![
                    "connections/from/departure".to_string(),
                    "connections/from/delay".to_string(),
                ],
                limit: 10
            },
            result_list: vec![],
            last_update: Instant::now() - Duration::from_secs(3600),
            color: [255,255,255],
        }
    }

    async fn update(&self){
        if self.last_update + Duration::from_secs(600) < Instant::now() {
            return;
        }
        let b_sta = &self.request_content.begin_station;
        let e_sta = &self.request_content.end_station;
        let limit = &self.request_content.limit;
        let mut ss = format!("{URL_SBB}from={b_sta}&to={e_sta}&limit={limit}");
        for elm in &self.request_content.fields {
            ss += format!("&fields[]={elm}").as_str();
        }
        let resa = reqwest::get(ss);
        let res = resa.await;
        if res.is_err(){
            print!("Error sending GET request: {ss}");
            return;
        }
        let res_text = res.unwrap().text().await;
        if res_text.is_err(){
            print!("Error getting text from GET request: {ss}");
            return;
        }

        let text_to_parse = res_text.unwrap();
        let conn_res = serde_json::from_str(text_to_parse);
        if conn_res.is_err() {
            return;

        }
        let conn: Connections = conn_res.unwrap();

        conn.connections.map



    }


}

impl Printable for DashBoardLine {
    fn get_text(&self) -> String {
        self.line.clone()
    }
    fn get_color(&self) -> [u8; 3] {
        self.color.clone()
    }
}

struct DashBoardPage {
    lines: [dyn Printable; 4],
}

struct DashBoard {
    pages: Vec<DashBoardPage>,
}

#[tokio::main]
async fn make_request() -> Result<(), Error> {
    let response = reqwest::get("https://google.com").await?;
    let body = response.text().await?;
    println!("{}", body);
    Ok(())
}

impl DashBoard {
    fn new() -> DashBoard {
        DashBoard {
            pages: vec![]
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

    fn get_content(&self) -> Vec<String> {
        let mut vec = Vec::new();
        vec.push(self.line1.clone());
        vec.push(self.line2.clone());
        vec.push(self.line3.clone());
        vec.push(self.line4.clone());
        return vec;
    }

    fn update_content(&self) {}
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
    lines: &Vec<String>,
    font: &sdl2::ttf::Font,
    co_b: i32,
) {
    canvas.set_draw_color(Color::RGBA(195, 217, co_b as u8, 255));
    canvas.clear();
    let texture_creator = canvas.texture_creator();
    let line_height = 160; // You might want to adjust this value
    for (i, line) in lines.iter().enumerate() {
        // render a surface, and convert it to a texture bound to the canvas

        let surface = font
            .render(line.as_str())
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
            } => {
                *alive = false;
                break;
            }
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

fn run(font_path: &Path) -> Result<(), String> {
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

    let event_subsystem = sdl_context.event()?;
    let event_subsystem_arc = Arc::new(Mutex::new(event_subsystem));

    let mut indexx = 50;
    let mut last_frame_time = Instant::now();
    let mut alive = true;

    let dbl = DashBoard::new();
    let mut index_f = 0;
    loop {
        if !alive {
            break;
        }
        // Calculate the elapsed time since the last frame
        let elapsed = last_frame_time.elapsed();

        // Update your game logic here...
        update(&sdl_context, &mut indexx, &mut alive);

        if indexx < 10 {
            indexx = 255;
        }
        indexx -= 10;

        dbl.update_content();
        let lines = dbl.get_content();

        // Render your game here...
        printooo(&mut canvas, &lines, &font, indexx);
        canvas.present();

        // Calculate the remaining time to reach the target frame time
        let remaining_time = FRAME_TIME.checked_sub(elapsed);

        // If there's remaining time, sleep for that duration
        // print!("\r");
        if let Some(remaining) = remaining_time {
            thread::sleep(remaining);
        }
        let aa = 1000 / remaining_time.unwrap().as_millis();

        print!("\rframe: {index_f} fps:{aa}");
        io::stdout().flush();
        index_f += 1;

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

        run(path)?;
    }

    Ok(())
}