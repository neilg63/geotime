use serde::{Serialize, Deserialize};
use mysql::prelude::*;
use super::super::data::mysql::*;
use super::super::lib::date_conv::*;


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
    pub gmt_offset: i16,
    pub dst: bool,
    #[serde(rename="timeEnd",skip_serializing_if = "Option::is_none")]
    pub time_end: Option<i64>,
    #[serde(rename="timeEndUtc",skip_serializing_if = "Option::is_none")]
    pub time_end_utc: Option<String>,
    #[serde(rename="nextGmtOffset",skip_serializing_if = "Option::is_none")]
    pub next_gmt_offset: Option<i16>,
    #[serde(rename="localDt",skip_serializing_if = "Option::is_none")]
    pub local_dt: Option<String>,
    #[serde(rename="refUnix",skip_serializing_if = "Option::is_none")]
    pub ref_unix: Option<i64>,
    #[serde(rename="refJd",skip_serializing_if = "Option::is_none")]
    pub ref_jd: Option<f64>,
}

impl TimeZone {
    pub fn new(zone_name: String, country_code: String, abbreviation: String, time_start: i64, gmt_offset: i16, dst: bool) -> TimeZone {
      let time_start_utc = Some(unixtime_to_utc(time_start));
      TimeZone { zone_name, country_code, abbreviation, time_start, time_start_utc, gmt_offset, dst, time_end: None, time_end_utc: None, next_gmt_offset: None, local_dt: None, ref_unix: None, ref_jd: None }
    }

    pub fn add_end(&mut self, end_ts: i64, gmt_offset: i16) {
        self.time_end = Some(end_ts);
        self.next_gmt_offset = Some(gmt_offset);
        self.time_end_utc = Some(unixtime_to_utc(end_ts));
    }

    pub fn set_ref_time(&mut self, ref_ts: i64) {
      self.ref_unix = Some(ref_ts);
      self.ref_jd = Some(unixtime_to_julian_day(ref_ts));
      self.local_dt = Some(unixtime_to_utc(ref_ts + self.gmt_offset as i64));
  }
}



fn match_nextprev_time_zone(zn: &str, ts: i64, next: bool) -> Option<TimeZone> {
  let comparator = if next { ">"} else { "<=" };
  let direction = if next { "ASC" } else { "DESC" };
  let sql = format!("SELECT zone_name, country_code, abbreviation, time_start, gmt_offset, IF (dst = '1', true, false) AS dst from time_zone 
  WHERE zone_name = '{}' AND time_start {} {}
  ORDER BY time_start {} LIMIT 0, 1", zn, comparator, ts, direction);
  fetch_time_zone_row(sql)
}

pub fn match_current_time_zone(zn: &str, date_str: &str) -> Option<TimeZone> {
  let ts = match_unix_ts_from_fuzzy_datetime(date_str);
  if let Some(mut current) = match_nextprev_time_zone(zn, ts, false) { 
      if let Some(next) = match_nextprev_time_zone(zn, ts, true) {
          current.add_end(next.time_start, next.gmt_offset);
      }
      current.set_ref_time(ts);
      Some(current)
  } else {
      None
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