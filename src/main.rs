#![allow(unused_imports)]
#![allow(clippy::single_component_path_imports)]

use core::ffi::c_void;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Condvar, Mutex};
use std::{cell::RefCell, env, ptr, sync::atomic::*, sync::Arc, thread, time::*};

use embedded_graphics::mono_font::MonoTextStyleBuilder;
use epd_waveshare::{
    color::*,
    epd2in9_v2::{Display2in9, Epd2in9},
    epd7in5_v2::{Display7in5, Epd7in5},
    graphics::DisplayRotation,
    prelude::*,
};

use anyhow::bail;
use log::*;

use embedded_svc::eth;
use embedded_svc::eth::Eth;
use embedded_svc::httpd::registry::*;
use embedded_svc::httpd::*;
use embedded_svc::io;
use embedded_svc::ipv4;
use embedded_svc::ping::Ping;
use embedded_svc::wifi::*;

use esp_idf_hal::delay;
use esp_idf_hal::gpio;
use esp_idf_hal::i2c;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi;
use esp_idf_hal::spi::config::Config;
use esp_idf_hal::spi::*;

use esp_idf_sys;
use esp_idf_sys::esp;

use display_interface_spi::SPIInterfaceNoCS;

use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::*;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::text::*;

use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayUs;
use embedded_hal::prelude::*;
use embedded_hal::spi::Mode;
use embedded_hal::spi::*;

use ssd1306;
use ssd1306::mode::DisplayConfig;
use st7789;

#[allow(dead_code)]
#[cfg(not(feature = "qemu"))]
const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
#[allow(dead_code)]
#[cfg(not(feature = "qemu"))]
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

#[cfg(esp32s2)]
include!(env!("EMBUILD_GENERATED_SYMBOLS_FILE"));

#[cfg(esp32s2)]
const ULP: &[u8] = include_bytes!(env!("EMBUILD_GENERATED_BIN_FILE"));

thread_local! {
    static TLS: RefCell<u32> = RefCell::new(13);
}

