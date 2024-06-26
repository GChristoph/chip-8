use crate::keypad::{self, Keypad};
use std::time::Duration;

#[derive(PartialEq)]
enum CPUState {
    Running,
    Sleeping,
    Panic,
}

pub struct CPU {
    pc: u16,
    i_register: u16,
    registers: [u8; 16],
    delay_timer: u8,
    sound_timer: u8,
    memory: Vec<u8>,
    pub frame_buffer: Vec<bool>,
    stack: Vec<u16>,
    keypad: Keypad,
    keypad_interrupt: Option<fn(&mut CPU, u8)>,
    interrupt_register: u16,

    memory_size: usize,
    frame_buffer_size: usize,
    max_stack_size: usize,
    pub redraw: bool,
    cpu_state: CPUState,
    pub detailed_logging: bool,

    time_since_last_decrease: Duration,
}

impl CPU {
    pub fn new(
        font: &[u8],
        memory_size: usize,
        frame_buffer_size: usize,
        max_stack_size: usize,
    ) -> Self {
        let mut cpu = Self {
            pc: 0x200,
            i_register: 0,
            registers: [0; 16],
            delay_timer: 0,
            sound_timer: 0,
            memory: vec![0; memory_size],
            frame_buffer: vec![false; frame_buffer_size],
            stack: vec![0; max_stack_size],
            keypad: Keypad::new(),
            keypad_interrupt: None,
            interrupt_register: 0,
            memory_size,
            frame_buffer_size,
            max_stack_size,
            redraw: true,
            cpu_state: CPUState::Running,
            detailed_logging: false,
            time_since_last_decrease: Duration::new(0, 0),
        };
        cpu.memory[20..100].copy_from_slice(font);

        return cpu;
    }

    pub fn emulate_cycle(&mut self, delta: Duration, keypad: &Keypad) {
        if self.cpu_state == CPUState::Panic {
            return;
        }

        self.update_timers(delta);

        match self.cpu_state {
            CPUState::Running => self.execute_instruction(),
            CPUState::Sleeping => self.handle_interrupt(keypad),
            _ => (),
        }
        self.keypad = keypad.clone();
    }

    fn execute_instruction(&mut self) {
        let instruction: u16 = (self.memory[self.pc as usize] as u16) << 8
            | (self.memory[(self.pc + 1) as usize] as u16);
        self.pc += 2;
        let na = (instruction & 0xF000) >> 12;
        let nb = (instruction & 0x0F00) >> 8;
        let nc = (instruction & 0x00F0) >> 4;
        let nd = instruction & 0x000F;

        if self.detailed_logging {
            println!("Instruction: {:x} {:x} {:x} {:x}", na, nb, nc, nd);
        }

        match na {
            0x0 => {
                match instruction {
                    0x00E0 => self.clear_screen(),
                    0x00EE => self.return_from_subroutine(),
                    _ => self.panic_unknown_instruction(instruction),
                };
            }
            0x1 => self.jump_to_address(nb << 8 | nc << 4 | nd),
            0x2 => self.jump_to_subroutine(nb << 8 | nc << 4 | nd),
            0x3 => self.skip_if_equal(nb, (nc << 4 | nd) as u8),
            0x4 => self.skip_if_not_equal(nb, (nc << 4 | nd) as u8),
            0x5 => self.skip_if_x_equals_y(nb, nc),
            0x6 => self.set_register_vx(nb, (nc << 4 | nd) as u8),
            0x7 => self.add_to_register_vx(nb, (nc << 4 | nd) as u8),
            0x8 => self.arithmetic_instructions(nb, nc, nd),
            0x9 => self.skip_if_x_not_equals_y(nb, nc),
            0xA => self.set_index_register(nb << 8 | nc << 4 | nd),
            0xB => self.jump_with_offset(nb << 8 | nc << 4 | nd),
            0xC => self.set_masked_random(nb, (nc << 4 | nd) as u8),
            0xD => self.draw_sprite(nb, nc, nd),
            0xE => self.e_instructions(nb, nc, nd),
            0xF => self.f_instructions(nb, nc, nd),
            _ => self.panic_unknown_instruction(instruction),
        };
    }

