#![allow(unused_imports)]
use serde::{Deserialize, Serialize};

use embedded_svc::http::*;
use embedded_svc::httpd::registry::*;
use embedded_svc::httpd::*;

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

#[derive(Debug, Serialize, Deserialize, Default)]
#[allow(non_snake_case)]
pub struct Task {
    pub _id: String,
    pub _rev: String,
    pub createdAt: Option<serde_json::Number>,
    pub db: Option<String>,
    pub title: Option<String>,
    pub _type: Option<String>,
    pub parentId: Option<String>,
    pub rank: Option<serde_json::Number>,
    pub masterRank: Option<serde_json::Number>,
    pub dueDate: Option<serde_json::Number>,
    pub updatedAt: Option<serde_json::Number>,
    pub day: Option<String>,
    pub timeEstimate: Option<serde_json::Number>,
    pub firstScheduled: Option<String>,
    pub workedOnAt: Option<serde_json::Number>,
    pub fieldUpdates: Option<FieldUpdates>,
}
#[derive(Debug, Serialize, Deserialize, Default)]
#[allow(non_snake_case)]
pub struct FieldUpdates {
    pub dueDate: Option<serde_json::Number>,
    pub masterRank: Option<serde_json::Number>,
    pub updatedAt: Option<serde_json::Number>,
    pub parentId: Option<serde_json::Number>,
    pub day: Option<serde_json::Number>,
    pub rank: Option<serde_json::Number>,
    pub timeEstimate: Option<serde_json::Number>,
    pub firstScheduled: Option<serde_json::Number>,
    pub workedOnAt: Option<serde_json::Number>,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.title.as_ref().unwrap().eq(other.title.as_ref().unwrap())
    }
}

impl Eq for Task {}

impl Clone for Task {
    fn clone(&self) -> Self {
        return Task {_id: self._id.to_owned(), title: self.title.to_owned(), ..Default::default()};
    }
}

impl PartialEq for FieldUpdates {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for FieldUpdates {}

impl Clone for FieldUpdates {
    fn clone(&self) -> Self {
        return FieldUpdates {..Default::default()};
    }
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

pub fn get_todos_for_today(
    token: &str,
    //) -> Result<ApiResult, serde_json::Error> {
) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    use embedded_svc::http::{self, client::*, status, Headers, Status};
    use embedded_svc::io::Bytes;
    use esp_idf_svc::http::client::*;

    let url: String = format!("{}/{}", "http://serv.amazingmarvin.com/api", "todayItems");

    println!("About to fetch content from {}", url);

    let body = minreq::get(&url)
        .with_header("X-Full-Access-Token", token.to_owned())
        .send()?;


    println!("Parsed body");

    let body_str = String::from_utf8_lossy(&body.as_bytes()).into_owned();

    println!("Body (raw):\n{:?}", body_str);

    let api_result: Result<Vec<Task>, serde_json::Error> = serde_json::from_str(&body_str);

    match api_result {
        Ok(result) => {
            println!("Tasks:");

            for task in &result {
                println!("{}", &task.title.as_ref().unwrap());
            }

            print_memory();

            Ok(result)
        }
        Err(_) => {
            println!("Failed to parse API response");
            println!("Adding fake data");
            let mut fake_results: Vec<Task> = Vec::<Task>::new();
            fake_results.push(Task {
                title: Some("FAKE TASK".into()),
                createdAt: Some(serde_json::Number::from_f64(696969.0).unwrap()),
                ..Default::default()
            });
            Ok(fake_results)
            //Err("Failed to parse API response".into())
        }
    }
}

pub fn update_todo(
    token: &str,
) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    use embedded_svc::http::{self, client::*, status, Headers, Status};
    use embedded_svc::io::Bytes;
    use esp_idf_svc::http::client::*;

    let url: String = format!("{}/{}", "http://serv.amazingmarvin.com/api", "doc/update");

    println!("About to fetch content from {}", url);
    let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
        //crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_detach) // comment to disable SSL (https). Unsafe, but saves precious memory
        ..Default::default()
    })?;

    println!("Created client!");

    /*{
      "itemId": "xy12n3i123",
      "setters": [
        { "key": "done", "val": true },
        { "key": "fieldUpdates.done", "val": 1648122403647 },
        { "key": "updatedAt", "val": 1648122403647 }
      ]
    } */

    let mut request: esp_idf_svc::http::client::EspHttpRequest = client.post(&url)?;
    request.set_header("X-Full-Access-Token", token);

    let response = request.submit()?;

    print_memory();

    println!("Sent request");

    let body: Result<Vec<u8>, _> = Bytes::<_, 64>::new(response.reader()).collect();

    println!("Parsed body");

    let body = body?;

    let body_str = String::from_utf8_lossy(&body).into_owned();

    println!("Body (raw):\n{:?}", body_str);

    let api_result: Result<Vec<Task>, serde_json::Error> = serde_json::from_str(&body_str);

    //let unwrapped_result = api_result.unwrap();
    match api_result {
        Ok(result) => {
            println!("Tasks:");

            for task in &result {
                println!("{}", &task.title.as_ref().unwrap());
            }

            print_memory();

            Ok(result)
        }
        Err(_) => {
            println!("Failed to parse API response");
            println!("Adding fake data");
            let mut fake_results: Vec<Task> = Vec::<Task>::new();
            fake_results.push(Task {
                title: Some("FAKE TASK".into()),
                createdAt: Some(serde_json::Number::from_f64(696969.0).unwrap()),
                ..Default::default()
            });
            Ok(fake_results)
            //Err("Failed to parse API response".into())
        }
    }
}
