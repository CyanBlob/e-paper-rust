#![allow(unused_imports)]
use core::ffi::c_void;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Condvar, Mutex};
use std::{cell::RefCell, env, ptr, sync::atomic::*, sync::Arc, thread, time::*};

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

use anyhow::bail;
use log::*;

use crate::marvin_api::get_categories;
use crate::marvin_api::get_todos_for_today;
use crate::marvin_api::QueryType;
use crate::marvin_api::Task;

#[allow(dead_code)]
#[cfg(not(feature = "qemu"))]
const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
#[allow(dead_code)]
#[cfg(not(feature = "qemu"))]
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

const FULL_API_KEY: &str = env!("MARVIN_FULL_API_KEY");

#[no_mangle]
pub extern "C" fn start_wifi(tasks_ptr: *mut c_void) {
    println!("WIFI BEGIN");

    let tasks_ptr: Box<Mutex<Vec<Task>>> =
        unsafe { Box::from_raw(tasks_ptr as *mut Mutex<Vec<Task>>) };

    let _wifi = init_wifi();

    unsafe {
        esp_idf_sys::vTaskDelay(1);
    }

    loop {
        let tasks = &*tasks_ptr;
        update_marvin_tasks(tasks);
        unsafe {
            esp_idf_sys::vTaskDelay(1000);
        }
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

    println!("Wifi status: {:?}", status);

    /*if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        ApStatus::Started(ApIpStatus::Done),
    ) = status
    {
        info!("Wifi connected");

        //ping(&ip_settings)?;
    } else {
        bail!("Unexpected Wifi status: {:?}", status);
    }*/

    Ok(wifi)
}

#[allow(unused)]
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

fn init_wifi() -> Result<Box<EspWifi>, Error> {
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
    let mut wifi = wifi(netif_stack_arc, sys_loop_stack_arc, default_nvs_arc);
    wifi
}

fn update_marvin_tasks(tasks_box: &Mutex<Vec<Task>>) {
    let res = get_todos_for_today(FULL_API_KEY);
    {
        let mut tasks = tasks_box.lock().unwrap();

        tasks.clear();

        let res = res.unwrap();

        println!("PUSHING TASKS");
        for i in 0..res.len() {
            tasks.push(res[i].clone());
        }
    }
}
