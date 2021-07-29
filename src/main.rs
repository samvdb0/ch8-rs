use anyhow::{Result, bail};
use std::env;
use std::ffi::c_void;
use std::path::Path;
use std::ptr::null;

extern crate sdl2;
use sdl2::pixels::{PixelFormatEnum};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::sys::{SDL_UpdateTexture};

use std::time::Duration;

mod ch8;
use ch8::Chip8;
use ch8::{VIDEO_HEIGHT, VIDEO_WIDTH};

mod tickrate;
use tickrate::Tickrate;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut rom = "";
    let mut is_debug: bool = false;
    let mut is_step_mode: bool = false;
    let mut tr = Tickrate::new();

    let mut iter = args.iter().skip(1);
    while let Some(ii) = iter.next() {        
        if ii.eq("--debug") {
            is_debug = true;
        }

        if ii.eq("--step") {
            is_step_mode = true;
        }

        if !ii.starts_with("--") {
            rom = ii;
        }
    }

    if rom == "" {
        bail!("usage: ./ch8-rs [optional: --debug] <path_to_rom_file>")
    }

    let mut ch8 = Chip8::new(is_debug);
    match ch8.read_rom(rom) {
        Err(s) => bail!(s), // early exit if read fails
        Ok(()) => { }
    }

    let sdl_ctx= sdl2::init().unwrap();
    let video = sdl_ctx.video().unwrap();
    let filename = String::from(Path::new(rom).file_stem().unwrap().to_str().unwrap());

    let window = video.window(std::format!("ch8-rs - playing: {}", filename).as_str(), VIDEO_WIDTH as u32 * 15, VIDEO_HEIGHT as u32 * 15).position_centered().build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    canvas.clear();
    canvas.present();

    let texture_creator = canvas.texture_creator();
    let output_texture = texture_creator.create_texture_streaming(Some(PixelFormatEnum::ARGB8888), 64, 32).unwrap();

    let mut advance = false;
    let mut events = sdl_ctx.event_pump().unwrap();
    'running: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), ..} => break 'running,
                Event::KeyDown { keycode: Some(Keycode::Return), .. } => advance = true,
                Event::KeyDown { keycode: Some(Keycode::F1), .. } => println!("{}", ch8.dump_registers()),
                // pong
                // Event::KeyDown { keycode: Some(Keycode::Z), .. } => ch8.set_input(1, true),
                // Event::KeyUp { keycode: Some(Keycode::Z), .. } => ch8.set_input(1, false),
                // Event::KeyDown { keycode: Some(Keycode::S), .. } => ch8.set_input(4, true),
                // Event::KeyUp { keycode: Some(Keycode::S), .. } => ch8.set_input(4, false),
                // Event::KeyDown { keycode: Some(Keycode::R), .. } => ch8.set_input(12, true),
                // Event::KeyUp { keycode: Some(Keycode::R), .. } => ch8.set_input(12, false),
                // Event::KeyDown { keycode: Some(Keycode::F), .. } => ch8.set_input(13, true),
                // Event::KeyUp { keycode: Some(Keycode::F), .. } => ch8.set_input(13, false),

                // space invaders
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => ch8.set_input(5, true),
                Event::KeyUp { keycode: Some(Keycode::Space), .. } => ch8.set_input(5, false),
                Event::KeyDown { keycode: Some(Keycode::Q), .. } => ch8.set_input(4, true),
                Event::KeyUp { keycode: Some(Keycode::Q), .. } => ch8.set_input(4, false),
                Event::KeyDown { keycode: Some(Keycode::D), .. } => ch8.set_input(6, true),
                Event::KeyUp { keycode: Some(Keycode::D), .. } => ch8.set_input(6, false),
                _ => { }
            }
        }

        if is_step_mode && !advance {
            continue;
        } 

        ch8.cycle();

        if ch8.should_draw() {
            ch8.set_should_draw(false);
            let mut r: Vec<u32> = vec![0; 64 * 32];

            for ii in 0..64 * 32 {
                if ch8.get_display(ii as usize) == 0 {
                    r[ii] = 0xFF000000;
                } else {
                    r[ii] = 0xFFFFFFFF;
                }
            }

            // todo(safe): figure out what texture::update() _actually_ does
            unsafe { 
                let op_raw = output_texture.raw();
                let rawc = r.as_ptr();
                SDL_UpdateTexture(op_raw, null(), rawc as *const c_void, 64 * 4); 
            }

            canvas.clear();
            canvas.copy(&output_texture, None, None).unwrap();
            canvas.present();
        }

        canvas.window_mut().set_title(std::format!("ch8-rs - running {} | fps: {}", filename, tr.tick()).as_str())?;
        advance = false;
        ::std::thread::sleep(Duration::from_micros(1500));
    }

    Ok(())
}
