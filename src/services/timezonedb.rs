use serde::{Serialize, Deserialize};
use mysql::prelude::*;
use super::super::data::mysql::*;
use super::super::lib::date_conv::*;
use chrono::{Datelike};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeZone {
    #[serde(rename="zoneName")]
    pub zone_name: String,
    #[serde(rename="countryCode")]
    pub country_code: String,
    pub abbreviation: String,
    #[serde(rename="timeStart")]
    pub time_start: i64,
    #[serde(rename="timeStartUtc",skip_serializing_if = "Option::is_none")]
    pub time_start_utc: Option<String>,
    #[serde(rename="gmtOffset")]
    pub gmt_offset: i32,
    pub dst: bool,
    #[serde(rename="timeEnd",skip_serializing_if = "Option::is_none")]
    pub time_end: Option<i64>,
    #[serde(rename="timeEndUtc",skip_serializing_if = "Option::is_none")]
    pub time_end_utc: Option<String>,
    #[serde(rename="nextGmtOffset",skip_serializing_if = "Option::is_none")]
    pub next_gmt_offset: Option<i32>,
    #[serde(rename="localDt",skip_serializing_if = "Option::is_none")]
    pub local_dt: Option<String>,
    #[serde(rename="refUnix",skip_serializing_if = "Option::is_none")]
    pub ref_unix: Option<i64>,
    #[serde(rename="refJd",skip_serializing_if = "Option::is_none")]
    pub ref_jd: Option<f64>,
    #[serde(rename="solarUtcOffset",skip_serializing_if = "Option::is_none")]
    pub solar_utc_offset: Option<i32>,
}

impl TimeZone {
  pub fn new(zone_name: String, country_code: String, abbreviation: String, time_start: i64, gmt_offset: i32, dst: bool) -> TimeZone {
    let time_start_utc = Some(unixtime_to_utc(time_start));
    TimeZone { zone_name, country_code, abbreviation, time_start, time_start_utc, gmt_offset, dst, time_end: None, time_end_utc: None, next_gmt_offset: None, local_dt: None, ref_unix: None, ref_jd: None, solar_utc_offset: None }
  }

  pub fn new_ocean(name: String, lng: f64) -> TimeZone {
    let solar_utc_offset = Some(natural_tz_offset_from_utc(lng));
    let gmt_offset_hours = natural_hours_offset_from_utc(lng);
    let letter = if lng < 0f64 { "W" } else { "E" };
    let hours = gmt_offset_hours.abs();
    let zone_name = format!("{}/{:02}{}", name, hours, letter);
    let gmt_offset = gmt_offset_hours * 3600i32;
    TimeZone { 
      zone_name,
      country_code: "".to_string(),
      abbreviation: "".to_string(),
      time_start: 0,
      time_start_utc: None,
      gmt_offset,
      dst: false,
      time_end: None,
      time_end_utc: None,
      next_gmt_offset: None,
      local_dt: None,
      ref_unix: None,
      ref_jd: None,
      solar_utc_offset
    }
  }

  pub fn add_end(&mut self, end_ts: i64, gmt_offset: i32) {
      self.time_end = Some(end_ts);
      self.next_gmt_offset = Some(gmt_offset);
      self.time_end_utc = Some(unixtime_to_utc(end_ts));
  }

  pub fn set_ref_time(&mut self, ref_ts: i64) {
    self.ref_unix = Some(ref_ts);
    self.ref_jd = Some(unixtime_to_julian_day(ref_ts));
    self.local_dt = Some(unixtime_to_utc(ref_ts + self.gmt_offset as i64));
  }

  pub fn set_natural_offset(&mut self, lng: f64) {
    self.solar_utc_offset = Some(natural_tz_offset_from_utc(lng));
  }
}

fn build_natural_timezone(zn: &str, date_str: &str, lng: f64, cc: String) -> Option<TimeZone>{
  let dt = iso_string_to_datetime(date_str);
  let year = dt.year();
  let is_before_1900 = year < 1900i32;
  let abbr = if is_before_1900 { "SOL" } else { "LOC" };
  let solar_utc_offset = natural_tz_offset_from_utc(lng);
  let gmt_offset_hours = if is_before_1900 {solar_utc_offset } else { natural_hours_offset_from_utc(lng) };
  let mut tz_info = TimeZone::new(zn.to_string(), cc, abbr.to_string(), dt.timestamp(), gmt_offset_hours, false );
  tz_info.set_natural_offset(lng);
  Some(tz_info)
}

fn match_nextprev_time_zone(zn: &str, ts: i64, next: bool) -> Option<TimeZone> {
  let comparator = if next { ">"} else { "<=" };
  let direction = if next { "ASC" } else { "DESC" };
  let sql = format!("SELECT zone_name, country_code, abbreviation, time_start, gmt_offset, IF (dst = '1', true, false) AS dst from time_zone 
  WHERE zone_name = '{}' AND time_start {} {}
  ORDER BY time_start {} LIMIT 0, 1", zn, comparator, ts, direction);
  fetch_time_zone_row(sql)
}

pub fn match_current_time_zone(zn: &str, date_str: &str, lng_opt: Option<f64>) -> Option<TimeZone> {
  let ts = match_unix_ts_from_fuzzy_datetime(date_str);
  if let Some(mut current) = match_nextprev_time_zone(zn, ts, false) { 
      if let Some(next) = match_nextprev_time_zone(zn, ts, true) {
          current.add_end(next.time_start, next.gmt_offset);
      }
      current.set_ref_time(ts);
      if let Some(lng) = lng_opt {
        current.set_natural_offset(lng);
      }
      Some(current)
  } else {
    if let Some(lng) = lng_opt {
      let mut cc = "-".to_owned();
      if let Some(current) = match_nextprev_time_zone(zn, current_timestamp(), false) {
        cc = current.country_code;
      }
      build_natural_timezone(zn, date_str, lng, cc)
    } else {
      None
    }
  }
}

pub fn fetch_time_zone_row(sql: String) -> Option<TimeZone> {
    if let Ok(mut conn) = connect_mysql() {
        let zone_results = conn
        .query_map( sql,
            |(zone_name, country_code, abbreviation, time_start, gmt_offset, dst)| {
                TimeZone::new(zone_name, country_code, abbreviation, time_start, gmt_offset, dst)
            },
        );
        if let Ok(zones) = zone_results {
            if zones.len() > 0 {
                if let Some(z) = zones.get(0) {
                    Some(z.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}