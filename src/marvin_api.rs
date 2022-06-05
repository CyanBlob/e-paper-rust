use serde::{Deserialize, Serialize};

use embedded_svc::http::*;
use embedded_svc::httpd::registry::*;
use embedded_svc::httpd::*;

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

use esp_idf_sys::{self, c_types};
use esp_idf_sys::{esp, EspError};
use log::*;

//#[cfg(not(test))]

pub enum QueryType {
    GET,
    POST,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub result: ApiResult,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)] // this is what lets serde guess at how to deserialize ApiResponse properly
pub enum ApiResult {
    Tasks(Vec<Task>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub _id: String,
    pub _rev: String,
    pub createdAt: u32,
    pub db: String,
    pub title: String,
    pub _type: String,
    pub parentId: Option<String>,
    pub rank: i32,
    pub masterRank: i32,
    pub dueDate: Option<u32>,
    pub updatedAt: u32,
    pub day: Option<String>,
    pub timeEstimate: Option<u32>,
    pub firstScheduled: Option<String>,
    pub workedOnAt: Option<u32>,
    pub fieldUpdates: FieldUpdates,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct FieldUpdates {
    pub dueDate: Option<u32>,
    pub masterRank: Option<u32>,
    pub updatedAt: u32,
    pub parentId: Option<String>,
    pub day: Option<u32>,
    pub rank: u32,
    pub timeEstimate: Option<u32>,
    pub firstScheduled: Option<u32>,
    pub workedOnAt: Option<u32>,
}

fn print_memory() {
    unsafe {
        println!(
            "API HEAP INTERNAL: {}",
            esp_idf_sys::esp_get_free_internal_heap_size()
        );
        println!(
            "API HEAP REMAINING: {}",
            esp_idf_sys::esp_get_free_heap_size()
        );
        println!(
            "API TASK HIGH WATER MARK: {}",
            esp_idf_sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut())
        );
    }
}

pub fn simple_query(
    token: &str,
    endpoint: &str,
    query_type: QueryType,
    form_args: Option<Box<[(&str, &str)]>>,
    //) -> Result<ApiResult, serde_json::Error> {
) -> Result<ApiResult, Box<dyn std::error::Error>> {
    use embedded_svc::http::{self, client::*, status, Headers, Status};
    use embedded_svc::io::Bytes;
    use esp_idf_svc::http::client::*;
    use esp_idf_sys::c_types;

    //let url: String = format!("{}/{}", "https://serv.amazingmarvin.com/api", endpoint);
    let url: String  = String::from("http://google.com");

    println!("About to fetch content from {}", url);
    let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
        //crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_detach),
        ..Default::default()
    })?;

    println!("Created client!");

    print_memory();

    let response = client.get(&url)?.submit()?;

    println!("Sent request");

    let body: Result<Vec<u8>, _> = Bytes::<_, 64>::new(response.reader()).take(3084).collect();

    println!("Parsed body");

    let body = body?;

    println!(
        "Body (truncated to 3K):\n{:?}", body
        //String::from_utf8_lossy(&body).into_owned()
    );

    print_memory();

    Ok(ApiResult::Tasks(Vec::<Task>::new()))
}