    fn handle_interrupt(&mut self, keypad: &Keypad) {
        if let Some(keycode) = self.keypad.get_new_key_release(keypad) {
            match self.keypad_interrupt {
                Some(handler) => {
                    // Before calling the handler we have to set the new keypad, since
                    // the callback requires the key to be released again.
                    self.keypad = keypad.clone();
                    handler(self, keycode as u8)
                }
                None => panic!("Wanted to call interrupt handler, bot none is registered"),
            }
        }
    }

    /// Update timers with the duration that has elapsed since the last cycle
    fn update_timers(&mut self, delta: Duration) {
        self.time_since_last_decrease += delta;
        let frequency_duration = Duration::from_millis(17);
        if self.time_since_last_decrease >= frequency_duration {
            self.decrease_timers();
            self.time_since_last_decrease -= frequency_duration;
        }
    }

    fn decrease_timers(&mut self) {
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
    }

    /// 0x00E0
    /// Clear the screen
    fn clear_screen(&mut self) {
        // println!("Clear screen");
        self.frame_buffer.fill(false);
    }

    /// 0x00EE
    /// Return from a subroutine
    fn return_from_subroutine(&mut self) {
        let address = self.stack.pop();
        match address {
            Some(address) => self.pc = address,
            None => {
                println!("Return from subroutine failed because stack was empty");
                self.cpu_state = CPUState::Panic;
            }
        }
    }

    /// 0x1NNN
    /// Jump to address NNN
    fn jump_to_address(&mut self, address: u16) {
        // println!("Jump to address {:x}", address);
        self.pc = address;
    }

    /// 0x2NNN
    /// Execute subroutine starting at address NNN
    fn jump_to_subroutine(&mut self, address: u16) {
        self.stack.push(self.pc);
        self.pc = address;
    }

    /// 0x3XNN
    /// Skip the following instruction if the value of register VX equals NN
    fn skip_if_equal(&mut self, register: u16, value: u8) {
        let reg_value = self.get_value_of_register(register);
        if reg_value == value {
            self.pc += 2;
        }
    }

    /// 0x4XNN
    /// Skip the following instruction if the value of register VX is not equal to NN
    fn skip_if_not_equal(&mut self, register: u16, value: u8) {
        let reg_value = self.get_value_of_register(register);
        if reg_value != value {
            self.pc += 2;
        }
    }

    /// 0x5XY0
    /// Skip the following instruction if the value of register VX is equal to the value of register VY
    fn skip_if_x_equals_y(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        if x_value == y_value {
            self.pc += 2;
        }
    }

    /// 0x6XNN
    /// Store number NN in register VX
    fn set_register_vx(&mut self, register: u16, number: u8) {
        //println!("Set register {:x}, {:x}", register, number);
        self.registers[register as usize] = number;
    }

    /// 0x7XNN
    /// Add the value NN to register VX
    fn add_to_register_vx(&mut self, register: u16, number: u8) {
        // println!("Add to register {:x} {:x}", register, number);
        let value = self.get_value_of_register(register) as u8;
        (self.registers[register as usize], _) = value.overflowing_add(number);
    }

    /// 0x8...
    /// Handler for the 0x8 instruction family
    fn arithmetic_instructions(&mut self, nb: u16, nc: u16, nd: u16) {
        match nd {
            0x0 => self.store_vy_in_vx(nb, nc),
            0x1 => self.set_vx_to_vx_or_vy(nb, nc),
            0x2 => self.set_vx_to_vx_and_vy(nb, nc),
            0x3 => self.set_vx_to_vx_xor_vy(nb, nc),
            0x4 => self.add_vy_to_vx_carry(nb, nc),
            0x5 => self.subtract_vy_from_vx_borrow(nb, nc),
            0x6 => self.shift_vy_one_right_store_in_vx(nb, nc),
            0x7 => self.subtract_vx_from_vy_borrow(nb, nc),
            0xE => self.shift_vy_one_left_store_in_vx(nb, nc),
            _ => self.panic_unknown_instruction(0x8 << 12 | nb << 8 | nc << 4 | nd),
        }
    }

