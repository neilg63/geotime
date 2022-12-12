use serde::{Serialize, Deserialize};
use serde_json::*;
use clap::Parser;
use super::super::args::*;
use super::timezonedb::*;
use super::super::{constants::*, lib::json_extract::*, lib::cached_http_client::*};

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
        let lng = extract_f64_from_value_map(&row, "lng");
        let lat = extract_f64_from_value_map(&row, "lat");
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

    pub fn new_from_params(lat: f64, lng: f64, name: String, fcode: String, pop: u32) -> GeoNameRow {
      GeoNameRow { 
        lng,
        lat,
        name: name.clone(),
        toponym: name,
        fcode,
        pop
      }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoNameNearby {
    pub lng: f64,
    pub lat: f64,
    pub name: String,
    pub toponym: String,
    pub fcode: String,
    pub distance: f64,
    pub pop: u32,
    pub admin_name: String,
    pub country_name: String,
}

impl GeoNameNearby {
  pub fn new(row: Map<String, Value>) -> GeoNameNearby {
    let lng = extract_f64_from_value_map(&row, "lng");
    let lat = extract_f64_from_value_map(&row, "lat");
    let name = extract_string_from_value_map(&row, "name");
    let admin_name = extract_string_from_value_map(&row, "adminName1");
    let country_name = extract_string_from_value_map(&row, "countryName");
    let toponym = extract_string_from_value_map(&row, "toponymName");
    let fcode = extract_string_from_value_map(&row, "fcode");
    let pop = extract_u32_from_value_map(&row, "population");
    let distance = extract_f64_from_value_map(&row, "distance");
    GeoNameNearby { 
        lng,
        lat,
        name,
        toponym,
        fcode,
        distance,
        pop,
        admin_name,
        country_name,
    }
  }

  pub fn to_rows(&self) -> Vec<GeoNameRow> {
      let mut rows: Vec<GeoNameRow> = vec![];
      rows.push(GeoNameRow::new_from_params(self.lat, self.lng, self.country_name.clone(), "PCLI".to_string(), 0));
      rows.push(GeoNameRow::new_from_params(self.lat, self.lng, self.admin_name.clone(), "ADM1".to_string(), 0));
      rows.push(GeoNameRow::new_from_params(self.lat, self.lng, self.name.clone(), self.fcode.clone(), self.pop));
      rows
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
    dotenv::var("geonames_username").unwrap_or(GEONAMES_USERNAME_DEFAULT.to_owned())
  }
}

fn match_max_nearby_radius() -> String {
  let def_string_val = GEONAMES_MAX_NEARBY_DISTANCE.to_string();
  let radius_ref = dotenv::var("max_nearby_radius").unwrap_or(def_string_val.clone());
  if let Ok(radius) = radius_ref.parse::<f64>() {
    if radius >= 0f64 {
      radius_ref
    } else {
      def_string_val
    }
  } else {
    def_string_val
  }
}

pub async fn fetch_from_geonames(method: &str, lat: f64, lng: f64) -> Option<Map<String, Value>> {
  let url = format!("{}/{}", GEONAMES_API_BASE, method);
  let lat_str = lat.to_string();
  let lng_str = lng.to_string();
  let uname = match_geonames_username();
  //let client = reqwest::Client::new();
  let client = get_cached_http_client();
  let result = match method {
    "findNearbyJSON" => client.get(url).query(&[
        ("username", uname.as_str()),
        ("lat", lat_str.as_str()),
        ("lng", lng_str.as_str()),
        ("featureClass", "P"),
        ("radius", match_max_nearby_radius().as_str())
        ]).send()
      .await
      .expect("failed to get response")
      .text()
      .await
    ,
    _ => client.get(url).query(&[
        ("username", uname.as_str()),
        ("lat", lat_str.as_str()),
        ("lng", lng_str.as_str()),
      ]).send()
        .await
        .expect("failed to get response")
        .text()
        .await
  };
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
        rows = fetch_nearby_from_geonames(lat, lng).await;
        if rows.len() < 2 {
          rows = match &data["ocean"] {
              Value::Object(row_map) => {
                  let new_row = GeoNameRow::new_ocean(row_map.clone(), lat, lng);
                  vec![new_row]
              },
              _ => vec![]
          };
        }
      }
  }
  rows
}

pub async fn fetch_nearby_from_geonames(lat: f64, lng: f64) -> Vec<GeoNameRow> {
  let output = fetch_from_geonames("findNearbyJSON", lat, lng).await;
  let mut rows:Vec<GeoNameRow> = vec![];
  if let Some(data) = output {
      if data.contains_key("geonames") {
          rows = match &data["geonames"] {
              Value::Array(items) => {
                  let mut new_rows: Vec<GeoNameRow> = vec![];
                  for row in items {
                      match row {
                          Value::Object(row_map) => {
                              let nearby_row = GeoNameNearby::new(row_map.clone());
                              if nearby_row.distance <= GEONAMES_MAX_NEARBY_DISTANCE {
                                  new_rows = nearby_row.to_rows();
                              }
                          },
                          _ => ()
                      }
                  }
                  new_rows
              },
              _ => Vec::new(),
          };
      }
  }
  rows
}

pub async fn fetch_tz_from_geonames(lat: f64, lng: f64) -> Option<TimeZoneInfo> {
  let data = fetch_from_geonames("timezoneJSON", lat, lng).await;
  match data {
      Some(item_data) => {
        let tz_data = TimeZoneInfo::new(item_data);
        if tz_data.tz.len() > 3 {
          Some(tz_data)
        } else {
          None
        }
      },
      _ => None
  }
}

pub fn extract_best_lat_lng_from_placenames(placenames: &Vec<GeoNameRow>, lat: f64, lng: f64) -> (f64, f64) {
  if let Some(last_row) = placenames.last() {
    (last_row.lat, last_row.lng)
  } else {
    (lat, lng)
  }
}

pub fn extract_time_from_first_row(placenames: &Vec<GeoNameRow>, lng: f64) -> Option<TimeZone> {
  let mut time: Option<TimeZone> = None;
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
  time
}

pub async fn fetch_geo_time_info(lat: f64, lng: f64, utc_string: String) -> GeoTimeInfo {
  let placenames = fetch_extended_from_geonames(lat, lng).await;
  let mut time: Option<TimeZone> = None;
  let mut time_matched = false;
  let (best_lat, best_lng) = extract_best_lat_lng_from_placenames(&placenames, lat, lng);
  if let Some(tz_item) = fetch_tz_from_geonames(best_lat, best_lng).await {
    if tz_item.tz.len() > 2 {
      time = match_current_time_zone(tz_item.tz.as_str(), utc_string.as_str(), Some(lng));
      if let Some(time_row) = time.clone() {
        time_matched = time_row.zone_name.len() > 2;
      }
    }
  }
  if !time_matched && placenames.len() > 0 {
    time = extract_time_from_first_row(&placenames, lng);
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
    let rows = fetch_nearby_from_geonames(lat, lng).await;
    if rows.len() > 0 {
      let (best_lat, best_lng) = extract_best_lat_lng_from_placenames(&rows, lat, lng);
      if let Some(tz_item) = fetch_tz_from_geonames(best_lat, best_lng).await {
          match_current_time_zone(tz_item.tz.as_str(), utc_string.as_str(), Some(best_lng))
      } else {
        extract_time_from_first_row(&rows, lng)
      }
    } else {
      let data = fetch_geo_time_info(lat, lng, utc_string).await;
      match data.time {
        Some(time) => Some(time),
        _ => None
      }
    }
  }
}
