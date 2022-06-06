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
#[allow(non_snake_case)]
pub struct Task {
    pub _id: String,
    pub _rev: String,
    pub createdAt: serde_json::Number,
    pub db: String,
    pub title: String,
    pub _type: Option<String>,
    pub parentId: Option<String>,
    pub rank: serde_json::Number,
    pub masterRank: serde_json::Number,
    pub dueDate: Option<serde_json::Number>,
    pub updatedAt: serde_json::Number,
    pub day: Option<String>,
    pub timeEstimate: Option<serde_json::Number>,
    pub firstScheduled: Option<String>,
    pub workedOnAt: Option<serde_json::Number>,
    pub fieldUpdates: FieldUpdates,
}
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FieldUpdates {
    pub dueDate: Option<serde_json::Number>,
    pub masterRank: Option<serde_json::Number>,
    pub updatedAt: serde_json::Number,
    pub parentId: Option<serde_json::Number>,
    pub day: Option<serde_json::Number>,
    pub rank: serde_json::Number,
    pub timeEstimate: Option<serde_json::Number>,
    pub firstScheduled: Option<serde_json::Number>,
    pub workedOnAt: Option<serde_json::Number>,
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
) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    use embedded_svc::http::{self, client::*, status, Headers, Status};
    use embedded_svc::io::Bytes;
    use esp_idf_svc::http::client::*;
    use esp_idf_sys::c_types;

    let url: String = format!("{}/{}", "http://serv.amazingmarvin.com/api", endpoint);

    println!("About to fetch content from {}", url);
    let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
        //crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_detach) // comment to disable SSL (https). Unsafe, but saves precious memory
        ..Default::default()
    })?;

    println!("Created client!");

    let mut request: esp_idf_svc::http::client::EspHttpRequest = client.get(&url)?;
    request.set_header("X-Full-Access-Token", token);

    let response = request.submit()?;

    print_memory();

    println!("Sent request");

    let body: Result<Vec<u8>, _> = Bytes::<_, 64>::new(response.reader()).collect();

    println!("Parsed body");

    let body = body?;

    let body_str = String::from_utf8_lossy(&body).into_owned();

    let api_result: Result<Vec<Task>, serde_json::Error> = serde_json::from_str(&body_str);

    let unwrapped_result = api_result.unwrap();

    //println!("Body (raw):\n{:?}", body_str);

    println!("Tasks:");
    
    for task in &unwrapped_result {
        println!("{}", task.title);
    }

    print_memory();

    Ok(unwrapped_result)
}
