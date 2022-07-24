use serde::{Serialize, Deserialize};
use serde_json::*;
use clap::Parser;
use super::super::args::*;
use super::timezonedb::*;
use super::super::{constants::*, lib::json_extract::*};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoNameRow {
    pub lng: f64,
    pub lat: f64,
    pub name: String,
    pub toponym: String,
    pub fcode: String,
    pub pop: u32
}

impl GeoNameRow {
    pub fn new(row: Map<String, Value>) -> GeoNameRow {
        let lng = extract_f64_value_map(&row, "lng");
        let lat = extract_f64_value_map(&row, "lat");
        let name = extract_string_from_value_map(&row, "name");
        let toponym = extract_string_from_value_map(&row, "toponymName");
        let fcode = extract_string_from_value_map(&row, "fcode");
        let pop = extract_u32_from_value_map(&row, "population");
        GeoNameRow { 
            lng,
            lat,
            name,
            toponym,
            fcode,
            pop
        }
    }

    pub fn new_ocean(row: Map<String, Value>, lat: f64, lng: f64) -> GeoNameRow {
        let name = extract_string_from_value_map(&row, "name");
        let toponym = extract_string_from_value_map(&row, "name");
        let fcode = "OCEAN".to_string();
        GeoNameRow { 
            lng,
            lat,
            name,
            toponym,
            fcode,
            pop: 0
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeZoneInfo {
    pub cc: String,
    pub tz: String,
}

impl TimeZoneInfo {
    pub fn new(row: Map<String, Value>) -> TimeZoneInfo {
        let cc = extract_string_from_value_map(&row, "countryCode");
        let tz = extract_string_from_value_map(&row, "timezoneId");
        TimeZoneInfo { 
            cc,
            tz
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoTimeInfo {
    placenames: Vec<GeoNameRow>,
    time: Option<TimeZone>,
}

fn match_geonames_username() -> String {
  let args = Args::parse();
  let un = args.geoname;
  if un.len() > 2 {
    un
  } else {
    GEONAMES_USERNAME_DEFAULT.to_owned()
  }
}

pub async fn fetch_from_geonames(method: &str, lat: f64, lng: f64) -> Option<Map<String, Value>> {
  let url = format!("{}/{}", GEONAMES_API_BASE, method);
  let client = reqwest::Client::new();
  let result = client.get(url)
      .query(
      &[
      ("username", match_geonames_username().as_str()),
      ("lat", lat.to_string().as_str()),
      ("lng", lng.to_string().as_str()),
      ]
  ).send()
  .await
  .expect("failed to get response")
  .text()
  .await;
  if let Ok(result_string) = result {
      let data: Map<String, Value> = serde_json::from_str(result_string.as_str()).unwrap();
      Some(data.clone())
  } else {
      None
  }
}


pub async fn fetch_extended_from_geonames(lat: f64, lng: f64) -> Vec<GeoNameRow> {
  let output = fetch_from_geonames("extendedFindNearbyJSON", lat, lng).await;
  let mut rows:Vec<GeoNameRow> = vec![];
  if let Some(data) = output {
      if data.contains_key("geonames") {
          rows = match &data["geonames"] {
              Value::Array(items) => {
                  let mut new_rows: Vec<GeoNameRow> = vec![];
                  let num_items = items.len();
                  for row in items {
                      match row {
                          Value::Object(row_map) => {
                              let new_row = GeoNameRow::new(row_map.clone());
                              let fcode_ref = new_row.fcode.as_str();
                              if fcode_ref != "AREA" && (fcode_ref != "CONT" || num_items < 3) {
                                  new_rows.push(new_row);
                              }
                          },
                          _ => ()
                      }
                  }
                  new_rows
              },
              _ => Vec::new(),
          };
      } else if data.contains_key("ocean") {
        rows = match &data["ocean"] {
            Value::Object(row_map) => {
                let new_row = GeoNameRow::new_ocean(row_map.clone(), lat, lng);
                vec![new_row]
            },
            _ => vec![]
        };
      }
  }
  rows
}

pub async fn fetch_tz_from_geonames(lat: f64, lng: f64) -> Option<TimeZoneInfo> {
  let data = fetch_from_geonames("timezoneJSON", lat, lng).await;
  match data {
      Some(item_data) => Some(TimeZoneInfo::new(item_data)),
      _ => None
  }
}

pub async fn fetch_geo_time_info(lat: f64, lng: f64, utc_string: String) -> GeoTimeInfo {
  let placenames = fetch_extended_from_geonames(lat, lng).await;
  let mut time: Option<TimeZone> = None;
  let mut time_matched = false;
  if let Some(tz_item) = fetch_tz_from_geonames(lat, lng).await {
    if tz_item.tz.len() > 2 {
      time = match_current_time_zone(tz_item.tz.as_str(), utc_string.as_str(), Some(lng));
      if let Some(time_row) = time.clone() {
        time_matched = time_row.zone_name.len() > 2;
      }
    }
  }
  if !time_matched && placenames.len() > 0 {
    if let Some(row) = placenames.get(0) {
      let words: Vec<&str> = row.name.split(" ").collect();
      let name_opt = words.into_iter().find(|s| match s.to_lowercase().as_str() {
        "north" | "south" | "east" | "west" => false,
        _ => true
      });
      if let Some(name) = name_opt {
        time = Some(TimeZone::new_ocean(name.to_owned(), lng));
      }
      
    }
  }
  GeoTimeInfo { 
    placenames,
    time
  }
}

pub async fn fetch_time_info_from_coords(lat: f64, lng: f64, utc_string: String) -> Option<TimeZone> {
  if let Some(tz_item) = fetch_tz_from_geonames(lat, lng).await {
      match_current_time_zone(tz_item.tz.as_str(), utc_string.as_str(), Some(lng))
  } else {
    None
  }
}
