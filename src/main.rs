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
    epd7in5_v3::{Display7in5, Epd7in5},
    graphics::DisplayRotation,
    prelude::*,
};

use anyhow::bail;
use embedded_svc::mqtt::client::utils::ConnState;
use log::*;

use embedded_hal::adc::OneShot;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;
use embedded_svc::eth;
use embedded_svc::eth::{Eth, TransitionalState};
use embedded_svc::httpd::registry::*;
use embedded_svc::httpd::*;
use embedded_svc::io;
use embedded_svc::ipv4;
use embedded_svc::mqtt::client::{Client, Connection, MessageImpl, Publish, QoS};
use embedded_svc::ping::Ping;
use embedded_svc::sys_time::SystemTime;
use embedded_svc::timer::TimerService;
use embedded_svc::timer::*;
use embedded_svc::wifi::*;

use esp_idf_hal::delay;
use esp_idf_hal::gpio;
use esp_idf_hal::i2c;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi;
use esp_idf_hal::spi::config::Config;
use esp_idf_hal::spi::*;

use esp_idf_svc::eth::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::httpd as idf;
use esp_idf_svc::httpd::ServerRegistry;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::nvs::*;
use esp_idf_svc::ping;
use esp_idf_svc::sntp;
use esp_idf_svc::sysloop::*;
use esp_idf_svc::systime::EspSystemTime;
use esp_idf_svc::timer::*;
use esp_idf_svc::wifi::*;

use esp_idf_hal::adc;
use esp_idf_hal::prelude::*;
use esp_idf_sys::{self, c_types};
use esp_idf_sys::{esp, EspError};
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

    delay.delay_us(200 as u16);

    let name_task1 = String::from("Display");

    let mut test_int = 7;
    let mut test_handle = 0;

    let test: *mut c_void = &mut test_int as *mut _ as *mut c_void;
    let created_task: *mut esp_idf_sys::TaskHandle_t =
        &mut test_handle as *mut _ as *mut esp_idf_sys::TaskHandle_t;

    // this task runs on core 0 and writes to the display
    unsafe {
        esp_idf_sys::xTaskCreatePinnedToCore(
            Some(test_draw),
            &(String::as_bytes(&name_task1).as_ptr() as i8),
            100000,
            test,
            0,
            created_task,
            0,
        );
    }

    let name_task2 = String::from("Wifi");
    let mut idle_int = 9;
    let mut idle_handle = 8;
    let test_idle: *mut c_void = &mut idle_int as *mut _ as *mut c_void;
    let idle_task: *mut esp_idf_sys::TaskHandle_t =
        &mut idle_handle as *mut _ as *mut esp_idf_sys::TaskHandle_t;

    // this task runs on core 1 and starts a wifi access point
    unsafe {
        esp_idf_sys::xTaskCreatePinnedToCore(
            Some(idle),
            &(String::as_bytes(&name_task2).as_ptr() as i8),
            10000,
            test_idle,
            0,
            idle_task,
            1,
        );
    }
    
    // "main" runs on core 1 by default. Delete the default task since we run our own task on it
    unsafe {
        esp_idf_sys::vTaskDelete(ptr::null_mut());
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn start_wifi(_test: *mut c_void) {
    println!("IDLE RESET BEGIN");
    // network stuff
    #[allow(unused)]
    let netif_stack = Arc::new(EspNetifStack::new().unwrap());
    #[allow(unused)]
    let sys_loop_stack = Arc::new(EspSysLoopStack::new().unwrap());
    #[allow(unused)]
    let default_nvs = Arc::new(EspDefaultNvs::new().unwrap());
    
    let netif_stack_arc = netif_stack.clone();
    let sys_loop_stack_arc = sys_loop_stack.clone();
    let default_nvs_arc = default_nvs.clone();
    
    #[allow(clippy::redundant_clone)]
    #[cfg(not(feature = "qemu"))]
    #[allow(unused_mut)]
    let mut wifi = wifi(
        netif_stack_arc,
        sys_loop_stack_arc,
        default_nvs_arc,
    );
    loop {
        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }
    }
}

fn draw_text(display: &mut Display7in5, text: &str, x: i32, y: i32) {
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
        .text_color(TriColor::White)
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
            println!("DISPLAY HEAP INTERNAL: {}", esp_get_free_internal_heap_size());
            println!("DISPLAY HEAP REMAINING: {}", esp_get_free_heap_size());
            println!("DISPLAY TASK STACK: {}", esp_idf_sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()));
        }

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

        let mut u8_delay = U8Delay { delay: delay::Ets };

        let mut display = Display7in5::default();

        unsafe {
            esp_idf_sys::vTaskDelay(1);
        }

        println!("White clear");
        display.clear_buffer(TriColor::White);
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
        epd.update_color_frame(&mut spi, display.bw_buffer(), display.chromatic_buffer());
        epd.display_frame(&mut spi, &mut u8_delay);

        unsafe {
            esp_idf_sys::vTaskDelay(1500);
        }

        println!("Black clear");
        display.clear_buffer(TriColor::Black);
        display.get_mut_buffer();

        epd.update_color_frame(&mut spi, display.bw_buffer(), display.chromatic_buffer());
        epd.display_frame(&mut spi, &mut u8_delay);

        unsafe {
            esp_idf_sys::vTaskDelay(1000);
        }

        println!("Red clear");
        display.clear_buffer(TriColor::Chromatic);
        /*let mut i = 0;
        for elem in display.get_mut_buffer().iter_mut() {
            match i {
                i if i < 48000 => *elem = 0x00,
                _              => *elem = 0xFF
            }
            i = i + 1;
        }*/
        epd.update_color_frame(&mut spi, display.bw_buffer(), display.chromatic_buffer());
        epd.display_frame(&mut spi, &mut u8_delay);

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
        epd.update_color_frame(&mut spi, &display.bw_buffer(), &display.chromatic_buffer());
        match epd.display_frame(&mut spi, &mut u8_delay) {
            Ok(_) => println!("Update frame ok"),
            Err(_) => println!("Update frame fail"),
        }
        println!("Tried to display");
        unsafe {
            esp_idf_sys::vTaskDelay(10000);
        }
    }
}

struct U8Delay {
    delay: delay::Ets,
}

impl embedded_hal::blocking::delay::DelayMs<u8> for U8Delay {
    fn delay_ms(&mut self, ms: u8) {
        let mut delay = delay::Ets;
        delay.delay_us(ms as u32 * 10);
    }
}

#[cfg(not(feature = "qemu"))]
#[allow(dead_code)]
fn wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    println!("Got status");

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        info!("Wifi connected");

        ping(&ip_settings)?;
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }

    Ok(wifi)
}

fn ping(ip_settings: &ipv4::ClientSettings) -> Result<()> {
    info!("About to do some pings for {:?}", ip_settings);

    let ping_summary =
        ping::EspPing::default().ping(ip_settings.subnet.gateway, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        bail!(
            "Pinging gateway {} resulted in timeouts",
            ip_settings.subnet.gateway
        );
    }

    info!("Pinging done");

    Ok(())
}