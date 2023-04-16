use serde::{Serialize, Deserialize};
use mysql::prelude::*;
use crate::lib::weekday_code::WeekdayCode;
use crate::data::mysql::*;
use crate::lib::date_conv::*;
use chrono::{Datelike};

struct OffsetOverride {
  value: Option<i32>
}

impl Default for OffsetOverride {
  fn default() -> Self {
    OffsetOverride {value: None}
  }
}

impl OffsetOverride {

  pub fn set(&mut self, value: i32) {
    self.value = Some(value);    
  }

  pub fn reset(&mut self) {
    self.value = None;
  }

  pub fn get(&self) -> Option<i32> {
    self.value
  }

  pub fn is_set(&self) -> bool {
    self.value.is_some()
  }
}

pub fn reset_override() {
  let mut ov = globals::get::<OffsetOverride>();
  ov.reset();
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeZone {
    #[serde(rename="zoneName")]
    pub zone_name: String,
    #[serde(rename="countryCode")]
    pub country_code: String,
    pub abbreviation: String,
    #[serde(rename="gmtOffset")]
    pub gmt_offset: i32,
    pub dst: bool,
    #[serde(rename="localDt",skip_serializing_if = "Option::is_none")]
    pub local_dt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc: Option<String>,
    pub period: TimeZonePeriod,
    #[serde(rename="weekDay",skip_serializing_if = "Option::is_none")]
    pub week_day: Option<WeekdayCode>,
    #[serde(rename="refUnix",skip_serializing_if = "Option::is_none")]
    pub ref_unix: Option<i64>,
    #[serde(rename="refJd",skip_serializing_if = "Option::is_none")]
    pub ref_jd: Option<f64>,
    #[serde(rename="solarUtcOffset",skip_serializing_if = "Option::is_none")]
    pub solar_utc_offset: Option<i32>,
}

impl TimeZone {
  pub fn new(zone_name: String, country_code: String, abbreviation: String, time_start: i64, gmt_offset: i32, dst: bool) -> TimeZone {
    let period = TimeZonePeriod::new(time_start, None, None);
    let ov = globals::get::<OffsetOverride>();
    let gmt_offset = ov.get().unwrap_or(gmt_offset);
    TimeZone { zone_name, country_code, abbreviation, gmt_offset, dst, local_dt: None, utc: None, period, week_day: None, ref_unix: None, ref_jd: None, solar_utc_offset: None }
  }

  pub fn new_ocean(name: &str, lng: f64, date_str: &str) -> TimeZone {
    let solar_utc_offset = Some(natural_tz_offset_from_utc(lng));
    let gmt_offset_hours = natural_hours_offset_from_utc(lng);
    let letter = if lng < 0f64 { "W" } else { "E" };
    let hours = gmt_offset_hours.abs();
    let zone_name = format!("{}/{:02}{}", name, hours, letter);
    let gmt_offset = gmt_offset_hours * 3600i32;
    let unix_ts = iso_string_to_datetime(date_str).timestamp();
    let ref_unix = Some(unix_ts);
    let utc = Some(unixtime_to_utc(unix_ts));
    let adjusted_unix_ts = unix_ts + gmt_offset as i64;
    let local_dt = Some(unixtime_to_utc(adjusted_unix_ts));
    let week_day = Some(unixtime_to_weekday(adjusted_unix_ts));
    TimeZone { 
      zone_name,
      country_code: "".to_string(),
      abbreviation: "".to_string(),
      gmt_offset,
      dst: false,
      local_dt,
      utc,
      period: TimeZonePeriod::empty(),
      week_day,
      ref_unix,
      ref_jd: None,
      solar_utc_offset
    }
  }


  pub fn add_end(&mut self, end_ts: i64, gmt_offset: i32) {
    if let Some(start) = self.period.start {
      self.period = TimeZonePeriod::new(start, Some(gmt_offset), Some(end_ts));
    }
  }

  pub fn time_start(&self) -> i64 {
    if let Some(start) =  self.period.start {
        start
    } else {
      i64::MIN
    }
  }

  pub fn set_ref_time(&mut self, ref_ts: i64) {
    self.ref_unix = Some(ref_ts);
    self.ref_jd = Some(unixtime_to_julian_day(ref_ts));
    let local_unix_ts = ref_ts + self.gmt_offset as i64;
    self.local_dt = Some(unixtime_to_utc(local_unix_ts));
    self.utc = Some(unixtime_to_utc(ref_ts));
    self.week_day = Some(unixtime_to_weekday(local_unix_ts));
  }

  pub fn set_natural_offset(&mut self, lng: f64) {
    self.solar_utc_offset = Some(natural_tz_offset_from_utc(lng));
  }

  pub fn offset(&self) -> i64 {
    self.gmt_offset as i64
  }

  pub fn next_offset(&self) -> i64 {
    if let Some(nx_offset) = self.period.next_gmt_offset {
      nx_offset as i64
    } else {
      self.offset()
    }
  }

  pub fn next_diff_offset(&self) -> i64 {
    self.offset() - self.next_offset()
  }

  pub fn secs_since_start(&self) -> i64 {
    let curr_ts = self.ref_unix.unwrap_or(0);
    if let Some(start_ts) = self.period.start {
      curr_ts - start_ts
    } else {
      0i64
    }
  }

  pub fn secs_to_end(&self) -> i64 {
    let curr_ts = self.ref_unix.unwrap_or(0);
    if let Some(end_ts) = self.period.end {
      end_ts - curr_ts
    } else {
      0i64
    }
  }

  pub fn is_overlap_period(&self) -> bool {
    self.is_overlap_period_offset(0)
  }

  pub fn is_overlap_period_extra(&self) -> bool {
    self.is_overlap_period_offset(0 - self.offset())
  }

  pub fn is_overlap_period_offset(&self, offset: i64) -> bool {
    let next_back = self.next_diff_offset() < 0;
    let diff_abs = self.next_diff_offset().abs();
    if self.secs_to_end() - offset < diff_abs {
      !next_back
    } else if self.secs_since_start() < diff_abs {
      next_back
    } else {
      false
    }
  }

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeZonePeriod {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub start: Option<i64>,
  #[serde(rename="startUtc",skip_serializing_if = "Option::is_none")]
  pub start_utc: Option<String>,
  #[serde(rename="nextGmtOffset", skip_serializing_if = "Option::is_none")]
  pub next_gmt_offset: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub end: Option<i64>,
  #[serde(rename="endUtc",skip_serializing_if = "Option::is_none")]
  pub end_utc: Option<String>,
}

impl TimeZonePeriod {
  pub fn new(start_ts: i64, next_offset: Option<i32>, end: Option<i64>) -> TimeZonePeriod {
    let start_utc = Some(unixtime_to_utc(start_ts));
    let has_end = end.is_some();
    let end_utc = if has_end { Some(unixtime_to_utc(end.unwrap())) } else { None };
    TimeZonePeriod {
      start: Some(start_ts),
      start_utc,
      next_gmt_offset: next_offset,
      end,
      end_utc,
    }
  }

  pub fn empty() -> TimeZonePeriod {
    TimeZonePeriod {
      start: None,
      start_utc: None,
      next_gmt_offset: None,
      end: None,
      end_utc: None,
    }
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

pub fn match_current_time_zone(zn: &str, date_str: &str, lng_opt: Option<f64>, enforce_dst: bool) -> Option<TimeZone> {
  let ts = match_unix_ts_from_fuzzy_datetime(date_str);
  if let Some(mut current) = match_nextprev_time_zone(zn, ts, false) { 
      if let Some(next) = match_nextprev_time_zone(zn, ts, true) {
          current.add_end(next.time_start(), next.gmt_offset);
      }
      let apply_correction = current.is_overlap_period() && !enforce_dst;
      if apply_correction {
        let ts_tomorrow = ts + 86400;
        if let Some(future) = match_nextprev_time_zone(zn, ts_tomorrow, false) {
          current.gmt_offset = future.gmt_offset;
          let mut ov = globals::get::<OffsetOverride>();
          ov.set(current.gmt_offset);
        }
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


