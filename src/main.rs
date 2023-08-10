extern crate sdl2;

use std::arch::x86_64::_xsavec64;
use std::cell::RefCell;
use std::env;
use std::future::IntoFuture;
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
use sdl2::libc::{clone, time};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use tokio::time::Instant;
use image::RgbaImage;
use sdl2::pixels::PixelFormatEnum;
use image::imageops::FilterType;
use tokio::task::JoinHandle;
use chrono::{Local, Datelike, Timelike};

const FPS: u32 = 30;
const STEP: u32 = 32;
const FRAME_TIME: Duration = Duration::from_micros((1_000_000 / FPS) as u64);
const SECOND_NUM_WAIT: u32 = 2;

const SCALE_FACTOR: u32 = 8;
static SCREEN_WIDTH: u32 = 64 * 3 * SCALE_FACTOR;
static SCREEN_HEIGHT: u32 = 64 * SCALE_FACTOR;
const CUSTOM_EVENT_TYPE: u32 = SDL_EventType::SDL_USEREVENT as u32 + 1;
const LINE_HEIGHT: i32 = 16 * SCALE_FACTOR as i32;

const TOTAL_CHAR_WIDTH: usize = 24;
// You might want to adjust this value
// 10 minutes
const REFRESH_INFERVAL: Duration = Duration::from_secs(60 * 10);