fn main() -> Result<()> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Get backtraces from anyhow; only works for Xtensa arch currently
    #[cfg(target_arch = "xtensa")]
    env::set_var("RUST_BACKTRACE", "1");
    println!("Booted!");

    let mut delay = delay::Ets;

    delay.delay_us(2000 as u16);
    println!("Test draw!");
    println!("Test draw!");
    println!("Test draw!");
    println!("Test draw!");
    //test_draw(ptr::null_mut());

    let name = String::from("Blink");

    let mut test_int = 7;
    let mut test_handle = 0;

    let test: *mut c_void = &mut test_int as *mut _ as *mut c_void;
    let created_task: *mut esp_idf_sys::TaskHandle_t =
        &mut test_handle as *mut _ as *mut esp_idf_sys::TaskHandle_t;

    unsafe {
        esp_idf_sys::xTaskCreatePinnedToCore(
            Some(test_draw),
            &(String::as_bytes(&name).as_ptr() as i8),
            100000,
            test,
            0,
            created_task,
            0,
        );
    }

    let name_idle = String::from("NotI");
    let mut idle_int = 9;
    let mut idle_handle = 8;
    let test_idle: *mut c_void = &mut idle_int as *mut _ as *mut c_void;
    let idle_task: *mut esp_idf_sys::TaskHandle_t =
        &mut idle_handle as *mut _ as *mut esp_idf_sys::TaskHandle_t;

    unsafe {
        esp_idf_sys::xTaskCreatePinnedToCore(
            Some(idle),
            &(String::as_bytes(&name).as_ptr() as i8),
            10000,
            test_idle,
            0,
            idle_task,
            1,
        );
    }

    unsafe {
        esp_idf_sys::vTaskDelete(ptr::null_mut());
    }

    delay.delay_us(200 as u16);

    loop {
        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn idle(_test: *mut c_void) {
    println!("IDLE RESET BEGIN");
    loop {
        unsafe {
            unsafe {
                esp_idf_sys::vTaskDelay(1);
            }
            thread::sleep(Duration::from_secs(1))
        }
    }
}

fn draw_text(display: &mut Display7in5, text: &str, x: i32, y: i32) {
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
        .text_color(TriColor::Black)
        .background_color(TriColor::Chromatic)
        .build();

    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    let _ = Text::with_text_style(text, Point::new(x, y), style, text_style).draw(display);

    println!("Writing: {}", text);
}

#[no_mangle]
pub extern "C" fn test_draw(_test: *mut c_void) {
    let mut delay = delay::Ets;

    let _peripherals = Peripherals::take();
    let peripherals;

    match _peripherals {
        Some(p) => {
            println!("Got peripherals");
            peripherals = p;
        }
        None => {
            print!("Failed to get peripherals :(");
            delay.delay_us(20000 as u32);
            return;
        }
    }
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

        let mut u8_delay = U8Delay { delay: delay::Ets };

        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }

        println!("Display init");

        unsafe {
            //println!("EDP TEST!!!\nHEAP INTERNAL: {}", esp_get_free_internal_heap_size());
            //println!("HEAP REMAINING: {}", esp_get_free_heap_size());
            //println!("TASK STACK: {}", esp_idf_sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()));
        }

        println!("Create epd");
        let mut epd = Epd7in5::new(&mut spi, cs, busy, dc, rst, &mut u8_delay);
        println!("Created epd?");

        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }

        match epd {
            Ok(_) => {
                println!("Got epd");
            }
            Err(e) => {
                print!("Failed to get epd :(");
                return;
            }
        }
        let mut epd = epd.unwrap();

        let mut u8_delay = U8Delay { delay: delay::Ets };

        let mut display = Display7in5::default();

        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }

        println!("White clear");
        display.clear_buffer(TriColor::Black);
        display.clear_buffer(TriColor::White);
        display.clear_buffer(TriColor::Chromatic);
        epd.update_color_frame(&mut spi, display.bw_buffer(), display.chromatic_buffer());
        epd.display_frame(&mut spi, &mut u8_delay);

        unsafe {
            esp_idf_sys::vTaskDelay(3000);
        }

        draw_text(&mut display, "Hello, world", 00, 20);
        draw_text(&mut display, "from Rust running on", 0, 40);
        draw_text(&mut display, "my ESP32 connected to a 7in5 V3 WaveShare display", 0, 60);
        draw_text(&mut display, "This is mostly working, but the colors are wrong", 0, 80);

        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }

        // Transfer the frame data to the epd and display it
        epd.update_color_frame(&mut spi, &display.bw_buffer(), &display.chromatic_buffer());
        match epd.display_frame(&mut spi, &mut u8_delay){
            Ok(_) => println!("Update frame ok"),
            Err(_) => println!("Update frame fail"),
        }
        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }
        /*match epd.display_frame(&mut spi, &mut u8_delay) {
            Ok(_) => println!("Display frame ok"),
            Err(_) => println!("Display frame fail"),
        }*/
        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }
        /*match epd.update_and_display_frame(&mut spi, &display.buffer(), &mut u8_delay) {
            Ok(_) => println!("Update and display frame ok"),
            Err(_) => println!("Update and display frame fail"),
        }*/
        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }
        println!("Tried to display");
        //thread::sleep(Duration::from_secs(1))
        unsafe {
            esp_idf_sys::vTaskDelay(10000);
        }

        /*display.clear_buffer(Color::Black);
        match epd.update_and_display_frame(&mut spi, &display.buffer(), &mut u8_delay) {
            Ok(_) => println!("Update and display frame ok"),
            Err(_) => println!("Update and display frame fail"),
        }*/
    }
}

struct U8Delay {
    delay: delay::Ets,
}

impl embedded_hal::blocking::delay::DelayMs<u8> for U8Delay {
    fn delay_ms(&mut self, ms: u8) {
        unsafe {
            //delay((ms as u32 * 1000));
            let mut delay = delay::Ets;
            delay.delay_us(ms as u32 * 10);
        }
    }
}