    /// 0x8XY0
    /// Store the value of register VY in register VX
    fn store_vy_in_vx(&mut self, x: u16, y: u16) {
        let y_value = self.get_value_of_register(y);
        self.set_value_of_register(x, y_value);
    }

    /// 0x8XY1
    /// Set VX to VX OR VY
    fn set_vx_to_vx_or_vy(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let value = x_value | y_value;
        self.reset_flag_register();
        self.set_value_of_register(x, value)
    }

    /// 0x8XY2
    /// Set VX to VX AND VY
    fn set_vx_to_vx_and_vy(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let value = x_value & y_value;
        self.reset_flag_register();
        self.set_value_of_register(x, value)
    }

    /// 0x8XY3
    /// Set VX to VX XOR VY
    fn set_vx_to_vx_xor_vy(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let value = x_value ^ y_value;
        self.reset_flag_register();
        self.set_value_of_register(x, value)
    }

    /// 0x8XY4
    /// Add VY to VX with carry
    fn add_vy_to_vx_carry(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let (value, carry) = x_value.overflowing_add(y_value);
        self.set_value_of_register(x, value);
        self.set_value_of_register(0xF, carry as u8);
    }

    /// 0x8XY5
    /// Subtract VY to VX with borrow
    fn subtract_vy_from_vx_borrow(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let (value, borrow) = x_value.overflowing_sub(y_value);
        self.set_value_of_register(x, value);
        self.set_value_of_register(0xF, !borrow as u8);
    }

    /// 0x8XY6
    /// Shift VY right one bit and store in VX
    /// Set VF to the prior least significant bit
    fn shift_vy_one_right_store_in_vx(&mut self, x: u16, y: u16) {
        let y_value = self.get_value_of_register(y);
        let value = y_value >> 1;
        self.set_value_of_register(x, value);
        self.set_value_of_register(0xF, y_value & 0x1);
    }

    /// 0x8XY7
    /// VX = VY - VX with borrow
    fn subtract_vx_from_vy_borrow(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let (value, borrow) = y_value.overflowing_sub(x_value);
        self.set_value_of_register(x, value);
        self.set_value_of_register(0xF, !borrow as u8);
    }

    /// 0x8XYE
    /// Shift VY left one bit and store in VX
    /// Set VF to the prior most significant bit
    fn shift_vy_one_left_store_in_vx(&mut self, x: u16, y: u16) {
        let y_value = self.get_value_of_register(y);
        let value = y_value << 1;
        self.set_value_of_register(x, value);
        self.set_value_of_register(0xF, (y_value & 0b1000_0000) >> 7);
    }

    /// 0x9XY0
    /// Skip the following instruction if the value of register VX is not equal to the value of register VY
    fn skip_if_x_not_equals_y(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        if x_value != y_value {
            self.pc += 2;
        }
    }

    /// 0xANNN
    /// Store memory address NNN in register I
    fn set_index_register(&mut self, value: u16) {
        // println!("set Index register {:x}", value);
        self.i_register = value;
    }

    /// 0xBNNN
    /// Jump with offset
    fn jump_with_offset(&mut self, value: u16) {
        let reg_0 = self.get_value_of_register(0x0) as u16;
        self.pc = value + reg_0;
    }

    /// 0xCXNN
    /// Set VX to a random number with mask NN
    fn set_masked_random(&mut self, x: u16, mask: u8) {
        let number: u8 = rand::random();
        let number = number & mask;
        self.set_value_of_register(x, number);
    }

