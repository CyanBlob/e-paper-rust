#![allow(clippy::single_component_path_imports)]

use core::ffi::c_void;

use std::{cell::RefCell, env, sync::Mutex};

use esp_idf_hal::delay;

use esp_idf_sys::EspError;
use esp_idf_sys::{self};

use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayUs;

pub mod drawing;
pub mod marvin_api;
pub mod wifi;

use drawing::U8Delay;

#[cfg(esp32s2)]
include!(env!("EMBUILD_GENERATED_SYMBOLS_FILE"));

#[cfg(esp32s2)]
const ULP: &[u8] = include_bytes!(env!("EMBUILD_GENERATED_BIN_FILE"));

thread_local! {
    static TLS: RefCell<u32> = RefCell::new(13);
}

fn main() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Get backtraces from anyhow; only works for Xtensa arch currently
    #[cfg(target_arch = "xtensa")]
    env::set_var("RUST_BACKTRACE", "1");
    println!("Booted!");

    let mut delay = delay::Ets;

    delay.delay_us(200 as u16);

    let tasks_box = Box::new(Mutex::new(Vec::<marvin_api::Task>::new()));

    let tasks_ptr: *mut c_void = Box::into_raw(tasks_box) as *mut _;

    let name_task2 = String::from("Wifi");
    let mut idle_handle = 8;
    let idle_task: *mut esp_idf_sys::TaskHandle_t =
        &mut idle_handle as *mut _ as *mut esp_idf_sys::TaskHandle_t;

    // this task runs on core 1 and starts a wifi access point
    unsafe {
        esp_idf_sys::xTaskCreatePinnedToCore(
            Some(wifi::start_wifi),
            &(String::as_bytes(&name_task2).as_ptr() as i8),
            80000,
            tasks_ptr,
            0,
            idle_task,
            1,
        );
    }

    let name_task1 = String::from("Display");

    let mut test_handle = 0;

    let created_task: *mut esp_idf_sys::TaskHandle_t =
        &mut test_handle as *mut _ as *mut esp_idf_sys::TaskHandle_t;

    // this task runs on core 0 and writes to the display
    unsafe {
        esp_idf_sys::xTaskCreatePinnedToCore(
            Some(drawing::start_draw),
            &(String::as_bytes(&name_task1).as_ptr() as i8),
            60000,
            tasks_ptr,
            0,
            created_task,
            0,
        );
    }

    loop {
        unsafe {
            esp_idf_sys::vTaskDelay(100);
            //esp_idf_sys::vTaskDelete(ptr::null_mut());
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}

impl embedded_hal::blocking::delay::DelayMs<u8> for U8Delay {
    fn delay_ms(&mut self, ms: u8) {
        let mut delay = delay::Ets;
        delay.delay_us(ms as u32 * 10);
    }
}

#[allow(unused)]
pub fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}
