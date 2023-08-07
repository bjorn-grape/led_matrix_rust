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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::io::{self, Write};
use std::string::ToString;
use reqwest::Error;
use sdl2::libc::time;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use tokio::time::Instant;

const FPS: u32 = 5;
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
    departureTimestamp: Option<u64>,
    delay: Option<u32>,
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
    fn update_text_field(&mut self);
}

struct CustomEventData {
    // Put your custom data fields here
    message: String,
}

const URL_SBB: &str = "http://www.transport.opendata.ch/v1/connections?";

struct URLRequest {
    begin_station: String,
    end_station: String,
    fields: Vec<String>,
    limit: u32,
}

struct URLResult {
    timestamp: SystemTime,
    delay: Duration,
    error: bool,
}

impl URLResult {
    fn new(opt_departure: Option<u64>, opt_delay: Option<u32>) -> URLResult {
        if opt_departure.is_none(){
            return URLResult {
                timestamp: SystemTime::now(),
                delay: Duration::from_secs(0),
                error: true,
            };
        }

        let mut delay = 0;
        if  !opt_delay.is_none() {
            delay = opt_delay.unwrap();
        }

        let duration_since_epoch = Duration::from_secs(opt_departure.unwrap());
        let instant = UNIX_EPOCH + duration_since_epoch;
        URLResult {
            timestamp: instant,
            delay: Duration::from_secs(delay as u64 * 60),
            error: false,
        }
    }
}

// from=Lausanne&to=Gen%C3%A8ve&fields[]=connections/from/departure&fields[]=connections/from/delay&limit=5
struct DashBoardLine {
    line: String,
    color: [u8; 3],
}

struct DashBoardBusLine {
    request_content: URLRequest,
    result_list: Vec<URLResult>,
    line: String,
    basename: String,
    last_update: SystemTime,
    color: [u8; 3],
}


impl DashBoardBusLine {
    fn new(begin_station: String, end_station: String, base_name: String) -> DashBoardBusLine {
        DashBoardBusLine {
            request_content: URLRequest {
                begin_station,
                end_station,
                fields: vec![
                    "connections/from/departureTimestamp".to_string(),
                    "connections/from/delay".to_string(),
                ],

                limit: 10,
            },
            line: "Not set".to_string(),
            basename: base_name,
            result_list: vec![],
            last_update: SystemTime::now() - Duration::from_secs(3600),
            color: [255, 255, 255],
        }
    }

    async fn update(&mut self) {
        if self.last_update + Duration::from_secs(600) > SystemTime::now() {
            return;
        }
        let b_sta = &self.request_content.begin_station;
        let e_sta = &self.request_content.end_station;
        let limit = &self.request_content.limit;
        let mut ss = format!("{URL_SBB}from={b_sta}&to={e_sta}&limit={limit}");
        for elm in &self.request_content.fields {
            ss += &format!("&fields[]={elm}").as_str();
        }
        let resa = reqwest::get(&ss);
        let res = resa.await;
        if res.is_err() {
            print!("Error sending GET request: {ss}");
            return;
        }
        let res_text = res.unwrap().text().await;
        if res_text.is_err() {
            print!("Error getting text from GET request: {ss}");
            return;
        }

        let text_to_parse = res_text.unwrap();
        println!("{}",text_to_parse);
        let conn_res = serde_json::from_str(text_to_parse.as_str());
        if conn_res.is_err() {
            println!("eee is {}", conn_res.err().unwrap());
            return;
        }
        let conn: Connections = conn_res.unwrap();

        let url_results: Vec<URLResult> = conn.connections.iter()
            .map(|c|
                URLResult::new(c.from.departureTimestamp, c.from.delay))
            .collect();

        self.result_list = url_results;
        self.last_update = SystemTime::now();
    }

    fn make_line_info(&self, index :usize, now: SystemTime) -> String{
        let mut acc ="".to_string();
        let bn = &self.basename;

        if index >= self.result_list.len() {
            acc = format!("{bn}: end reached update req!");
            return acc;
        }
        let current_rr = &self.result_list[index];
        let ts = current_rr.timestamp + current_rr.delay;
        let diff_dur_res = ts.duration_since(now);
        if diff_dur_res.is_err() {
            acc = format!("{bn}: invalid time!");
            return acc;
        }
        let diff_dur = diff_dur_res.unwrap();
        let minutes = diff_dur.as_secs() / 60;
        let seconds = diff_dur.as_secs() % 60;
        acc = format!("{minutes}:{seconds}");

        let delay = current_rr.delay.as_secs() / 60;
        if delay != 0 {
            acc += format!("(+{delay})").as_str();
        }
        return acc;
    }
}