    /// 0xDXYN
    /// Draw a sprite to the screen
    fn draw_sprite(&mut self, register_x: u16, register_y: u16, n: u16) {
        //println!("Draw sprite {:x}, {:x}, {:x}, {:x}", register_x, register_y, n, self.i_register);
        let mut x_coordinate = self.registers[register_x as usize] as u16;
        let mut y_coordinate = self.registers[register_y as usize] as u16;
        if x_coordinate > 63 {
            x_coordinate = x_coordinate % 64;
        }
        if y_coordinate > 31 {
            y_coordinate = y_coordinate % 32;
        }

        self.registers[0xF] = 0;
        for i in 0..n {
            let row = self.memory[(self.i_register + i) as usize];
            let y = ((y_coordinate + i) as usize) * 64;
            if y_coordinate + i > 31 {
                break;
            }
            for j in 0..8 {
                let pixel = (row & (0x1 << (7 - j))) >> (7 - j);
                let x = (x_coordinate + j) as usize;

                if x > 63 {
                    break;
                }

                if pixel == 1 {
                    if self.frame_buffer[x + y] {
                        self.frame_buffer[x + y] = false;
                        // A collision occured, set VF to 1
                        self.registers[0xF] = 1;
                    } else {
                        self.frame_buffer[x + y] = true;
                    }
                }
            }
        }

        self.redraw = true;
    }

    fn e_instructions(&mut self, nb: u16, nc: u16, nd: u16) {
        let encoded = nc << 4 | nd;
        match encoded {
            0x9E => self.skip_if_pressed(nb),
            0xA1 => self.skip_if_not_pressed(nb),
            _ => self.panic_unknown_instruction(0xE << 12 | nb << 8 | encoded),
        }
    }

    /// 0xEX9E
    /// Skip next instruction if key in VX is pressed
    fn skip_if_pressed(&mut self, x: u16) {
        let value = self.get_value_of_register(x);
        if value > 16 {
            println!("Keycode passed to 0xEX9E was > 16: {}", value);
            self.cpu_state = CPUState::Panic;
            return;
        }
        if self.keypad.is_key_pressed(value as usize) {
            self.pc += 2;
        }
    }

    /// 0xEXA1
    /// Skip next instruction if the key in VX is NOT pressed
    fn skip_if_not_pressed(&mut self, x: u16) {
        let value = self.get_value_of_register(x);
        if value > 16 {
            println!("Keycode passed to 0xEX9E was > 16: {}", value);
            self.cpu_state = CPUState::Panic;
            return;
        }
        if !self.keypad.is_key_pressed(value as usize) {
            self.pc += 2;
        }
    }

    /// F instruction family
    fn f_instructions(&mut self, nb: u16, nc: u16, nd: u16) {
        let encoded = nc << 4 | nd;
        match encoded {
            0x07 => self.store_delay_timer_in_vx(nb),
            0x0A => self.store_next_keypress_in_vx(nb),
            0x15 => self.set_timer_delay(nb),
            0x18 => self.set_sound_delay(nb),
            0x1E => self.add_vx_to_i(nb),
            0x33 => self.store_decimal_at_i(nb),
            0x55 => self.store_register_values_in_memory(nb),
            0x65 => self.load_register_values_from_memory(nb),
            _ => self.panic_unknown_instruction(0xF << 12 | nb << 8 | encoded),
        };
    }

    /// 0xFX07
    /// Store the current value of the delay timer in register VX
    fn store_delay_timer_in_vx(&mut self, x: u16) {
        let delay_timer = self.delay_timer;
        self.set_value_of_register(x, delay_timer);
    }

    /// 0xFX0A
    /// Wait for a keypress and store the result in register VX
    fn store_next_keypress_in_vx(&mut self, x: u16) {
        self.interrupt_register = x;
        self.keypad_interrupt = Some(CPU::store_next_keypress_in_vx_interrupt);
        self.cpu_state = CPUState::Sleeping;
    }

    fn store_next_keypress_in_vx_interrupt(cpu: &mut CPU, keycode: u8) {
        cpu.set_value_of_register(cpu.interrupt_register, keycode);
        cpu.keypad_interrupt = None;
        cpu.cpu_state = CPUState::Running;
    }

