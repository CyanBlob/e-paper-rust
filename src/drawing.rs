#![allow(unused_imports)]
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use epd_waveshare::graphics::TriDisplayCompact;
use epd_waveshare::{
    color::*,
    epd2in9_v2::{Display2in9, Epd2in9},
    epd7in5_v3::{Display7in5, Epd7in5},
    graphics::DisplayRotation,
    prelude::*,
};

use esp_idf_hal::spi;
use esp_idf_hal::spi::config::Config;
use esp_idf_hal::spi::*;

use esp_idf_hal::delay;
use esp_idf_hal::gpio;
use esp_idf_hal::i2c;
use esp_idf_hal::prelude::*;

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::*;


use std::sync::{Condvar, Mutex};
use core::ffi::c_void;

use crate::marvin_api::Task;

const TASK_SPACING: i16 = 30;

pub fn draw_text(display: &mut Display7in5, text: &str, x: i16, y: i16) {
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
        .text_color(TriColor::White)
        //.background_color(TriColor::Chromatic)
        .build();

    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    let _ = Text::with_text_style(text, Point::new(x.into(), y.into()), style, text_style)
        .draw(display);

    println!("Writing: {}", text);
}

#[no_mangle]
pub extern "C" fn start_draw(tasks_ptr: *mut c_void) {
    let tasks_box: Box<Mutex<Vec<Task>>> =
        unsafe { Box::from_raw(tasks_ptr as *mut Mutex<Vec<Task>>) };

    let peripherals = get_peripherals().unwrap();
    let pins = peripherals.pins;

    unsafe {
        esp_idf_sys::vTaskDelay(1);
    }

    println!("SPI Config");
    let config = <spi::config::Config as Default>::default().baudrate((4).MHz().into());

    {
        println!("SPI init");
        let mut spi = spi::Master::<spi::SPI2, _, _, _, _>::new(
            peripherals.spi2,
            spi::Pins {
                sclk: pins.gpio18,
                sdo: pins.gpio23,
                sdi: Option::<gpio::Gpio21<gpio::Unknown>>::None,
                cs: Some(pins.gpio15),
            },
            config,
        )
        .unwrap();

        println!("Pin defs");
        let cs = pins.gpio17.into_output().unwrap();
        let busy = pins.gpio0.into_input().unwrap();
        let dc = pins.gpio16.into_output().unwrap();
        let rst = pins.gpio4.into_output().unwrap();

        #[allow(unused)]
        let mut u8_delay = U8Delay { delay: delay::Ets };

        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }

        println!("Display init");

        println!("Create epd");
        let epd = Epd7in5::new(&mut spi, cs, busy, dc, rst, &mut u8_delay);
        println!("Created epd");

        match epd {
            Ok(_) => {
                println!("Got epd");
            }
            Err(_) => {
                print!("Failed to get epd :(");
                return;
            }
        }
        let mut epd = epd.unwrap();

        let mut display = Display7in5::default();

        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }
        print_memory();

        do_draw(&mut epd, &mut display, &mut spi, tasks_box, u8_delay);
}
}

