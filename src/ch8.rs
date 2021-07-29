use anyhow::{Context, Result, bail};
use std::{fs::{File}, io::Read};
use rand::{Rng, prelude::ThreadRng};

static CH8_FONT: &'static [u8] = &[                    
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

pub struct Chip8 {
    registers: Vec<u8>, // 16 u8 registers from V(x0) to V(xF)
    memory: Vec<u8>, // total chip8 memory is 4096 (4k)
    stack: Vec<u16>, // stack can hold 16 u16's

    display: Vec<u32>, // 64x32 pixel display

    kp_input: Vec<u32>, // keypad input

    index: u16, // instruction index
    pc: usize,
    sp: u8,

    delay_timer: u8,
    sound_timer: u8,

    should_draw: bool,
    debug_enabled: bool,
    rng: ThreadRng
}

impl Chip8 {
    pub fn new(debug_enabled: bool) -> Self { 
        let mut s = Self { 
            registers: vec![0; 16],
            memory: vec![0; 4096],
            stack: vec![0; 16],
            display: vec![0; 64 * 32],
            kp_input: vec![0; 16],
            index: 0,
            pc: 0x200,
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            should_draw: false,
            debug_enabled,
            rng: rand::thread_rng()
        };

        // load fontset into memory
        for ii in 0..80 {
            s.memory[ii] = CH8_FONT[ii];
        }

        s
    }

    pub fn read_rom(&mut self, path: &String) -> Result<()> {
        let mut buffer = Vec::new();
        let mut file = File::open(path).context("invalid rom path supplied")?;

        file.read_to_end(&mut buffer).context("failed to read rom file")?;

        let mut j = 512;
        for ii in 0..buffer.len() {
            if j >= 4096 {
                bail!("file is too big to load into memory");
            }

            self.memory[j] = buffer[ii];
            j += 1;
        }

        Ok(())
    }

