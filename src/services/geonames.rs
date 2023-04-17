use serde::{Serialize, Deserialize};
use serde_json::*;
use clap::Parser;
use regex::{Regex};
use diacritics::*;
use crate::lib::coords::Coords;
use crate::lib::date_conv::iso_string_to_datetime;
use crate::{lib::date_conv::unixtime_to_utc, data::alternative_names::ALTERNATIVE_NAMES};

use crate::args::*;
use super::timezonedb::*;
use crate::{constants::*, lib::json_extract::*, lib::cached_http_client::*};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoNameRow {
    pub lng: f64,
    pub lat: f64,
    pub name: String,
    pub toponym: String,
    pub fcode: String,
    pub pop: u32,
    #[serde(rename="countryCode",skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(rename="adminName",skip_serializing_if = "Option::is_none")]
    pub admin_name: Option<String>,
}

impl GeoNameRow {
    pub fn new(row: Map<String, Value>) -> GeoNameRow {
        let lng = extract_f64_from_value_map(&row, "lng");
        let lat = extract_f64_from_value_map(&row, "lat");
        let name = extract_string_from_value_map(&row, "name");
        let toponym = extract_string_from_value_map(&row, "toponymName");
        let fcode = extract_string_from_value_map(&row, "fcode");
        let pop = extract_u32_from_value_map(&row, "population");
        let country_code = extract_optional_string_from_value_map(&row, "countryCode");
        let admin_name = extract_optional_string_from_value_map(&row, "adminName1");
        GeoNameRow { 
            lng,
            lat,
            name,
            toponym,
            fcode,
            pop,
            country_code,
            admin_name,
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
            pop: 0,
            country_code: None,
            admin_name: None,
        }
    }

    pub fn new_from_params(lat: f64, lng: f64, name: String, fcode: String, pop: u32) -> GeoNameRow {
      GeoNameRow { 
        lng,
        lat,
        name: name.clone(),
        toponym: name,
        fcode,
        pop,
        country_code: None,
        admin_name: None,
      }
    }

    pub fn cc_suffix(&self) -> String {
      if let Some(cc) = self.country_code.clone() {
        format!(" ({})", cc)
      } else {
        "".to_owned()
      }
    }

    pub fn admin_suffix(&self) -> String {
      if let Some(a_name) = self.admin_name.clone() {
        format!(", {}", a_name)
      } else {
        "".to_owned()
      }
    }

    pub fn text(&self) -> String {
      format!("{}{}{}", self.name, self.admin_suffix(), self.cc_suffix())
    }

    pub fn to_simple(&self) -> GeoNameSimple {
      GeoNameSimple { lng: self.lng, lat: self.lat, text: self.text() }
    }

    pub fn to_key(&self) -> String {
      format!("{}_{}_{}_{}_{}", self.name, self.admin_name.clone().unwrap_or("".to_string()), self.country_code.clone().unwrap_or("".to_string()), self.lat.floor(), self.lng.floor())
    }

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoNameSimple {
    pub lng: f64,
    pub lat: f64,
    pub text: String,
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
        ("username", &uname),
        ("lat", &lat_str),
        ("lng", &lng_str),
        ("featureClass", &"P".to_owned()),
        ("radius", &match_max_nearby_radius())
        ]).send()
      .await
      .expect("failed to get response")
      .text()
      .await
    ,
    _ => client.get(url).query(&[
        ("username", &uname),
        ("lat", &lat_str),
        ("lng", &lng_str),
      ]).send()
        .await
        .expect("failed to get response")
        .text()
        .await
  };
  if let Ok(result_string) = result {
      let data: Map<String, Value> = serde_json::from_str(&result_string).unwrap();
      Some(data.clone())
  } else {
      None
  }
}


pub async fn fetch_extended_from_geonames(lat: f64, lng: f64) -> Vec<GeoNameRow> {
  let output = fetch_from_geonames("extendedFindNearbyJSON", lat, lng).await;
  map_json_to_geoname_rows(output, Some((lat, lng))).await
}

