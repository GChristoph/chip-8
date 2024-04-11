mod cpu;

use std::{fs::File, io::Read};

use crate::cpu::*;
use speedy2d::{color::Color, shape::Rectangle, window::{VirtualKeyCode, WindowHandler}, Window};

const DEFAULT_MEMORY_SIZE: usize = 4 * 1024;
const DEFAULT_FRAME_BUFFER_SIZE: usize = 64 * 32;
const DEFAULT_MAX_STACK_SIZE: usize = 32;

const SCREEN_WIDTH: u32 = 1280;
const SCREEN_HEIGHT: u32 = 640;

const TC1: &str = "test-programs/IBM Logo.ch8";
const TC2: &str = "test-programs/3-corax+.ch8";
const TC3: &str = "test-programs/4-flags.ch8";

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

struct Emulator {
    cpu: CPU,
    debug_mode: bool,
}

impl WindowHandler for Emulator {
    fn on_draw(
            &mut self,
            helper: &mut speedy2d::window::WindowHelper<()>,
            graphics: &mut speedy2d::Graphics2D
        ) {
        if !self.debug_mode {
            self.cpu.emulate_cycle();
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
            _helper: &mut speedy2d::window::WindowHelper<()>,
            virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
            _scancode: speedy2d::window::KeyScancode
        ) {
        match virtual_key_code {
            Some(vcode) => match vcode {
                VirtualKeyCode::F => self.cpu.print_frame_buffer(),
                VirtualKeyCode::M => self.cpu.print_memory(),
                VirtualKeyCode::R => self.cpu.print_registers(),
                VirtualKeyCode::C => if self.debug_mode {self.cpu.emulate_cycle()},
                VirtualKeyCode::L => self.cpu.detailed_logging = !self.cpu.detailed_logging,
                VirtualKeyCode::I => self.cpu.print_value_at_i(),
                _ => (),
            },
            _ => (),
        };
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
    let program = read_ch8(TC3);
    let mut cpu = CPU::new(&FONT, DEFAULT_MEMORY_SIZE, DEFAULT_FRAME_BUFFER_SIZE, DEFAULT_MAX_STACK_SIZE);
    cpu.set_program(&program);
    let window = Window::new_centered("Title", (SCREEN_WIDTH, SCREEN_HEIGHT)).unwrap();
    window.run_loop(Emulator{ cpu, debug_mode: false, });

}
