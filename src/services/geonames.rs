use actix_web::web::Query;
use mysql::prelude::Queryable;
use serde::{Serialize, Deserialize};
use serde_json::*;
use clap::Parser;
use string_patterns::*;
use diacritics::*;
use crate::data::alternative_names::CORRECTED_COUNTRY_CODES;
use crate::data::mysql::connect_mysql;
use crate::app::coords::Coords;
use crate::app::date_conv::iso_string_to_datetime;
use crate::query_params::InputOptions;
use crate::{app::date_conv::unixtime_to_utc, data::alternative_names::ALTERNATIVE_NAMES};

use crate::args::*;
use super::timezonedb::*;
use crate::{constants::*, app::json_extract::*, app::cached_http_client::*};

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
        let cc = extract_optional_string_from_value_map(&row, "countryCode");
        let country_code = correct_country_code_optional(cc);
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

    pub fn new_with_country_name(lat: f64, lng: f64, name: String, full_name: String, fcode: String, pop: u32) -> GeoNameRow {
      GeoNameRow { 
        lng,
        lat,
        name: name.clone(),
        toponym: full_name,
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
      GeoNameSimple { lng: self.lng, lat: self.lat, text: self.text(), zone_name: None }
    }

    pub fn to_key(&self) -> String {
      format!("{}_{}_{}_{}_{}", self.name, self.admin_name.clone().unwrap_or("".to_string()), self.country_code.clone().unwrap_or("".to_string()), self.lat.floor(), self.lng.floor())
    }

    pub fn weighted_pop(&self) -> u64 {
      if self.fcode.starts_with("P") {
        self.pop as u64 * 8u64
      } else {
        self.pop as u64
      }
    }

}



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CountryRow {
  #[serde(rename="countryCode")]
  pub country_code: String,
  #[serde(rename="countryName")]
  pub country_name: String,
}

