use anyhow::{Context, Result, bail};
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

mod tickrate;
use tickrate::Tickrate;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let rom = args.get(1).with_context(|| "usage: ch8-rs <rom_file>")?;
    let mut is_debug: bool = false;
    let mut tr = Tickrate::new();
    
    for ii in &args {
        if ii.eq("--debug") {
            is_debug = true;
        }
    }

    let mut ch8 = Chip8::new(is_debug);

    match ch8.read_rom(rom) {
        Err(s) => bail!(s), // early exit if read fails
        Ok(()) => { }
    }

    let sdl_ctx= sdl2::init().unwrap();
    let video = sdl_ctx.video().unwrap();
    let filename = String::from(Path::new(rom.as_str()).file_stem().unwrap().to_str().unwrap());

    let window = video.window(std::format!("ch8-rs - playing: {}", filename).as_str(), 800, 600).position_centered().build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    canvas.clear();
    canvas.present();

    let texture_creator = canvas.texture_creator();
    let output_texture = texture_creator.create_texture_streaming(Some(PixelFormatEnum::ARGB8888), 64, 32).unwrap();

    let mut events = sdl_ctx.event_pump().unwrap();
    'running: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'running;
                },
                Event::KeyDown { keycode: Some(Keycode::Escape), ..} => {
                    break 'running;
                }
                _ => { }
            }
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
        ::std::thread::sleep(Duration::from_micros(1500));
    }

    Ok(())
}