#[derive(Serialize, Deserialize, Debug)]
struct Connection {
    from: Departure,
    sections: Vec<Section>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Departure {
    departureTimestamp: Option<u64>,
    delay: Option<u32>,
}


#[derive(Debug, Serialize, Deserialize)]
struct Section {
    journey: Option<Journey>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Journey {
    category: String,
    number: String,
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

struct Answer {
    last_update: SystemTime,
    result_list: Vec<URLResult>,
}

fn get_formatted_time() -> String {
    let now = Local::now();

    let month = now.month();
    let day = now.day();
    let hour = now.hour();
    let minute = now.minute();
    let second = now.second();

    let month_str = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    };

    let formatted_time = format!("{:}-{:02} {}:{:02}:{:02}", month_str, day, hour, minute, second);
    return formatted_time;
}

async fn update_request_content(request_content: URLRequest, last_update: SystemTime) -> Option<Answer> {
    // let last_update_val = *last_update.lock().unwrap();

    let b_sta = &request_content.begin_station;
    let e_sta = &request_content.end_station;
    let limit = request_content.limit;
    let mut ss = format!("{URL_SBB}from={b_sta}&to={e_sta}&limit={limit}");
    for elm in &request_content.fields {
        ss += &format!("&fields[]={elm}").as_str();
    }
    let resa = reqwest::get(&ss);
    let res = resa.await;
    println!("{}", &ss);
    if res.is_err() {
        print!("Error sending GET request: {ss}");
        return None;
    }
    let res_text = res.unwrap().text().await;
    if res_text.is_err() {
        print!("Error getting text from GET request: {ss}");
        return None;
    }

    let text_to_parse = res_text.unwrap();
    println!("{}", text_to_parse);
    let conn_res = serde_json::from_str(text_to_parse.as_str());
    if conn_res.is_err() {
        println!("eee is {}", conn_res.err().unwrap());
        return None;
    }
    let conn: Connections = conn_res.unwrap();

    let url_results: Vec<URLResult> = conn.connections.iter()
        .map(|c| {
            let mut curr_index = 0;
            let len = c.sections.len();
            while curr_index < len {
                if c.sections[curr_index].journey.is_some() {
                    break;
                }
                curr_index += 1;
            }
            let mut name = "err".to_string();
            if curr_index < len {
                let jn = ((c.sections[curr_index].journey).as_ref()).unwrap();
                name = format!("{}{}", jn.category, jn.number);
            }

            return URLResult::new(c.from.departureTimestamp, c.from.delay,
                                  name);
        }
        )
        .collect();

    return Option::Some(Answer {
        result_list: url_results,
        last_update: SystemTime::now(),
    });
}

#[derive(Clone)]
struct URLRequest {
    begin_station: String,
    end_station: String,
    fields: Vec<String>,
    limit: u32,
}

#[derive(Clone)]
struct URLResult {
    timestamp: SystemTime,
    delay: Duration,
    transport_name: String,
    error: bool,
}

impl URLResult {
    fn new(opt_departure: Option<u64>, opt_delay: Option<u32>, transport_name: String) -> URLResult {
        if opt_departure.is_none() {
            return URLResult {
                timestamp: SystemTime::now(),
                delay: Duration::from_secs(0),
                transport_name,
                error: true,
            };
        }

        let mut delay = 0;
        if !opt_delay.is_none() {
            delay = opt_delay.unwrap();
        }

        let duration_since_epoch = Duration::from_secs(opt_departure.unwrap());
        let instant = UNIX_EPOCH + duration_since_epoch;
        URLResult {
            timestamp: instant,
            delay: Duration::from_secs(delay as u64 * 60),
            transport_name: transport_name,
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
    lines: Vec<String>,
    basename: String,
    last_update: SystemTime,
    future_answer: Option<JoinHandle<Option<Answer>>>,
    color: (u8, u8, u8),
}


impl DashBoardBusLine {
    fn new(begin_station: String, end_station: String, base_name: String, color: (u8, u8, u8)) -> DashBoardBusLine {
        DashBoardBusLine {
            request_content: URLRequest {
                begin_station,
                end_station,
                fields: vec![
                    "connections/from/departureTimestamp".to_string(),
                    "connections/from/delay".to_string(),
                    "connections/sections/journey/category".to_string(),
                    "connections/sections/journey/number".to_string(),
                ],

                limit: 10,
            },
            result_list: vec![],
            lines: vec![],
            basename: add_N_padding_or_cut(base_name, TOTAL_CHAR_WIDTH),

            last_update:
            SystemTime::now() - Duration::from_secs(3600)
            ,
            future_answer: None,
            color: color,
        }
    }

    fn get_text(&self) -> Vec<String> {
        let mut res = Vec::new();
        res.push(self.basename.clone());
        for elm in &self.lines {
            res.push(elm.clone());
        }
        return res;
    }

    fn get_color(&self) -> (u8, u8, u8) {
        self.color.clone()
    }

    fn make_line_info(&self, index: usize, now: SystemTime) -> String {
        let mut acc = "".to_string();
        let bn = "err";

        let res_list_copy = self.result_list.clone();

        if index >= res_list_copy.len() {
            acc = format!("{bn}: end reached update req!");
            return acc;
        }

        let current_rr_res = &res_list_copy.get(index);
        if current_rr_res.is_none() {
            acc = format!("{bn}: out of bounds access!");
            return acc;
        }
        let current_rr = current_rr_res.unwrap();
        let ts = current_rr.timestamp + current_rr.delay;
        let diff_dur_res = ts.duration_since(now);
        if diff_dur_res.is_err() {
            acc = format!("{bn}: invalid time!");
            return acc;
        }
        let diff_dur = diff_dur_res.unwrap();
        let mut minutes = diff_dur.as_secs() / 60;
        let seconds = diff_dur.as_secs() % 60;
        let name = &current_rr.transport_name;
        if minutes < 60 {
            acc = format!("{name} {minutes}:{seconds:02}");
        } else {
            let hours = minutes / 60;
            minutes %= 60;
            acc = format!("{name} {hours}:{minutes:02}:{seconds:02}");
        }

        let delay = current_rr.delay.as_secs() / 60;
        if delay != 0 {
            acc += format!("(+{delay})").as_str();
        }
        return acc;
    }

    async fn update_text_field(&mut self) {
        let copyyy = self.request_content.clone();
        let last_upp = self.last_update.clone();
        println!("update_text_field ");
        if self.future_answer.is_none() {
            if last_upp + REFRESH_INFERVAL > SystemTime::now() {} else {
                let future = update_request_content(copyyy, last_upp);
                // *self.last_update = SystemTime::now();
                self.future_answer = Some(tokio::spawn(future));
                // self.future_answer.unwrap().into_future().await;
                println!("spawn future ");
            }
        } else if let Some(future_handle) = &mut self.future_answer {
            if future_handle.is_finished() {
                println!("future_handle");
                // Run the future to completion

                let option_answ = future_handle.into_future().await.unwrap();
                match option_answ {
                    Some(answ) => {
                        self.last_update = answ.last_update;
                        self.result_list = answ.result_list;
                    }
                    _ => {}
                }
                // self.last_update = result_answ.unwrap().;
                self.future_answer = None;
            }
        }

        let res_list_copy = self.result_list.clone();
        let now = SystemTime::now();
        let mut index = 0;
        for rr in &res_list_copy {
            if rr.error {
                index += 1;
                continue;
            }
            let is_delay_passed = now > rr.timestamp + rr.delay;

            if !is_delay_passed {
                break;
            }
            index += 1;
        }

        let len_res_list = res_list_copy.len();


        let bn = &self.basename;
        if len_res_list == 0 {
            self.lines.push(format!("{bn}: len=0 update req!"));
            return;
        }
        self.lines.clear();
        for i in 0..2 {
            let mut text_acc: String = "".to_string();
            for j in 0..2 {
                while index < len_res_list {
                    if !res_list_copy[index].error {
                        break;
                    }
                    index += 1;
                }

                let mut text1 = self.make_line_info(index, now);

                let text = add_N_padding_or_cut(text1, TOTAL_CHAR_WIDTH / 2);

                text_acc += text.as_str();
                index += 1;
            }
            self.lines.push(text_acc);
        }
    }
}

fn add_N_padding_or_cut(text1: String, max_car_num: usize) -> String {
    let mut text_res;
    if text1.len() > max_car_num {
        let first_20 = &text1[0..max_car_num];
        text_res = first_20.to_string();
    } else {
        let padding = max_car_num - text1.len();
        let supp_text = " ".repeat(padding);
        text_res = text1 + supp_text.as_str();
    }
    text_res
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


struct DashBoardPage {
    sbb_entry: Vec<Box<DashBoardBusLine>>,
    current_index: usize,
}

impl DashBoardPage {
    fn new() -> DashBoardPage {
        DashBoardPage { sbb_entry: vec![], current_index: 0 }
    }

    fn add_sbb_entry(&mut self, base_name: String, begin: String, end: String,
                     color: (u8, u8, u8)) {
        let line = DashBoardBusLine::new(begin, end, base_name, color);
        self.sbb_entry.push(Box::new(line));
    }

    fn move_to_next_sbb_entry(&mut self) {
        let index = (self.current_index + 1) % self.sbb_entry.len();
        self.current_index = index;
    }

    fn get_current_size(&self) -> usize {
        let index = self.current_index;
        return self.sbb_entry[index].lines.len() + 2;
    }

    fn get_current_entry(&self) -> &DashBoardBusLine {
        let index = self.current_index;
        return &self.sbb_entry[index];
    }

    fn get_next_entry(&self) -> &DashBoardBusLine {
        let index = (self.current_index + 1) % self.sbb_entry.len();
        return &self.sbb_entry[index];
    }
}

struct DashBoard {
    pages: Vec<DashBoardPage>,
    curr_page: usize,
}

struct DisplayLineData {
    text: String,
    color: (u8, u8, u8),
}

impl DisplayLineData {
    fn new(name: String, color: (u8, u8, u8)) -> DisplayLineData {
        DisplayLineData { text: name, color }
    }
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

    async fn update_content(&mut self) {
        let mut page = &mut self.pages[self.curr_page];
        let lines = &mut page.sbb_entry;
        for elm in lines {
            elm.update_text_field().await;
        }
    }

    fn get_content(&self) -> Vec<DisplayLineData> {
        let mut vec = Vec::new();
        if self.curr_page >= self.pages.len() {
            let cp = self.curr_page;
            for i in 0..4 {
                vec.push(DisplayLineData::new(format!("page {cp} missing"), (255, 255, 255)));
            }
            return vec;
        }
        let page: &DashBoardPage = &self.pages[self.curr_page];
        let ftime = add_N_padding_or_cut(format!("[{}]", get_formatted_time()), 24);
        vec.push(DisplayLineData::new(
            ftime.clone(),
            (255, 150, 255),
        ));
        let sbb_entry_1 = page.get_current_entry();
        vec.push(DisplayLineData::new(
            sbb_entry_1.basename.clone(),
            sbb_entry_1.get_color(),
        ));
        for elm in &sbb_entry_1.lines {
            vec.push(DisplayLineData::new(
                elm.clone(),
                sbb_entry_1.get_color(),
            ));
        }
        vec.push(DisplayLineData::new(
            ftime.clone(),
            (255, 150, 255),
        ));

        let sbb_entry_2 = page.get_next_entry();
        vec.push(DisplayLineData::new(
            sbb_entry_2.basename.clone(),
            sbb_entry_2.get_color(),
        ));
        for elm in &sbb_entry_2.lines {
            vec.push(DisplayLineData::new(
                elm.clone(),
                sbb_entry_2.get_color(),
            ));
        }

        return vec;
    }

    fn get_curr_page_size(&self) -> usize {
        let page: &DashBoardPage = &self.pages[self.curr_page];
        return page.get_current_size();
    }

    fn move_next_page_element(&mut self) {
        let mut page = &mut self.pages[self.curr_page];
        return page.move_to_next_sbb_entry();
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
    lines: &Vec<DisplayLineData>,
    font: &sdl2::ttf::Font,
    co_b: i32,
) {
    canvas.clear();
    let texture_creator = canvas.texture_creator();

    let mut pixels = vec![0; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize];

    for (i, line) in lines.iter().enumerate() {
        // render a surface, and convert it to a texture bound to the canvas

        let (r, g, b) = line.color;
        let surface = font
            .render(line.text.as_str())
            .blended(Color::RGBA(r, g, b, 255))
            .map_err(|e| e.to_string()).unwrap();
        let mut texture = texture_creator
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
        let mut change_val = 0;
        if co_b < 0 {
            change_val = co_b;
        }
        // Offset target rect to the correct line
        let target = Rect::new(0, 0 + i as i32 * LINE_HEIGHT + change_val, target.width(), target.height());

        let aa = canvas.copy(&texture, None, Some(target));
        if aa.is_err() {
            println!("COPY ERROR");
        }
    }

    let pixels_res = canvas.read_pixels(None, sdl2::pixels::PixelFormatEnum::RGBA8888);
    if pixels_res.is_err() {
        println!("cannot read pixels ");
        return;
    }

    pixels = pixels_res.unwrap();

    let image_res =
        RgbaImage::from_raw(
            SCREEN_WIDTH as u32,
            SCREEN_HEIGHT as u32,
            pixels);
    if image_res.is_none() {
        return;
    }
    let image = image_res.unwrap();
    let resized_img = image::imageops::resize(&image, 64 * 3, 64, FilterType::Nearest);
    resized_img.save(Path::new("output.png")).unwrap();
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

async fn run(font_path: &Path) -> Result<(), String> {
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

    let mut indexx = (FPS * STEP * SECOND_NUM_WAIT) as i32;
    let mut last_frame_time = SystemTime::now();
    let mut alive = true;

    let mut dbl: DashBoard = DashBoard::new();
    let mut page: DashBoardPage = DashBoardPage::new();

    page.add_sbb_entry(
        "Freihofstrasse => HB".to_string(),
        "Zurich,Freihofstrasse".to_string(),
        "Zurich,Letzigrund".to_string(),
        (255, 0, 0),
    );
    page.add_sbb_entry(
        "Siemens => HB".to_string(),
        "Zurich, Siemens".to_string(),
        "Zurich, HB".to_string(),
        (0, 255, 0),
    );
    page.add_sbb_entry(
        "Kappeli => Altstatten".to_string(),
        "Zurich,Kappeli".to_string(),
        "Zurich,Letzipark West".to_string(),
        (0, 255, 255),
    );
    page.add_sbb_entry(
        "Albisrank => Hardbrucke".to_string(),
        "Zurich,Albisrank".to_string(),
        "Zurich,Hardbrucke".to_string(),
        (255, 0, 255),
    );

    page.add_sbb_entry(
        "HB => Geneve".to_string(),
        "Zurich,HB".to_string(),
        "Geneve".to_string(),
        (255, 255, 0),
    );

    dbl.add_page(page);
    let mut index_f = 0;
    loop {
        if !alive {
            break;
        }
        // Calculate the elapsed time since the last frame
        let elapsed = last_frame_time.elapsed();

        update(&sdl_context, &mut indexx, &mut alive);

        let mini = LINE_HEIGHT * (dbl.get_curr_page_size()) as i32 * -1;
        if indexx < mini + STEP as i32 {
            indexx = (FPS * STEP * SECOND_NUM_WAIT) as i32;
            dbl.move_next_page_element();
        }
        indexx -= STEP as i32;

        dbl.update_content().await;
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

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();

    println!("linked sdl2_ttf: {}", sdl2::ttf::get_linked_version());

    if args.len() < 2 {
        println!("Usage: ./demo font.[ttf|ttc|fon]")
    } else {
        let path: &Path = Path::new(&args[1]);
        run(path).await.unwrap();
    }

    Ok(())
}