pub fn do_draw<A, B, C, D, E, F, G, H, I>(
    epd: &mut Epd7in5<spi::Master<spi::SPI2, F, G, H, I>, A, B, C, D, E>,
    display: &mut Display7in5,
    spi: &mut spi::Master<spi::SPI2, F, G, H, I>,
    tasks_box: Box<Mutex<Vec<Task>>>,
    _u8_delay: E,
) where
    A: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
    B: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::InputPin,
    C: esp_idf_hal::gpio::OutputPin
        + esp_idf_hal::gpio::InputPin
        + embedded_hal::digital::v2::OutputPin,
    D: esp_idf_hal::gpio::OutputPin + embedded_hal::digital::v2::OutputPin,
    E: DelayMs<u8>,
    F: esp_idf_hal::gpio::OutputPin,
    G: esp_idf_hal::gpio::OutputPin,
    H: esp_idf_hal::gpio::OutputPin + esp_idf_hal::gpio::InputPin,
    I: esp_idf_hal::gpio::OutputPin,
{
    display.set_rotation(DisplayRotation::Rotate270);

    loop {
        loop {
            {
                let tasks = tasks_box.lock().unwrap();
                println!(
                    "Draw task sees the following tasks ({} total): ",
                    tasks.len()
                );

                if tasks.len() > 0 {
                    println!("White clear");
                    display.clear_bw_buffer(TriColor::White);
                    epd.update_achromatic_frame(spi, display.bw_buffer())
                        .unwrap();
                }

                for (i, task) in tasks.iter().enumerate() {
                    println!("{}", &task.title.as_ref().unwrap());
                    draw_text(
                        display,
                        &task.title.as_ref().unwrap(),
                        0,
                        i as i16 * TASK_SPACING,
                    );
                }
            }
        }

        #[allow(unreachable_code)]
        {
            unsafe {
                esp_idf_sys::vTaskDelay(100);
            }

            println!("White clear");
            display.clear_bw_buffer(TriColor::White);
            // manual buffer update for testing
            /*display.get_mut_buffer();
            for elem in display.get_mut_buffer().iter_mut() {
                *elem = 0xFF;
            }
            println!("Updated {} bytes", display.buffer().len());*/
            /*let mut i = 0;
            for elem in display.get_mut_buffer().iter_mut() {
                match i {
                    i if i < 24000              => *elem = 0xFF,
                    i if i < 48000              => *elem = 0x00,
                    i if i > 48000 && i < 60000 => *elem = 0xFF,
                    i if i > 72000 && i < 84000 => *elem = 0xFF,
                    _                           => *elem = 0x00
                }
                i = i + 1;
            }*/
            //epd.update_color_frame(&mut spi, display.bw_buffer(), display.chromatic_buffer());
            epd.update_achromatic_frame(&mut spi, display.bw_buffer())
                .unwrap();

            display.clear_chromatic_buffer(TriColor::White);
            epd.update_chromatic_frame(&mut spi, display.bw_buffer())
                .unwrap();

            epd.display_frame(&mut spi, &mut _u8_delay).unwrap();

            unsafe {
                esp_idf_sys::vTaskDelay(1500);
            }

            println!("Black clear");
            display.clear_bw_buffer(TriColor::Black);
            display.get_mut_buffer();

            // r/w frame already empty
            epd.update_achromatic_frame(&mut spi, display.bw_buffer())
                .unwrap();
            epd.display_frame(&mut spi, &mut _u8_delay).unwrap();

            unsafe {
                esp_idf_sys::vTaskDelay(1000);
            }

            println!("Red clear");
            // set b/w frame to white. NOTE: not needed; red is highest priority
            //display.clear_buffer(TriColor::White);
            //epd.update_achromatic_frame(&mut spi, display.bw_buffer());
            /*let mut i = 0;
            for elem in display.get_mut_buffer().iter_mut() {
                match i {
                    i if i < 48000 => *elem = 0x00,
                    _              => *elem = 0xFF
                }
                i = i + 1;
            }*/
            display.clear_chromatic_buffer(TriColor::Chromatic);
            epd.update_chromatic_frame(&mut spi, display.chromatic_buffer())
                .unwrap();
            epd.display_frame(&mut spi, &mut _u8_delay).unwrap();

            unsafe {
                esp_idf_sys::vTaskDelay(1000);
            }

            draw_text(&mut display, "Hello, world", 00, 20);
            draw_text(&mut display, "from Rust running on", 0, 40);
            draw_text(
                &mut display,
                "my ESP32 connected to a 7in5 V3 WaveShare display",
                0,
                60,
            );
            draw_text(
                &mut display,
                "This is mostly working, but the colors are wrong",
                0,
                80,
            );

            unsafe {
                esp_idf_sys::vTaskDelay(1);
            }

            // Transfer the frame data to the epd and display it
            epd.update_achromatic_frame(&mut spi, &display.bw_buffer())
                .unwrap();
            match epd.display_frame(&mut spi, &mut _u8_delay) {
                Ok(_) => println!("Update frame ok"),
                Err(_) => println!("Update frame fail"),
            }
            println!("Tried to display");

            //loop {
            unsafe {
                esp_idf_sys::vTaskDelay(1000);
            }
            //}
        }
    }
}

pub struct U8Delay {
    #[allow(dead_code)]
    delay: delay::Ets,
}

#[allow(unused)]
fn print_memory() {
    unsafe {
        println!(
            "HEAP INTERNAL: {}",
            esp_idf_sys::esp_get_free_internal_heap_size()
        );
        println!("HEAP REMAINING: {}", esp_idf_sys::esp_get_free_heap_size());
        println!(
            "TASK HIGH WATER MARK: {}",
            esp_idf_sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut())
        );
    }
}

fn get_peripherals() -> Option<Peripherals> {
    {
        let mut _peripherals = Peripherals::take();

        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }

        match _peripherals {
            Some(p) => {
                println!("Got peripherals");
                Some(p)
            }
            None => {
                print!("Failed to get peripherals :(");
                None
            }
        }
    }
}

