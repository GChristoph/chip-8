pub struct CPU {
    pc: u16,
    i_register: u16,
    registers: [u8; 16],
    delay_timer: u8,
    sound_timer: u8,
    memory: Vec<u8>,
    pub frame_buffer: Vec<bool>,
    stack: Vec<u16>,
    keypad_input: u8,

    memory_size: usize,
    frame_buffer_size: usize,
    max_stack_size: usize,
    pub redraw: bool,
    panic: bool,
    pub detailed_logging: bool,
}

impl CPU {
    pub fn new(font: &[u8], memory_size: usize, frame_buffer_size: usize, max_stack_size: usize) -> Self {
        let mut cpu = Self {
            pc: 0x200,
            i_register: 0,
            registers: [0; 16],
            delay_timer: 0,
            sound_timer: 0,
            memory: vec![0; memory_size],
            frame_buffer: vec![false; frame_buffer_size],
            stack: vec![0; max_stack_size],
            keypad_input: 0,
            memory_size,
            frame_buffer_size,
            max_stack_size,
            redraw: true,
            panic: false,
            detailed_logging: false,
        };
        cpu.memory[20..100].copy_from_slice(font);

        return cpu;
    }

    pub fn emulate_cycle(&mut self,) {
        if self.panic {
            return;
        }

        let instruction: u16 = (self.memory[self.pc as usize] as u16) << 8 | (self.memory[(self.pc + 1) as usize] as u16);
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
            },
            0x1 => self.jump_to_address(nb << 8 | nc << 4 | nd),
            0x2 => self.jump_to_subroutine(nb << 8 | nc << 4 | nd),
            0x3 => self.skip_if_equal(nb, nc << 4| nd),
            0x4 => self.skip_if_not_equal(nb, nc << 4| nd),
            0x5 => self.skip_if_x_equals_y(nb, nc),
            0x6 => self.set_register_vx(nb, (nc << 4 | nd) as u8),
            0x7 => self.add_to_register_vx(nb, (nc << 4 | nd) as u8),
            0x8 => self.arithmetic_instructions(nb, nc, nd),
            0x9 => self.skip_if_x_not_equals_y(nb, nc),
            0xA => self.set_index_register(nb << 8 | nc << 4 | nd),
            0xB => self.jump_with_offset(nb << 8 | nc << 4 | nd),
            0xD => self.draw_sprite(nb, nc, nd),
            0xF => self.f_instructions(nb, nc, nd),
            _ => self.panic_unknown_instruction(instruction),
        };
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
                self.panic = true;
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
    fn skip_if_equal(&mut self, register: u16, value: u16) {
        let reg_value = self.get_value_of_register(register);
        if reg_value == value {
            self.pc += 2;
        }
    }

    /// 0x4XNN
    /// Skip the following instruction if the value of register VX is not equal to NN
    fn skip_if_not_equal(&mut self, register: u16, value: u16) {
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
        self.set_value_of_register(x, value)
    }

    /// 0x8XY2
    /// Set VX to VX AND VY
    fn set_vx_to_vx_and_vy(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let value = x_value & y_value;
        self.set_value_of_register(x, value)
    }

    /// 0x8XY3
    /// Set VX to VX XOR VY
    fn set_vx_to_vx_xor_vy(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let value = x_value ^ y_value;
        self.set_value_of_register(x, value)
    }

    /// 0x8XY4
    /// Add VY to VX with carry
    fn add_vy_to_vx_carry(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let (value, carry) = x_value.overflowing_add(y_value);
        self.set_value_of_register(x, value);
        self.set_value_of_register(0xF, carry as u16);
    }

    /// 0x8XY5
    /// Subtract VY to VX with borrow
    fn subtract_vy_from_vx_borrow(&mut self, x: u16, y: u16) {
        let x_value = self.get_value_of_register(x);
        let y_value = self.get_value_of_register(y);
        let (value, borrow) = x_value.overflowing_sub(y_value);
        self.set_value_of_register(x, value);
        self.set_value_of_register(0xF, !borrow as u16);
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
        self.set_value_of_register(0xF, !borrow as u16);
    }

    /// 0x8XYE
    /// Shift VY left one bit and store in VX
    /// Set VF to the prior least significant bit
    fn shift_vy_one_left_store_in_vx(&mut self, x: u16, y: u16) {
        let y_value = self.get_value_of_register(y);
        let value = y_value << 1;
        self.set_value_of_register(x, value);
        self.set_value_of_register(0xF, y_value & 0b1000_0000);
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
        let reg_0 = self.get_value_of_register(0x0);
        self.pc = value + reg_0;
    }

    /// 0xCXNN
    /// Set VX to a random number with mask NN
    fn set_masked_random(&mut self, x: u16, mask: u16) {
        let number: u16 = rand::random();
        let number = number & mask;
        self.set_value_of_register(x, number);
    }

    /// 0xDXYN
    /// Draw a sprite to the screen
    fn draw_sprite(&mut self, register_x: u16, register_y: u16, n: u16) {
        //println!("Draw sprite {:x}, {:x}, {:x}, {:x}", register_x, register_y, n, self.i_register);
        let mut x_coordinate = self.registers[register_x as usize];
        let mut y_coordinate = self.registers[register_y as usize];
        //println!("X: {}, Y: {}", x_coordinate, y_coordinate);
        if x_coordinate > 63 {
            x_coordinate = (x_coordinate + 1) % 64;
        }
        if y_coordinate > 31 {
            y_coordinate = (y_coordinate + 1) % 32;
        }

        self.registers[0xF] = 0;
        for i in 0..n {
            let row = self.memory[(self.i_register + i) as usize];
            let y = ((y_coordinate as usize) + i as usize) * 64;
            for j in 0..8 {
                let pixel = (row & (0x1 << (7-j))) >> (7-j);
                let x = (x_coordinate + j) as usize;
                // Break if the draw would go over the border of the screen
                if x >= 64 {
                    //panic!("The draw would go over the border of the screen");
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

    /// 0xEX9E
    /// ...

    /// F instruction family
    fn f_instructions(&mut self, nb: u16, nc: u16, nd: u16) {
        let encoded = nc << 4 | nd;
        match encoded {
            0x07 => self.store_delay_timer_in_vx(nb),
            0x1E => self.add_vx_to_i(nb),
            0x33 => self.store_decimal_at_i(nb),
            0x55 => self.store_register_values_in_memory(nb),
            0x65 => self.load_register_values_from_memory(nb),
            _ => self.panic_unknown_instruction(0xE << 12 | nb << 8 | encoded),
        };

    }

    /// 0xFX07
    /// Store the current value of the delay timer in register VX
    fn store_delay_timer_in_vx(&mut self, x: u16) {
        let delay_timer = self.delay_timer as u16;
        self.set_value_of_register(x, delay_timer);
    }

    /// 0xFX0A
    /// Wait for a keypress and store the result in register VX
    fn store_next_keypress_in_vx(&mut self, x: u16) {

    }

    /// 0xFX15
    /// Set the delay timer to the value of register VX

    /// 0xFX18
    /// Set the sound timer to the value of register VX

    /// 0xFX1E
    /// Add the value stored in register VX to register I
    fn add_vx_to_i(&mut self, x: u16) {
        let value = self.get_value_of_register(x);
        self.i_register += value;
    }

    /// 0xFX29
    /// Set I to the memory address of the sprite data corresponding to the hexadecimal digit stored in register VX

    /// 0xFX33
    /// Store the binary-coded decimal equivalent of the value stored in register VX at addresses I, I + 1, and I + 2
    fn store_decimal_at_i(&mut self, x: u16) {
        let mut value = self.get_value_of_register(x);
        for i in (0..3).rev() {
            self.memory[(self.i_register as usize) + i] = (value % 10) as u8;
            value /= 10;
        }
    }

    /// 0xFX55
    /// Store the values of registers V0 to VX inclusive in memory starting at address I
    /// I is set to I + X + 1 after operation²
    fn store_register_values_in_memory(&mut self, x: u16) {
        for i in 0..(x+1) {
            self.memory[self.i_register as usize] = self.get_value_of_register(i as u16) as u8;
            self.i_register += 1;
        }
    }

    /// 0xFX65
    /// Fill registers V0 to VX inclusive with the values stored in memory starting at address I
    /// I is set to I + X + 1 after operation²
    fn load_register_values_from_memory(&mut self, x: u16) {
        for i in 0..(x+1) {
            let value = self.memory[self.i_register as usize] as u16;
            self.set_value_of_register(i as u16, value);
            self.i_register += 1;
        }
    }

    fn get_value_of_register(&self, register: u16) -> u16 {
        self.registers[register as usize] as u16
    }

    fn set_value_of_register(&mut self, register: u16, value: u16) {
        self.registers[register as usize] = value as u8;
    }

    fn panic_unknown_instruction(&mut self, instruction: u16) {
        let na = (instruction & 0xF000) >> 12;
        let nb = (instruction & 0x0F00) >> 8;
        let nc = (instruction & 0x00F0) >> 4;
        let nd = instruction & 0x000F;
        println!("Unknown instruction {} {} {} {}", na, nb, nc, nd);
        self.print_memory();
        self.panic = true;
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
    }

    pub fn print_value_at_i(&self) {
        let value = self.memory[self.i_register as usize];
        println!("I: {:x}", value);
    }

    pub fn set_program(&mut self, data: &[u8]) {
        self.memory[512..512+data.len()].copy_from_slice(data);
    }
}