    pub fn cycle(&mut self) {      
        let opcode = (i32::from(self.memory[self.pc]) << 8) | i32::from(self.memory[self.pc + 1]);
        let instruction = shift_i32(opcode, 12, 0xF000);

        match instruction {
            0 => {
                match opcode {
                    0x00E0 => self.cls(),
                    0x00EE => self.ret(),
                    _ => { }
                }
            }
            1 => self.jmp(opcode & 0x0FFF),
            2 => self.call(opcode & 0x0FFF),
            3 => self.se_val(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 0, 0x00FF)),
            4 => self.sne_val(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 0, 0x00FF)),
            5 => self.se_reg(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
            6 => self.ld_reg( shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 0, 0x00FF)),
            7 => self.add_val(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 0, 0x00FF)),
            8 => {
                match shift_i32(opcode, 0, 0x000F) {
                    0 => self.copy(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
                    1 => self.or(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
                    2 => self.and(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
                    3 => self.xor(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
                    4 => self.add_reg(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
                    5 => self.sub_regxy(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
                    6 => self.shift_r(shift_u8(opcode, 8, 0x0F00)),
                    7 => self.sub_regyx(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
                    14 => self.shift_l(shift_u8(opcode, 8, 0x0F00)),
                    _ => { }
                }
            }
            9 => self.sne_reg(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x0F0)),
            10 => self.ld_indx(opcode & 0x0FFF),
            11 => self.jmpadd(opcode & 0x0FFF),
            12 => self.rand_and(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 0, 0x00FF)),
            13 => self.draw_pixel(shift_u8(opcode, 8, 0x0F00), shift_u8(opcode, 4, 0x00F0), opcode & 0x000F),
            15 => {
                match shift_i32(opcode, 0, 0x00FF) {
                    7 => self.get_delay(shift_u8(opcode, 8, 0x0F00)),
                    10 => self.wait_key(shift_u8(opcode, 8, 0x0F00)),
                    21 => self.set_delay(shift_u8(opcode, 8, 0x0F00)),
                    24 => self.set_sound(shift_u8(opcode, 8, 0x0F00)),
                    30 => self.add_indx(shift_u8(opcode, 8, 0x0F00)),
                    51 => self.encode_save(shift_u8(opcode, 8, 0x0F00)),
                    85 => self.save(shift_u8(opcode, 8, 0x0F00)),
                    101 => self.load(shift_u8(opcode, 8, 0x0F00)),
                    _ => println!("missing -> {}", shift_i32(opcode, 0, 0x00FF))
                }
            }
            _ => println!("unimplemented instruction {}", instruction)
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn should_draw(&self) -> bool {
        self.should_draw
    }

    pub fn set_should_draw(&mut self, should_draw: bool) {
        self.should_draw = should_draw;
    }

    pub fn get_display(&self, index: usize) -> u32 {
        self.display[index]
    }

    // instruction(00E0): clear display
    pub fn cls(&mut self) {
        if self.debug_enabled {
            println!("cls");
        }

        for ii in &mut self.display { *ii = 0; }

        self.should_draw = true;
    }

    // instruction(00EE): return from subroutine
    pub fn ret(&mut self) {
        if self.debug_enabled {
            println!("ret");
        }

        self.sp -= 1;
        self.pc = self.stack[self.sp as usize] as usize;
        self.pc += 2;
    }

    // instruction(1xxx): jmp to xxx
    pub fn jmp(&mut self, address: i32) {
        if self.debug_enabled {
            println!("jmp {}", address);
        }

        self.pc = address as usize;
    }

    // instruction(2xxx): call subroutine at xxx
    pub fn call(&mut self, address: i32) {
        if self.debug_enabled {
            println!("call {}", address);
        }

        self.stack[self.sp as usize] = self.pc as u16;
        self.sp += 1;
        self.pc = address as usize;
    }

    // instruction(3xyy): skip next instruction if register x equals value yy
    pub fn se_val(&mut self, register: u8, value: u8) {
        if self.debug_enabled {
            println!("se_val r{}, {}", register, value);
        }

        if self.registers[register as usize] == value {
            self.pc += 2;
        }

        self.pc += 2;
    }

    // instruction(4xyy): skip next instruction if register x does not equal value yy
    pub fn sne_val(&mut self, register: u8, value: u8) {
        if self.debug_enabled {
            println!("sne_val r{}, {}", register, value);
        }

        if self.registers[register as usize] != value {
            self.pc += 2;
        }

        self.pc += 2;
    }

    // instruction(5xy0): skip next instruction if register x equals register y
    pub fn se_reg(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("se_reg r{}, r{}", register_x, register_y);
        }

        if self.registers[register_x as usize] == self.registers[register_y as usize] {
            self.pc += 2;
        }

        self.pc += 2;
    }

    // instruction(6xyy): set register x to byte yy
    pub fn ld_reg(&mut self, register: u8, value: u8) {
        if self.debug_enabled {
            println!("ld_reg r{}, {}", register, value);
        }

        self.registers[register as usize] = value;
        self.pc += 2;
    }

    // instruction(7xyy): adds yy to register x
    pub fn add_val(&mut self, register: u8, value: u8) {
        if self.debug_enabled {
            println!("add_val r{}, {}", register, value);
        }

        let val = self.registers[register as usize];
        self.registers[register as usize] = val.wrapping_add(value);
        self.pc += 2;
    }

    // instruction(8xy0): copy value from register y to register x
    pub fn copy(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("copy r{}, r{}", register_x, register_y);
        }

        self.registers[register_x as usize] = self.registers[register_y as usize];
        self.pc += 2;
    }

    // instruction(8xy1): bitwise or on register x using register y, set register F to 0
    pub fn or(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("or r{}, r{}", register_x, register_y);
        }

        self.registers[register_x as usize] |= self.registers[register_y as usize];
        self.registers[0x0F] = 0;
        self.pc += 2;
    }

    // instruction(8xy2): bitwise and on register x using register y, set register F to 0
    pub fn and(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("and r{}, r{}", register_x, register_y);
        }

        self.registers[register_x as usize] &= self.registers[register_y as usize];
        self.registers[0x0F] = 0;
        self.pc += 2;
    }

    // instruction(8xy3): xor on register x using register y, set register F to 0
    pub fn xor(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("xor r{}, r{}", register_x, register_y);
        }

        self.registers[register_x as usize] ^= self.registers[register_y as usize];
        self.registers[0x0F] = 0;
        self.pc += 2;
    }

    // instruction(8xy4): adds register y to register x, set register F to 1 if operation wraps around, 0 if not
    pub fn add_reg(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("add_reg r{}, r{}", register_x, register_y);
        }

        if self.registers[register_x as usize] as i32 + self.registers[register_y as usize] as i32 > 0xFF {
            self.registers[0x0F] = 1;
        } else {
            self.registers[0x0F] = 0;
        }

        let val = self.registers[register_x as usize];
        self.registers[register_x as usize] = val.wrapping_add(self.registers[register_y as usize]);
        self.pc += 2;
    }

    // instruction(8xy5): subtracts register y from register x, set register F to 1 if operation wraps around, 0 if not
    pub fn sub_regxy(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("sub_regxy r{}, r{}", register_x, register_y);
        }

        if (self.registers[register_x as usize] as i32) - (self.registers[register_y as usize] as i32) < 0 {
            self.registers[0x0F] = 1;
        } else {
            self.registers[0x0F] = 0;
        }

        let val = self.registers[register_x as usize];
        self.registers[register_x as usize] = val.wrapping_sub(self.registers[register_y as usize]);
        self.pc += 2;
    }

    // instruction(8xy6): shift register right by 1, register F is set to the lsb of register before shifting 
    pub fn shift_r(&mut self, register: u8) {
        if self.debug_enabled {
            println!("shift_r r{}", register);
        }

        self.registers[0x0F] = self.registers[register as usize] & 0x1;
        self.registers[register as usize] >>= 1;
        self.pc += 2;
    }

    // instruction(8xy7): sets register x to register y minus register x, set register F to 1 if operation wraps around, 0 if not
    pub fn sub_regyx(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("sub_regyx r{}, r{}", register_x, register_y);
        }

        if (self.registers[register_y as usize] as i32) - (self.registers[register_x as usize] as i32) < 0 {
            self.registers[0x0F] = 1;
        } else {
            self.registers[0x0F] = 0;
        }

        let val = self.registers[register_y as usize];
        self.registers[register_x as usize] = val.wrapping_sub(self.registers[register_x as usize]);
        self.pc += 2;
    }

    // instruction(8xyE): shift register left by 1, register F is set to the msb of register before shifting 
    pub fn shift_l(&mut self, register: u8) {
        if self.debug_enabled {
            println!("shift_l r{}", register);
        }

        self.registers[0x0F] = self.registers[register as usize] >> 7;
        self.registers[register as usize] <<= 1;
        self.pc += 2;
    }

    // instruction(9xy0): skip next instruction if register x does not equal register y
    pub fn sne_reg(&mut self, register_x: u8, register_y: u8) {
        if self.debug_enabled {
            println!("sne_reg r{}, r{}", register_x, register_y);
        }

        if self.registers[register_x as usize] != self.registers[register_y as usize] {
            self.pc += 2;
        }

        self.pc += 2;
    }

    // instruction(Axxx): set index to xxx
    pub fn ld_indx(&mut self, value: i32) {
        if self.debug_enabled {
            println!("ld_indx {}", value);
        }

        self.index = value as u16;
        self.pc += 2;
    }

    // instruction(Bxxx): jump to address xxx plus value of register 0
    pub fn jmpadd(&mut self, address: i32) {
        if self.debug_enabled {
            println!("jmpadd {}", address);
        }
        
        self.pc = address as usize;
        self.pc += self.registers[0 as usize] as usize;
    }

    // instruction(Cxyy): performs and operation on random byte and value yy, stores it into register x
    pub fn rand_and(&mut self, register: u8, value: u8) {
        if self.debug_enabled {
            println!("rand_and r{}, {}", register, value);
        }

        self.registers[register as usize] = self.rng.gen_range(0..255) & value;                
        self.pc += 2;
    }

    // instruction(Dxyz): set pixel at x/y coord to weight z
    pub fn draw_pixel(&mut self, register_x: u8, register_y: u8, weight: i32) {
        if self.debug_enabled {
            println!("draw_pixel r{}, r{}, {}", register_x, register_y, weight);
        }

        let pixel_x = self.registers[register_x as usize];
        let pixel_y = self.registers[register_y as usize];
        let wt = 8;

        for ii in 0..weight {
            let pixel = self.memory[(self.index as i32 + ii) as usize];

            for j in 0..wt {
                if (pixel & (0x80 >> j)) != 0 {
                    let indx = ((pixel_x as i32 + j) + ((pixel_y as i32 + ii) * 64) % 2048) as usize;
                    if self.display[indx] == 1 {
                        self.registers[0x0F] = 1;
                    }

                    self.display[indx] ^= 1;
                }
            }
        }

        self.registers[0x0F as usize] = 0;

        self.should_draw = true;
        self.pc += 2;
    }

    // instruction(Fx07): sets register x to value of delay timer
    pub fn get_delay(&mut self, register: u8) {
        if self.debug_enabled {
            println!("get_delay r{}", register);
        }

        self.registers[register as usize] = self.delay_timer;
        self.pc += 2;
    }

    // instruction(Fx0A): awaits key press and stores it into register x
    pub fn wait_key(&mut self, register: u8) {
        if self.debug_enabled {
            println!("wait_key r{}", register);
        }
        
        let mut key_pressed = false;
        for ii in 0..self.kp_input.len() {
            if self.kp_input[ii] != 0 {
                key_pressed = true;
                self.registers[register as usize] = ii as u8;
            }
        }

        if key_pressed {
            self.pc += 2;
        }
    }

    // instruction(Fx15): sets delay timer to value of register x 
    pub fn set_delay(&mut self, register: u8) {
        if self.debug_enabled {
            println!("set_delay r{}", register);
        }

        self.delay_timer = self.registers[register as usize];
        self.pc += 2;
    }

    // instruction(Fx18): sets sound timer to value of register x 
    pub fn set_sound(&mut self, register: u8) {
        if self.debug_enabled {
            println!("set_sound r{}", register);
        }

        self.sound_timer = self.registers[register as usize];
        self.pc += 2;
    }

    // instruction(Fx1E): adds register x to index, set register F to 1 if operation wraps around, 0 if not
    pub fn add_indx(&mut self, register: u8) {
        if self.debug_enabled {
            println!("add_indx r{}", register);
        }

        if self.registers[register as usize] as i32 + self.index as i32 > 0xFFF {
            self.registers[0x0F] = 1;
        } else {
            self.registers[0x0F] = 0;
        }

        self.index = self.index.wrapping_add(self.registers[register as usize] as u16);
        self.pc += 2;
    }

    // instruction(Fx33): saves most significant bits of register into memory at index
    pub fn encode_save(&mut self, register: u8) {
        if self.debug_enabled {
            println!("encode_save r{}", register);
        }

        let value = self.registers[register as usize];
        self.memory[(self.index as usize)] = (value / 100) as u8;
        self.memory[((self.index + 1) as usize)] = ((value / 10) % 10) as u8;
        self.memory[((self.index + 2) as usize)] = ((value % 100) % 10) as u8;
        self.pc += 2;
    }

    // instruction(Fx55): save register 0 until register x to memory starting at index
    pub fn save(&mut self, register: u8) {
        if self.debug_enabled {
            println!("save r{}", register);
        }

        for ii in 0..register + 1 {
            self.memory[(self.index + ii as u16) as usize] = self.registers[ii as usize];
        }

        self.index = self.index.wrapping_add((register + 1) as u16);
        self.pc += 2;
    }

    // instruction(Fx65): load register 0 until register x to memory starting at index
    pub fn load(&mut self, register: u8) {
        if self.debug_enabled {
            println!("load r{}", register);
        }

        for ii in 0..register + 1 {
            self.registers[ii as usize] = self.memory[(self.index + ii as u16) as usize];
        }

        self.index = self.index.wrapping_add((register + 1) as u16);
        self.pc += 2;
    }
}

// bit shifting stuff
pub fn shift_u8(value: i32, bits: i32, binary_and: i32) -> u8 {
    ((value & binary_and) >> bits) as u8
}

pub fn shift_i32(value: i32, bits: i32, binary_and: i32) -> i32 {
    (value & binary_and) >> bits
}