impl CountryRow {
  pub fn new(cc: String, name: String) -> Self {
    CountryRow {
      country_code: cc,
      country_name: name,
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Locality {
  name: String,
  #[serde(rename="asciiName")]
  ascii_name:	String,
  #[serde(rename="adminName")]
  admin_name: String,
  lat: f64,
  lng:	f64,
  cc: String,
  population: u32,
  #[serde(rename="zoneName")]
  zone_name: String,
}

impl Locality {
    pub fn new(name: String, ascii_name: String, admin_name: String, lat: f64, lng: f64, cc: String, population: u32, zone_name: String) -> Locality {
      let cc = correct_country_code(&cc);
      Locality {
        name,
        ascii_name,
        admin_name,
        lat,
        lng,
        cc,
        population,
        zone_name,
      }
    }

  pub fn weight(&self, text: &str) -> u32 {
    let plain = self.ascii_name.to_lowercase();
    let normal = self.name.to_lowercase();
    let mut match_plain = false;
    let mut pos = plain.find(text).unwrap_or(20) as u32;
    if pos > 12 {
      let pos2 = normal.find(text).unwrap_or(20) as u32;
      if pos2 < pos {
        pos = pos2;
        match_plain = false;
      }
    }
    let ref_name = if match_plain { plain } else { normal };
    let words: Vec<String> = ref_name.split(" ").into_iter().map(|s| s.to_owned()).collect::<Vec<String>>();
    let start_word_index = words.clone().into_iter().position(|s| s.to_owned().starts_with(text)).unwrap_or(10);
    let word_lens = words.into_iter().map(|s| s.len()).collect::<Vec<usize>>();
    let mut main_word_index = 0;
    let mut max_len = 0;
    let mut index: usize = 0;
    for wl in word_lens {
      if wl > max_len {
        max_len = wl;
        main_word_index = index;
      }
      index += 1;
    }
    let exact_match: u32 = if ref_name == text { if ref_name.len() > 3 { 4 } else { 3 } } else { 2 };
    let start_weight: u32 = if start_word_index == main_word_index { 2 } else { 1 };
    let weight: u32 = if pos <= 20 { 20 - pos } else { 0 };
    ((self.population + 5000) / 800) * weight * start_weight * exact_match
  }

  pub fn cc_suffix(&self) -> String {
    if self.cc.len() > 1 {
      format!(" ({})", self.cc)
    } else {
      "".to_owned()
    }
  }

  pub fn admin_suffix(&self) -> String {
    if self.admin_name.len() > 1 && self.admin_name != self.cc {
      format!(", {}", self.admin_name)
    } else {
      "".to_owned()
    }
  }

  pub fn text(&self) -> String {
    format!("{}{}{}", self.name, self.admin_suffix(), self.cc_suffix())
  }


  pub fn to_simple(&self) -> GeoNameSimple {
    GeoNameSimple {
      lng: self.lng,
      lat: self.lat,
      text: self.text(), 
      zone_name: Some(self.zone_name.clone())
    }
  }
}

pub fn fetch_locality_rows(sql: String) -> Vec<Locality> {
    if let Ok(mut conn) = connect_mysql() {
        let results = conn
        .query_map( sql,
            |(name, ascii_name, admin_name, lat, lng, cc, population, zone_name)| {
                Locality::new(name, ascii_name, admin_name, lat, lng, cc, population, zone_name)
            },
        );
        if let Ok(zones) = results {
            if zones.len() > 0 {
                zones
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

pub fn extract_country_row(sql: String) -> Option<CountryRow> {
  if let Ok(mut conn) = connect_mysql() {
      let result = conn
        .query_map(sql, |(country_code, country_name)| {
          CountryRow::new(country_code, country_name)
        },
      );
      if let Ok(rows) = result {
        return rows.get(0).map(|cr| cr.to_owned());
      }
  }
  None
}

pub fn extract_country_name(sql: String) -> Option<String> {
  if let Some(row) = extract_country_row(sql) {
    return Some(row.country_name.to_owned());
  }
  None
}

pub fn extract_country_code(sql: String) -> Option<String> {
  if let Some(row) = extract_country_row(sql) {
    return Some(row.country_code.to_owned());
  }
  None
}

pub fn fetch_geoname_toponym_rows(sql: String, add_country_name: bool) -> Vec<GeoNameNearby> {
  if let Ok(mut conn) = connect_mysql() {
    // from_row(distance: f64, lat: f64, lng: f64, name: &str, admin_name: &str, zn: &str, cc: &str, fcode: &str, pop: u32)
    // lng, lat, name, cc, admin_name, zone_name, fcode, population,
      let results = conn
      .query_map( sql,
          |(lat, lng, name, cc, region, admin_name, zn, fcode, population, distance)| {
            GeoNameNearby::from_db_row(lat, lng, name, cc, region, admin_name, zn, fcode, population, distance, add_country_name)
          },
      );
      if let Ok(zones) = results {
          if zones.len() > 0 {
              zones
          } else {
              vec![]
          }
      } else {
          vec![]
      }
  } else {
      vec![]
  }
}



pub fn match_locality(text: &str, cc: &Option<String>, max: u8) -> Vec<Locality> {
  let limit = if max < 40 { max + 10 } else if max < 80 { max + 20 } else if max < 225 { max + 30 } else { 255 };
  let cc_ref = if let Some(cc_str) = cc { cc_str.to_owned().to_uppercase() } else { "".to_owned() };
  let cc_len = cc_ref.len();
  let has_cc = cc_ref != "ALL" && cc_len > 1 && cc_len < 3;
  let country_clause = if has_cc { format!(" AND cc = '{}'", cc_ref) } else { "".to_owned() };
  let sql = format!("select name, ascii_name, admin_name, lat, lng, cc, population, zone_name from cities WHERE (name REGEXP '[[:<:]]{}' OR ascii_name REGEXP '[[:<:]]{}') {} ORDER BY population DESC LIMIT {}", text,text, country_clause, limit);
  
  let mut rows = fetch_locality_rows(sql);
  let lc_text = text.to_lowercase();
  rows.sort_by(|a, b| b.weight(&lc_text).cmp(&a.weight(&lc_text)));
  rows
}

fn lat_lng_to_distance_sql_field(lat: f64, lng: f64) -> String {
  format!("(
    6371 * acos(
      cos(radians({})) * cos(radians(x(g))) * cos(radians(y(g)) - radians({}))
      +
      sin(radians({})) * sin(radians(x(g)))
    )
  ) AS distance", lat, lng, lat)
}

pub fn match_toponym_proximity(lat: f64, lng: f64, tolerance: f64, add_country_name: bool) -> Option<GeoNameNearby> {
  let min_lng = lng - tolerance;
  let max_lng = lng + tolerance;
  let min_lat = lat - tolerance;
  let max_lat = lat + tolerance;
  let distance_field_sql = lat_lng_to_distance_sql_field(lat, lng);
  let sql = format!("SELECT lat, lng, name, cc, region, admin_name, zone_name, fcode, population, {} FROM toponyms WHERE fcode NOT IN ('PCLI', 'ADM1', 'ADM2', 'ANS', 'AIRF', 'AIRP', 'AIRQ') AND lat BETWEEN {} and {} AND lng BETWEEN {} AND {} ORDER BY distance LIMIT 1", distance_field_sql, min_lat, max_lat, min_lng, max_lng);
  
  let rows = fetch_geoname_toponym_rows(sql, add_country_name); 
  
  rows.get(0).map(|row| row.to_owned())
}

pub fn match_country_name(cc: &str) -> Option<String> {
  let code = recorrect_country_code(cc).to_uppercase();
  let sql = format!("select * FROM country WHERE country_code = '{}' LIMIT 1", code);
  
  extract_country_name(sql)
}

pub fn match_cc_from_country_name(c_name: &str) -> Option<String> {
  let sql = format!("select * FROM country WHERE country_name LIKE '{}%' LIMIT 1", c_name);
  extract_country_code(sql)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoNameSimple {
    pub lng: f64,
    pub lat: f64,
    pub text: String,
    #[serde(rename="zoneName",skip_serializing_if = "Option::is_none")]
    pub zone_name: Option<String>
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
    #[serde(rename="adminName")]
    pub admin_name: String,
    pub region: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    #[serde(rename="countryName")]
    pub country_name: String,
    #[serde(rename="zoneName",skip_serializing_if = "Option::is_none")]
    pub zone_name: Option<String>
}

impl GeoNameNearby {
  pub fn new(row: Map<String, Value>) -> GeoNameNearby {
    let lng = extract_f64_from_value_map(&row, "lng");
    let lat = extract_f64_from_value_map(&row, "lat");
    let name = extract_string_from_value_map(&row, "name");
    let region = extract_string_from_value_map(&row, "adminName1");
    let admin_name = extract_string_from_value_map(&row, "adminName2");
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
      region,
      cc: None,
      country_name,
      zone_name: None
    }
  }

  pub fn from_db_row(lat: f64, lng: f64, name: String, cc: String, region:String, admin_name: String, zn: String, fcode: String, pop: u32, distance: f64, add_country_name: bool) -> GeoNameNearby {
    let admin_name = admin_name.to_string();
    let c_code = correct_country_code(&cc);
    let c_name = if add_country_name { match_country_name(&c_code) } else { Some(c_code.clone()) };
    let country_name = c_name.unwrap_or(c_code.clone());
    let cc = if add_country_name { Some(c_code) } else { None };
    GeoNameNearby { 
        lng,
        lat,
        name: name.clone(),
        toponym: name,
        fcode,
        distance,
        pop,
        admin_name,
        region,
        cc,
        country_name,
        zone_name: Some(zn.to_string()),
    }
  }

  pub fn from_places(places: &[GeoNameRow], date_str: &str) -> Option<Self> {
    let num_places = places.len();
    if num_places > 0 {
      let best = places.last().unwrap().to_owned();
      let name = best.name.as_str();
      let fcode = best.fcode.as_str();
      let mut admin_name = "";
      let mut country_name = "";
      let is_ocean = fcode == "OCEAN" || fcode == "SEA";
      let mut pop = 0;
      if !is_ocean {
        pop = best.pop;
      }
      let mut cc: Option<String> = None;
      if !is_ocean && num_places > 1 {
        if let Some(item) = places.into_iter().find(|p| p.fcode.starts_with("ADM")) {
          admin_name = item.name.as_str();
        }
        if let Some(item) = places.into_iter().find(|p| p.fcode.starts_with("PCLI")) {
          country_name = item.name.as_str();
          cc = match_cc_from_country_name(&country_name);
        }
      }
      let mut zone_name: Option<String> = None;
      if is_ocean {
        let tz = TimeZone::new_ocean(name, best.lng, date_str);
        zone_name = Some(tz.zone_name);
      }
      Some( GeoNameNearby { 
        lng: best.lng,
        lat: best.lat,
        name: name.to_owned(),
        toponym: name.to_owned(),
        fcode: fcode.to_owned(),
        distance: 0f64,
        pop,
        admin_name: admin_name.to_string(),
        region: name.to_owned(),
        cc,
        country_name: country_name.to_string(),
        zone_name,
    })
    } else {
      None
    }
  }

  pub fn to_rows(&self) -> Vec<GeoNameRow> {
      let mut rows: Vec<GeoNameRow> = vec![];
      let cc = self.country_name.clone(); // country name is the code when it comes from toponyms is mapped to the name field
      let full_name = match_country_name(&self.country_name).unwrap_or(cc.clone());
      rows.push(GeoNameRow::new_with_country_name(self.lat, self.lng, cc, full_name, "PCLI".to_string(), 0));
      let region = self.region.clone();
      let has_region = self.region.len() > 0;
      if has_region {
        rows.push(GeoNameRow::new_from_params(self.lat, self.lng, region.clone(), "ADM1".to_string(), 0));

      }
      if self.admin_name.len() > 0 && self.admin_name != region {
        let fcode = if has_region { "ADM2" } else { "ADM1" };
        rows.push(GeoNameRow::new_from_params(self.lat, self.lng, self.admin_name.clone(), fcode.to_string(), 0));
      }
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
        let mut cc = extract_string_from_value_map(&row, "countryCode");
        let mut tz = extract_string_from_value_map(&row, "timezoneId");
        if tz.len() < 3 {
          let hrs_offset = extract_f64_from_value_map(&row, "gmtOffset");
          let hrs_suffix = if hrs_offset % 1.0 == 0.0 {
            format!("{:0}", hrs_offset.abs())
          } else {
            format!("{:2}", hrs_offset.abs())
          };
          let letter = if hrs_offset < 0.0 { "W" } else { "E" };
          let lat = extract_f64_from_value_map(&row, "lat");
          let lng = extract_f64_from_value_map(&row, "lng");
          let mut zone_prefix = "";
          if lat > 68.0  {
            zone_prefix = "Arctic";
          } else if lng > 150.0 || lng < -110.0 && lat > -60.0 && lat < 68.0 {
            zone_prefix = if lat < 0.0 {
              "South_Pacific"
            } else {
              "North_Pacific"
            };
          } else if lng < 15.0 && lng > -80.0 {
            zone_prefix = if lat < 0.0 {
              "South_Atlantic"
            } else {
              "North_Atlantic"
            };
          } else if lat > 20.0 && lng > 20.0 && lng > -60.0 && lat < 100.0 {
            zone_prefix = "Indian";
          } else if lat <= -60.0 {
            zone_prefix = "Southern";
          } else {
            if lat > 30.0 && lng > -120.0 && lng < -60.0 {
              zone_prefix = "North_America";
            } else {
              zone_prefix = if lat < 0.0 {
                "Southern_Hemisphere"
              } else {
                "Northern_Hemisphere"
              };
            }
          }
          tz = format!("{}/{}{}", zone_prefix, hrs_suffix, letter);
          cc = "-".to_string();
        }
        TimeZoneInfo { 
            cc,
            tz
        }
    }

/*   pub fn from_strs(tz: &str, cc: &str) -> TimeZoneInfo {
    TimeZoneInfo { 
      cc: cc.to_string(),
      tz: tz.to_string()
    }
  } */
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoTimeInfo {
    placenames: Vec<GeoNameRow>,
    time: Option<TimeZone>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoTimeZone {
  #[serde(skip_serializing_if = "Option::is_none")]
  place: Option<GeoNameNearby>,
  #[serde(skip_serializing_if = "Option::is_none")]
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
  
  // let client = reqwest::Client::new();
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
  rows.sort_by(|a,b| b.weighted_pop().partial_cmp(&a.weighted_pop()).unwrap());
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

pub async fn fetch_timezone_from_place_reference(place: &str, cc: &Option<String>, region: &Option<String>) -> Option<(TimeZoneInfo, Coords)> {
  let rows = search_by_fuzzy_names(place, cc, region, None, false, true, 4).await;
  if let Some(first) = rows.get(0) {
    let zi_opt = fetch_tz_from_geonames(first.lat, first.lng).await;
    if let Some(zi) = zi_opt {
      Some((zi, Coords::new(first.lat, first.lng)))
    } else {
      None
    }
  } else {
    None
  }
}

pub async fn extract_zone_name_from_place_params(params: &Query<InputOptions>) -> Option<(TimeZoneInfo, Coords)> {
  let place_ref = params.place.clone().unwrap_or("".to_string());
  let has_place = place_ref.len() > 2;
  let cc_ref = params.cc.clone().unwrap_or("".to_owned());
  let has_cc = cc_ref.len() > 1 && cc_ref.len() < 4;
  let cc = if has_cc { Some(cc_ref) } else { None };
  let match_by_place = has_place && has_cc;
  let reg_ref = if match_by_place { params.reg.clone().unwrap_or("".to_owned()) } else { "".to_owned() };
  let region = if reg_ref.len() > 1 { Some(reg_ref) } else { None };
  fetch_timezone_from_place_reference(&place_ref, &cc, &region).await
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
  let mut nearby_row_opt = match_toponym_proximity(lat, lng, 1.25, false);
  if nearby_row_opt.is_none() {
    nearby_row_opt = match_toponym_proximity(lat, lng, 2.5, false);
  }
  let placenames = if let Some(nb_row) = nearby_row_opt.clone() {
    nb_row.to_rows()
  } else {
    fetch_extended_from_geonames(lat, lng).await
  };
  let mut time: Option<TimeZone> = if let Some(nb_row) = nearby_row_opt {
    if let Some(zn) = nb_row.zone_name {
      match_current_time_zone(&zn, utc_string, Some(lng), enforce_dst)
    } else {
      None
    }
  } else {
    None
  };
  let mut time_matched = time.is_some();
  if !time_matched {
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
  }
  GeoTimeInfo { 
    placenames,
    time
  }
}

fn is_in_ocean_zone(lat: f64, lng: f64) -> bool {
  if (lng > 150.0 && lng < -130.0) || (lng > -50.0 && lng > 10.0) {
    true
  } else if lat < 20.0 && lng > 40.0 && lng < 110.0 {
    true
  } else {
    false
  }
}

pub async fn fetch_geo_tz_info(lat: f64, lng: f64, utc_string: &str, enforce_dst: bool) -> GeoTimeZone {
  let mut place = match_toponym_proximity(lat, lng, 1.25, true);
  if place.is_none() && !is_in_ocean_zone(lat, lng) {
    place = match_toponym_proximity(lat, lng, 3.0, true);
  }
  let mut time: Option<TimeZone> = if let Some(nb_row) = place.clone() {
    if let Some(zn) = nb_row.zone_name {
      match_current_time_zone(&zn, utc_string, Some(lng), enforce_dst)
    } else {
      None
    }
  } else {
    None
  };
  if place.is_none() {
    let placenames = fetch_extended_from_geonames(lat, lng).await;
    place = GeoNameNearby::from_places(&placenames, utc_string);
    if let Some(pl) = place.clone() {
      if let Some(zn) = pl.zone_name.clone() {
        time = match_current_time_zone(&zn, utc_string, Some(lng), enforce_dst);
      } else {
        if let Some(tz_item) = fetch_tz_from_geonames(lat, lng).await {
          time = match_current_time_zone(&tz_item.tz, utc_string, Some(lng), enforce_dst);
        }
      }
    }
  }
  GeoTimeZone { 
    place,
    time
  }
}

pub async fn fetch_adjusted_date_str(lat: f64, lng: f64, utc_string: &str, enforce_dst: bool) ->String {
  let mut adjusted_dt = utc_string.to_owned();
  if let Some(tz_info) = fetch_time_info_from_coords(lat, lng, utc_string, enforce_dst, true).await {
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
        if let Some(tzi) = fetch_time_info_from_coords(lat, lng, &adjusted_dt, enforce_dst, true).await {
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
    if let Some(tz_info) = fetch_time_info_from_coords_db(lat, lng, utc_string, enforce_dst).await {
      if let Some(unix_ts) = tz_info.ref_unix {
        let adjusted_unix_time = unix_ts - tz_info.gmt_offset as i64;
        if tz_info.gmt_offset != 0 {
          let adjust_dt_str = unixtime_to_utc(adjusted_unix_time);
          fetch_time_info_from_coords_db(lat, lng, &adjust_dt_str, enforce_dst).await
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
    fetch_time_info_from_coords_db(lat, lng, utc_string, false).await
  }
}

pub async fn fetch_time_info_from_coords_adjusted(coords: Coords, utc_string: &str, local: bool, enforce_dst: bool) -> Option<TimeZone> {
  let adjusted_dt = if local { fetch_adjusted_date_str(coords.lat, coords.lng, utc_string, enforce_dst).await } else { utc_string.to_owned() };
  fetch_time_info_from_coords_local(coords.lat, coords.lng, &adjusted_dt, false, enforce_dst).await
}

pub async fn fetch_time_info_from_coords_db(lat: f64, lng: f64, utc_string: &str, enforce_dst: bool) -> Option<TimeZone> {
  let data = fetch_geo_time_info(lat, lng, utc_string, enforce_dst).await;
  let result = match data.time {
    Some(time) => Some(time),
    _ => {
      None
    }
  };
  if result.is_none() {
    fetch_time_info_from_coords(lat, lng, utc_string, enforce_dst, true).await
  } else {
    result
  }
}

pub async fn fetch_time_info_from_coords(lat: f64, lng: f64, utc_string: &str, enforce_dst: bool, skip_fallback: bool) -> Option<TimeZone> {
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
      if !skip_fallback {
        let data = fetch_geo_time_info(lat, lng, utc_string, enforce_dst).await;
        match data.time {
          Some(time) => Some(time),
          _ => {
            None
          }
        }
      } else {
        None
      }
    }
  }
}

pub async fn search_by_fuzzy_names(search: &str, cc: &Option<String>, region: &Option<String>, fuzzy: Option<f32>, all_classes: bool, included: bool, max_rows: u8) -> Vec<GeoNameRow> {
  let url = format!("{}/{}", GEONAMES_API_BASE, "searchJSON"); 
  let client = get_cached_http_client();
  let uname = match_geonames_username();
  let fuzzy_int = if let Some(f_int) = fuzzy { f_int } else { 1f32 };
  let fuzzy_string = fuzzy_int.to_string();
  let mut search_str: String = search.to_owned().clone();
  if let Some(rg_str) = region {
    search_str.push_str(" ");
    search_str.push_str(rg_str);
  }
  let mut items: Vec<(&str, &str)> = vec![
        ("username", &uname),
        ("q", &search_str),
        ("fuzzy", &fuzzy_string)];
  if !all_classes {
    items.push(("featureClass", "P"));
    items.push(("featureClass", "A"));
    items.push(("orderby", "population"));
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

pub fn correct_country_code_optional(cc_opt: Option<String>) -> Option<String> {
  if let Some(cc) = cc_opt {
    if let Some((_cc, new_cc)) = CORRECTED_COUNTRY_CODES.into_iter().find(|pair| pair.0.to_owned() == cc) {
      Some(new_cc.to_string())
    } else {
      Some(cc)
    }
  } else {
    None
  }
}

pub fn correct_country_code(cc: &str) -> String {
  if let Some((_cc, new_cc)) = CORRECTED_COUNTRY_CODES.into_iter().find(|pair| pair.0.to_owned() == cc) {
    new_cc
  } else {
    cc
  }.to_string()
}

pub fn recorrect_country_code(cc: &str) -> String {
  if let Some((table_cc, _other_cc)) = CORRECTED_COUNTRY_CODES.into_iter().find(|pair| pair.1.to_owned() == cc) {
    table_cc
  } else {
    cc
  }.to_string()
}

pub async fn list_by_fuzzy_name_match(search: &str, cc: &Option<String>, region: &Option<String>, fuzzy: Option<f32>, max: u8) -> Vec<GeoNameSimple> {
  let max_initial_search = if max < 10 { 20 } else if max < 127 {  max * 2 } else { 255 };
  let items = search_by_fuzzy_names(search, cc, region, fuzzy, false, false, max_initial_search).await;
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

pub async fn list_by_fuzzy_localities(search: &str, cc: &Option<String>, region: &Option<String>, fuzzy: Option<f32>, max: u8) -> Vec<GeoNameSimple> {
  let fuzzy_val = fuzzy.unwrap_or(100f32);
  let search_remote = fuzzy.unwrap_or(100f32) < 91f32;
  let search_again = fuzzy_val >= 150.0;
  let local_rows = if search_remote { vec![] } else { match_locality(search, cc, max) };
  let str_len = search.len();
  let min_long = if max < 2 { 0 } else if max < 5 { max - 2 } else if max < 20 { 5 } else { 6 } as usize;
  let mut min = min_long;
  if local_rows.len() > 0 {
    let mut is_match = false;
    if let Some(first) = local_rows.get(0) {
      let first_len = if first.name.len() > str_len { first.name.len() } else { str_len };
      let diff_len = first_len - str_len;
      is_match = diff_len < 1;
      min = 1;
    }
    if !is_match {
      min = if str_len > 5 { min_long / 2 } else { min_long };
    }
    if min < 1 {
      min = 1;
    }
  }
  if local_rows.len() < min && search_again {
    list_by_fuzzy_name_match(search, cc, region, fuzzy, max).await
  } else {
    local_rows.into_iter().map(|row| row.to_simple()).collect()
  }
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


fn is_in_simple_string(text: &String, search: &str) -> bool {
  let search_first = search.trim().split(" ").nth(0).unwrap_or("");
  let pat = remove_diacritics(search_first);
  let simple_text = remove_diacritics(text);
  simple_text.pattern_match(&pat, true)
}

fn simplify_string(text: &str) -> String {
  remove_diacritics(text).to_lowercase()
}

fn string_to_static_str(value: u8) -> &'static str {
  let s = value.to_string();
  Box::leak(s.into_boxed_str())
}

pub fn is_valid_zone_name(zn: &str) -> bool {
  zn.len() > 4 && zn.contains("/") && !zn.ends_with("/") && !zn.starts_with("/")
}