    /// 0xFX15
    /// Set the delay timer to the value of register VX
    fn set_timer_delay(&mut self, x: u16) {
        self.delay_timer = self.get_value_of_register(x);
    }

    /// 0xFX18
    /// Set the sound timer to the value of register VX
    fn set_sound_delay(&mut self, x: u16) {
        self.sound_timer = self.get_value_of_register(x);
    }

    /// 0xFX1E
    /// Add the value stored in register VX to register I
    fn add_vx_to_i(&mut self, x: u16) {
        let value = self.get_value_of_register(x) as u16;
        self.i_register += value;
    }

    /// 0xFX29
    /// Set I to the memory address of the sprite data corresponding to the hexadecimal digit stored in register VX

    /// 0xFX33
    /// Store the binary-coded decimal equivalent of the value stored in register VX at addresses I, I + 1, and I + 2
    fn store_decimal_at_i(&mut self, x: u16) {
        let mut value = self.get_value_of_register(x);
        for i in (0..3).rev() {
            self.memory[(self.i_register as usize) + i] = value % 10;
            value /= 10;
        }
    }

    /// 0xFX55
    /// Store the values of registers V0 to VX inclusive in memory starting at address I
    /// I is set to I + X + 1 after operation²
    fn store_register_values_in_memory(&mut self, x: u16) {
        for i in 0..(x + 1) {
            self.memory[self.i_register as usize] = self.get_value_of_register(i as u16) as u8;
            self.i_register += 1;
        }
    }

    /// 0xFX65
    /// Fill registers V0 to VX inclusive with the values stored in memory starting at address I
    /// I is set to I + X + 1 after operation²
    fn load_register_values_from_memory(&mut self, x: u16) {
        for i in 0..(x + 1) {
            let value = self.memory[self.i_register as usize];
            self.set_value_of_register(i as u16, value);
            self.i_register += 1;
        }
    }

    fn get_value_of_register(&self, register: u16) -> u8 {
        self.registers[register as usize]
    }

    fn set_value_of_register(&mut self, register: u16, value: u8) {
        self.registers[register as usize] = value
    }

    fn reset_flag_register(&mut self) {
        self.set_value_of_register(0xF, 0)
    }

    fn panic_unknown_instruction(&mut self, instruction: u16) {
        let na = (instruction & 0xF000) >> 12;
        let nb = (instruction & 0x0F00) >> 8;
        let nc = (instruction & 0x00F0) >> 4;
        let nd = instruction & 0x000F;
        println!("Unknown instruction {:x} {:x} {:x} {:x}", na, nb, nc, nd);
        self.print_memory();
        self.cpu_state = CPUState::Panic;
    }

    pub fn print_memory(&self) {
        for i in 0..self.memory_size / 32 {
            print!("{:>8x}  ", i * 32);
            for j in 0..32 {
                print!("{:>3} ", self.memory[i * 32 + j]);
            }
            println!();
            // Temporarily we don't need to print out more than that of the memory
            if i * 32 > 1500 {
                break;
            }
        }
    }

    pub fn print_frame_buffer(&self) {
        for y in 0..32 {
            for x in 0..64 {
                print!("{}", self.frame_buffer[y * 64 + x] as i32);
            }
            println!();
        }
    }

    pub fn print_registers(&self) {
        for i in 0..16 {
            print!("{:>3} ", i);
        }
        println!();
        for i in 0..16 {
            print!("{:>3} ", self.registers[i]);
        }
        println!();
        println!("Index: {:>3x}", self.i_register);
        println!("PC:    {:>3x}", self.pc);
    }

    pub fn print_value_at_i(&self) {
        let value = self.memory[self.i_register as usize];
        println!("I: {:x}", value);
    }

    pub fn set_program(&mut self, data: &[u8]) {
        self.memory[512..512 + data.len()].copy_from_slice(data);
    }
}