pub async fn map_json_to_geoname_rows(output: Option<Map<String, Value>>, lat_lng: Option<(f64, f64)>) -> Vec<GeoNameRow> {
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
      } else if lat_lng.is_some() && data.contains_key("ocean") {
        let (lat, lng) = lat_lng.unwrap();
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

pub async fn fetch_ocean_name(lat: f64, lng: f64) -> String {
  let rows = fetch_extended_from_geonames(lat, lng).await;
  if let Some(row) = rows.get(0) {
    let new_name = abbreviate_ocean_name(row);
    new_name
  } else {
    "Ocean".to_owned()
  }
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

pub fn extract_time_from_first_row(placenames: &Vec<GeoNameRow>, lng: f64, utc_string: &str) -> Option<TimeZone> {
  let mut time: Option<TimeZone> = None;
  if let Some(row) = placenames.get(0) {
    let words: Vec<&str> = row.name.split(" ").collect();
    let name_opt = words.into_iter().find(|s| match s.to_lowercase().as_str() {
      "north" | "south" | "east" | "west" => false,
      _ => true
    });
    if let Some(name) = name_opt {
      time = Some(TimeZone::new_ocean(name, lng, utc_string));
    }
  }
  time
}

pub async fn fetch_geo_time_info(lat: f64, lng: f64, utc_string: &str, enforce_dst: bool) -> GeoTimeInfo {
  let placenames = fetch_extended_from_geonames(lat, lng).await;
  let mut time: Option<TimeZone> = None;
  let mut time_matched = false;
  let (best_lat, best_lng) = extract_best_lat_lng_from_placenames(&placenames, lat, lng);

  if let Some(tz_item) = fetch_tz_from_geonames(best_lat, best_lng).await {
    if tz_item.tz.len() > 2 {
      time = match_current_time_zone(tz_item.tz.as_str(), utc_string, Some(lng), enforce_dst);
      if let Some(time_row) = time.clone() {
        time_matched = time_row.zone_name.len() > 2;
      }
    }
  }
  if !time_matched && placenames.len() > 0 {
    time = extract_time_from_first_row(&placenames, lng, utc_string);
  }
  GeoTimeInfo { 
    placenames,
    time
  }
}

pub async fn fetch_adjusted_date_str(lat: f64, lng: f64, utc_string: &str, enforce_dst: bool) ->String {
  let mut adjusted_dt = utc_string.to_owned();
  if let Some(tz_info) = fetch_time_info_from_coords(lat, lng, utc_string, enforce_dst).await {
    if let Some(unix_ts) = tz_info.ref_unix {
      let adjusted_unix_time = unix_ts - tz_info.offset();
      let next_adjusted_unix_time = adjusted_unix_time + tz_info.next_diff_offset();
      adjusted_dt = unixtime_to_utc(adjusted_unix_time);
      let ref_start = if enforce_dst { adjusted_unix_time } else { next_adjusted_unix_time };
      let before_start = next_adjusted_unix_time <= tz_info.period.start.unwrap_or(0);
      let beyond_end = if !before_start && tz_info.period.end.is_some() { ref_start >= tz_info.period.end.unwrap() } else { false };
      let skip = before_start && tz_info.secs_since_start() < tz_info.next_diff_offset().abs();
      if before_start || beyond_end {
        if skip {
          if enforce_dst {
            adjusted_dt = unixtime_to_utc(unix_ts - tz_info.next_diff_offset().abs());
          }
        }
        if let Some(tzi) = fetch_time_info_from_coords(lat, lng, &adjusted_dt, enforce_dst).await {
          let ref_offset = if enforce_dst { tzi.offset() } else { tzi.offset() - tzi.next_diff_offset().abs() };
          let ts = unix_ts - ref_offset;
          adjusted_dt = unixtime_to_utc(ts);
        }
        if tz_info.is_overlap_period_extra() && !enforce_dst {  
          let overlap_secs = tz_info.offset();
          let diff = if enforce_dst { overlap_secs } else { 0 - tz_info.next_diff_offset() };
          if tz_info.offset() < 0 {
            adjusted_dt = unixtime_to_utc(iso_string_to_datetime(&adjusted_dt).timestamp() + diff);
          }
        }
      }
    }
  }
  adjusted_dt
}

fn abbreviate_ocean_name(row: &GeoNameRow) -> String {
  row.name.replace(" Ocean", "").trim().replace(" ", "_")
}

pub async fn fetch_time_info_from_coords_local(lat: f64, lng: f64, utc_string: &str, local: bool, enforce_dst: bool) -> Option<TimeZone> {
  if local {
    if let Some(tz_info) = fetch_time_info_from_coords(lat, lng, utc_string, enforce_dst).await {
      if let Some(unix_ts) = tz_info.ref_unix {
        let adjusted_unix_time = unix_ts - tz_info.gmt_offset as i64;
        if tz_info.gmt_offset != 0 {
          let adjust_dt_str = unixtime_to_utc(adjusted_unix_time);
          fetch_time_info_from_coords(lat, lng, &adjust_dt_str, enforce_dst).await
        } else {
          Some(tz_info)
        }
      } else {
        let name = fetch_ocean_name(lat, lng).await;
        Some(TimeZone::new_ocean(&name, lng, utc_string))
      }
    } else {
      None
    }
  } else {
    fetch_time_info_from_coords(lat, lng, utc_string, false).await
  }
}

pub async fn fetch_time_info_from_coords_adjusted(coords: Coords, utc_string: &str, local: bool, enforce_dst: bool) -> Option<TimeZone> {
  let adjusted_dt = if local { fetch_adjusted_date_str(coords.lat, coords.lng, utc_string, enforce_dst).await } else { utc_string.to_owned() };
  fetch_time_info_from_coords_local(coords.lat, coords.lng, &adjusted_dt, false, enforce_dst).await
}

pub async fn fetch_time_info_from_coords(lat: f64, lng: f64, utc_string: &str, enforce_dst: bool) -> Option<TimeZone> {
  if let Some(tz_item) = fetch_tz_from_geonames(lat, lng).await {
      match_current_time_zone(&tz_item.tz, utc_string, Some(lng), enforce_dst)
  } else {
    let rows = fetch_nearby_from_geonames(lat, lng).await;
    if rows.len() > 0 {
      let (best_lat, best_lng) = extract_best_lat_lng_from_placenames(&rows, lat, lng);
      if let Some(tz_item) = fetch_tz_from_geonames(best_lat, best_lng).await {
          match_current_time_zone(&tz_item.tz, utc_string, Some(best_lng), enforce_dst)
      } else {
        extract_time_from_first_row(&rows, lng, utc_string)
      }
    } else {
      let data = fetch_geo_time_info(lat, lng, utc_string, enforce_dst).await;
      match data.time {
        Some(time) => Some(time),
        _ => {
          None
        }
      }
    }
  }
}

pub async fn search_by_fuzzy_names(search: &str, cc: &Option<String>, fuzzy: Option<f32>, all_classes: bool, included: bool, max_rows: u8) -> Vec<GeoNameRow> {
  let url = format!("{}/{}", GEONAMES_API_BASE, "searchJSON"); 
  let client = get_cached_http_client();
  let uname = match_geonames_username();
  let fuzzy_int = if let Some(f_int) = fuzzy { f_int } else { 1f32 };
  let fuzzy_string = fuzzy_int.to_string();
  let mut items: Vec<(&str, &str)> = vec![
        ("username", &uname),
        ("q", search),
        ("fuzzy", &fuzzy_string)];
  if !all_classes {
    items.push(("featureClass", "P"));
    items.push(("featureClass", "A"));
  }
  if let Some(cc_str) = cc {
    items.push(("country", &cc_str ));
  }
  if included {
    items.push(("isNameRequired", "true" ));
  }
  let search_len = search.len();
  if search_len < 4 && search_len > 0 {
    let ml = if search_len < 2 { 1 } else { 2 };
    items.push(("name_startsWith", &search[0..ml] ));
  }
  let m_str = string_to_static_str(max_rows);
  if max_rows > 1 {
    items.push(("maxRows", m_str ));
  }
  let result = client.get(url).query(&items).send()
        .await
        .expect("failed to get response")
        .text()
        .await;
  if let Ok(result_string) = result {
      let data: Map<String, Value> = serde_json::from_str(&result_string).unwrap();
      map_json_to_geoname_rows(Some(data), None).await
  } else {
      vec![]
  }
}

pub fn matches_alternative(search: &str) -> Option<String> {
  let text = simplify_string(search);
  let pair_opt = ALTERNATIVE_NAMES.into_iter().find(|pair| simplify_string(pair.0).starts_with(&text));
  if let Some(pair) = pair_opt {
    Some(pair.1.to_owned())
  } else {
    None
  }
}

pub async fn list_by_fuzzy_name_match(search: &str, cc: &Option<String>, fuzzy: Option<f32>, max: u8) -> Vec<GeoNameSimple> {
  let max_initial_search = if max < 10 { 20 } else if max < 127 {  max * 2 } else { 255 };
  let items = search_by_fuzzy_names(search, cc, fuzzy, false, true, max_initial_search).await;
  let mut rows: Vec<GeoNameSimple> = Vec::new();
  let mut keys: Vec<String> = Vec::new();
  let mut count: usize = 0;
  let max_count = max as usize;
  for row in items {
    if count < max_count {
      let key = row.to_key();
      if !keys.contains(&key) && is_in_geo_row_alternative(&row, search) {
        keys.push(key);
        rows.push(row.to_simple());
        count += 1;
      }
    }
  }
  rows
}

fn is_in_geo_row(row: &GeoNameRow, search: &str) -> bool {
  is_in_simple_string(&row.name, search) || is_in_simple_string(&row.toponym, search)
}

fn is_in_geo_row_alternative(row: &GeoNameRow, search: &str) -> bool {
  let mut ok = is_in_geo_row(row, search);
  if !ok {
    if let Some(matched_name) = matches_alternative(search) {
      ok = is_in_simple_string(&matched_name, &row.name);
      if !ok {
        ok = is_in_simple_string(&matched_name, &row.toponym)
      }
    }
  }
  ok
}

fn build_regex(pat: &str, case_insensitive: bool) -> Regex {
    let prefix = if case_insensitive { "(?i)" } else { "" };
    let corrected_pattern = [prefix, pat].join("");
    Regex::new(&corrected_pattern).unwrap()
}

fn pattern_matches(text: &str, pat: &str, case_insensitive: bool) -> bool {
    let re = build_regex(pat, case_insensitive);
    re.is_match(text)
}

fn is_in_simple_string(text: &String, search: &str) -> bool {
  let search_first = search.trim().split(" ").nth(0).unwrap_or("");
  let pat = remove_diacritics(search_first);
  let simple_text = remove_diacritics(text);
  pattern_matches(&simple_text, &pat, true)
}

fn simplify_string(text: &str) -> String {
  remove_diacritics(text).to_lowercase()
}

fn string_to_static_str(value: u8) -> &'static str {
  let s = value.to_string();
  Box::leak(s.into_boxed_str())
}