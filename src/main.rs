mod cpu;
mod keypad;

use std::{fs::File, io::Read, thread, time::{Duration, Instant}};

use crate::cpu::*;
use keypad::{Keypad, KEY_MAP};
use speedy2d::{color::Color, dimen::UVec2, shape::Rectangle, window::{VirtualKeyCode, WindowCreationOptions, WindowHandler, WindowSize}, Window};

const DEFAULT_MEMORY_SIZE: usize = 4 * 1024;
const DEFAULT_FRAME_BUFFER_SIZE: usize = 64 * 32;
const DEFAULT_MAX_STACK_SIZE: usize = 32;

const SCREEN_WIDTH: u32 = 1280;
const SCREEN_HEIGHT: u32 = 640;

const TC1: &str = "test-programs/IBM Logo.ch8";
const TC2: &str = "test-programs/3-corax+.ch8";
const TC3: &str = "test-programs/4-flags.ch8";
const TC4: &str = "test-programs/5-quirks.ch8";
const TC5: &str = "test-programs/6-keypad.ch8";

const G1: &str  = "games/br8kout.ch8";
const G2: &str  = "games/spaceracer.ch8";

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

static SECOND: Duration = Duration::from_secs(1);

struct Emulator {
    cpu: CPU,
    debug_mode: bool,
    keypad: Keypad,
    last_cycle: Instant,
    target_cycle_duration: Duration,

    cycle_counter: usize,
    fps_duratin_counter: Instant,
    fps_measurement_duration: Duration,
}

impl Emulator {
    fn new (cpu: CPU, target_fps: u64, debug_mode: bool) -> Self {
        let duration = Duration::from_micros(1_000_000 / target_fps);
        Self {
            cpu,
            debug_mode,
            keypad: Keypad::new(),
            cycle_counter: 0,
            target_cycle_duration: duration,
            fps_measurement_duration: Duration::new(0, 0),
            last_cycle: Instant::now(),
            fps_duratin_counter: Instant::now(),
        }
    }

    fn emulate_cycle(&mut self) {
        // self.print_fps();

        let delta = self.last_cycle.elapsed();
        self.cpu.emulate_cycle(delta, &self.keypad);
        self.synch_fps(delta);

        self.cycle_counter += 1;
        self.last_cycle = Instant::now();
    }

    fn synch_fps(&mut self, delta: Duration) {
        if delta > self.target_cycle_duration {
            return;
        }
        let difference = self.target_cycle_duration - delta;
        //println!("Difference: {:?}", difference);
        thread::sleep(difference);
    }

    fn print_fps(&mut self) {
        let difference = self.fps_duratin_counter.elapsed();
        self.fps_duratin_counter = Instant::now();
        self.fps_measurement_duration += difference;
        if self.fps_measurement_duration > SECOND {
            self.fps_measurement_duration = Duration::from_secs(0);
            println!("Cycles this second: {}", self.cycle_counter);
            self.cycle_counter = 0;
        }
    }
}

impl WindowHandler for Emulator {
    fn on_draw(
            &mut self,
            helper: &mut speedy2d::window::WindowHelper<()>,
            graphics: &mut speedy2d::Graphics2D
        ) {
        if !self.debug_mode {
            self.emulate_cycle();
        }

        if self.cpu.redraw || self.debug_mode || true {
            self.cpu.redraw = false;
            graphics.clear_screen(Color::DARK_GRAY);
            let width: f32 = SCREEN_WIDTH as f32 / 64.0;
            let height: f32 = SCREEN_HEIGHT as f32 / 32.0;
            for y in 0..32 {
                for x in 0..64 {
                    if self.cpu.frame_buffer[y * 64 + x] {
                        let y: f32 = y as f32;
                        let x: f32 = x as f32;
                        graphics.draw_rectangle(Rectangle::from_tuples((width * x, height * y), (width * x + width, height * y + height)), Color::WHITE);
                    }
                }
            }
        }

        helper.request_redraw();
    }

    fn on_key_down(
            &mut self,
            helper: &mut speedy2d::window::WindowHelper<()>,
            virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
            _scancode: speedy2d::window::KeyScancode
        ) {
        match virtual_key_code {
            Some(vcode) => match vcode {
                VirtualKeyCode::B => self.cpu.print_frame_buffer(),
                VirtualKeyCode::M => self.cpu.print_memory(),
                VirtualKeyCode::P => self.cpu.print_registers(),
                VirtualKeyCode::N => if self.debug_mode { self.emulate_cycle(); },
                VirtualKeyCode::L => self.cpu.detailed_logging = !self.cpu.detailed_logging,
                VirtualKeyCode::I => self.cpu.print_value_at_i(),
                _ => {
                    if KEY_MAP.contains_key(&vcode) {
                        let id = KEY_MAP[&vcode];
                        self.keypad.key_down(id);
                    }
                }
            },
            _ => (),
        };
        helper.request_redraw();
    }

    fn on_key_up(
            &mut self,
            helper: &mut speedy2d::window::WindowHelper<()>,
            virtual_key_code: Option<VirtualKeyCode>,
            _scancode: speedy2d::window::KeyScancode
        ) {
        if let Some(vcode) = virtual_key_code {
            if KEY_MAP.contains_key(&vcode) {
                let id = KEY_MAP[&vcode];
                self.keypad.key_up(id);
            }
        }
        helper.request_redraw();
    }

}

fn read_ch8(file_path: &str) -> Vec<u8> {
    let mut file = File::open(file_path).expect("Couldn't open file");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("Couldn't read file");
    println!("Program length: {}", contents.len());
    return contents;
}

fn main() {
    let program = read_ch8(G1);
    let mut cpu = CPU::new(&FONT, DEFAULT_MEMORY_SIZE, DEFAULT_FRAME_BUFFER_SIZE, DEFAULT_MAX_STACK_SIZE);
    cpu.set_program(&program);
    let options = WindowCreationOptions::new_windowed(WindowSize::PhysicalPixels(UVec2::new(SCREEN_WIDTH, SCREEN_HEIGHT)), None).with_vsync(false);
    let window = Window::new_with_options("Title", options).unwrap();

    window.run_loop(Emulator::new(cpu, 10000, false));

}