impl Printable for DashBoardLine {
    fn get_text(&self) -> String {
        self.line.clone()
    }
    fn get_color(&self) -> [u8; 3] {
        self.color.clone()
    }
    fn update_text_field(&mut self) {}
}

impl Printable for DashBoardBusLine {
    fn get_text(&self) -> String {
        self.line.clone()
    }

    fn get_color(&self) -> [u8; 3] {
        self.color.clone()
    }



    fn update_text_field(&mut self) {
        let mut rt = Runtime::new().unwrap();
        let future = self.update();
        let result = rt.block_on(future);

        let now = SystemTime::now();
        let mut index = 0;
        for rr in &self.result_list {
            if rr.error {
                index+=1;
                continue;
            }
            let is_delay_passed = now > rr.timestamp + rr.delay;

            if !is_delay_passed{
                break;
            }
            index += 1;
        }

        let len_res_list = self.result_list.len();


        let bn = &self.basename;
        if self.result_list.len() == 0 {
            self.line = format!("{bn}: len=0 update req!");
            return;
        }
        while  index < len_res_list {
            if ! self.result_list[index].error {
                break;
            }
            index+=1;
        }

        let text1 = self.make_line_info(index, now);

        index +=1;
        while  index < len_res_list {
            if ! self.result_list[index].error {
                break;
            }
            index+=1;
        }
        let text2 = self.make_line_info(index, now);
        self.line = format!("{bn}: {text1} & {text2}")
    }
}

struct DashBoardPage {
    lines: Vec<Box<dyn Printable>>,
}

impl DashBoardPage {
    fn new() -> DashBoardPage {
        DashBoardPage { lines: vec![] }
    }

    fn add_sbb_line(&mut self, base_name: String, begin: String, end: String) {
        let line = DashBoardBusLine::new(begin, end, base_name);
        self.lines.push(Box::new(line));
    }
}

struct DashBoard {
    pages: Vec<DashBoardPage>,
    curr_page: usize,
}


impl DashBoard {
    fn new() -> DashBoard {
        DashBoard {
            pages: vec![],
            curr_page: 0,
        }
    }

    fn add_page(&mut self, p: DashBoardPage) {
        self.pages.push(p);
    }

    fn update_content(&mut self) {
        let mut page = &mut self.pages[self.curr_page];
        let lines = &mut page.lines;
        for i in 0..4 {
            lines[i].update_text_field();
        }
    }

    fn get_content(&self) -> Vec<String> {
        let mut vec = Vec::new();
        if self.curr_page >= self.pages.len() {
            let cp = self.curr_page;
            for i in 0..4 {
                vec.push(format!("page {cp} missing"));
            }
            return vec;
        }
        let page: &DashBoardPage = &self.pages[self.curr_page];
        let lines = &page.lines;
        for i in 0..4 {
            let todisp = lines[i].get_text();
            vec.push(todisp);
        }
        return vec;
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
    let mut last_frame_time = SystemTime::now();
    let mut alive = true;

    let mut dbl: DashBoard = DashBoard::new();
    let mut page: DashBoardPage = DashBoardPage::new();

    page.add_sbb_line(
        "T2|HB".to_string(),
        "Zurich,Freihofstrasse".to_string(),
        "Zurich,Letzigrund".to_string(),
    );
    page.add_sbb_line(
        "T3|HB".to_string(),
        "Zurich, Siemens".to_string(),
        "Zurich, Hubertus".to_string(),
    );
    page.add_sbb_line(
        "89|Alt".to_string(),
        "Zurich,Kappeli".to_string(),
        "Zurich,Letzipark West".to_string(),
    );
    page.add_sbb_line(
        "89|Oer".to_string(),
        "Zurich,Albisrank".to_string(),
        "Zurich,Oerlikon".to_string(),
    );

    dbl.add_page(page);
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
        if elapsed.is_err() {
            println!("Err with system time");
            thread::sleep(Duration::from_secs(1));
        } else {
            // Calculate the remaining time to reach the target frame time
            let remaining_time = FRAME_TIME.checked_sub(elapsed.unwrap());

            // If there's remaining time, sleep for that duration
            // print!("\r");
            if let Some(remaining) = remaining_time {
                thread::sleep(remaining);
            }
            let aa = 1000 / remaining_time.unwrap().as_millis();

            print!("\rframe: {index_f} fps:{aa}");
        }

        io::stdout().flush();

        index_f += 1;

        // Set the last_frame_time to the current time to measure the next frame duration
        last_frame_time = SystemTime::now();
    }

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