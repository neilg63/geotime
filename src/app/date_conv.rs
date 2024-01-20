use crate::app::weekday_code::WeekdayCode;
use chrono::{NaiveDateTime};
use julian_day_converter::*;

pub enum JulianDayEpoch {
  Days = 2440587, // ref year in julian days
  Hours = 12, // ref hours in addition to ref years, 12 hours = 0.5 days
  //RefYear = 1970, // ref year at 1 Jan 00:00:00 UTC for conversion from unix time
}

impl JulianDayEpoch {
  fn days_unix() -> f64 {
    JulianDayEpoch::Days as i64 as f64 + JulianDayEpoch::Hours as i64 as f64 / 24f64
  }
}
/**
 * Utility function to convert any ISO-8601-like date string to a Chrono NaiveDateTime object
 * This function accepts YYYY-mm-dd HH:MM:SS separated by a space or letter T and with or without hours, minutes or seconds.
 * Missing time parts will be replaced by 00, hence 2022-06-23 will be 2022-06-23 00:00:00 UTC and 22-06-23 18:20 will be 2022-06-23 18:30:00
 */
pub fn iso_string_to_datetime(dt: &str) -> NaiveDateTime {
  if let Ok(dt) = iso_fuzzy_string_to_datetime(dt) {
    dt
  } else {
    NaiveDateTime::from_timestamp(0, 0)
  }
}

/*
  Convert the current unixtime to julian days
*/
pub fn unixtime_to_utc(ts: i64) -> String {
  NaiveDateTime::from_timestamp(ts, 0).format("%Y-%m-%dT%H:%M:%S").to_string()
}

/*
  Convert the current unixtime to julian days
*/
pub fn unixtime_to_weekday(ts: i64) -> WeekdayCode {
  let day_ref = NaiveDateTime::from_timestamp(ts, 0).format("%u/%a").to_string();
  let parts = day_ref.split("/").collect::<Vec<_>>();
  let mut abbr = "";
  let mut iso = 0u8;
  if parts.len() > 1 {
    if let Some(num_str) = parts.get(0) {
      if let Ok(num) = num_str.parse::<u8>() {
        iso = num;
      }
    }
    if let Some(abbr_str) = parts.get(1) {
      abbr = abbr_str;
    }
  }
  WeekdayCode::new(iso, abbr)
}


pub fn unixtime_to_julian_day(ts: i64) -> f64 {
  (ts as f64 / 86_400f64) + JulianDayEpoch::days_unix()
}

pub fn julian_day_to_iso_datetime(jd: f64) -> String {
  let datetime = if let Ok(dt) = julian_day_to_datetime(jd) {
    dt
  } else {
    NaiveDateTime::from_timestamp(0, 0)
  };
  datetime.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn current_datetime_string() -> String {
  unixtime_to_utc(chrono::offset::Utc::now().timestamp())
}

pub fn current_timestamp() -> i64 {
  chrono::offset::Utc::now().timestamp()
}

pub fn match_unix_ts_from_fuzzy_datetime(date_str: &str) -> i64 {
  let clean_dt = date_str.replace("T", " ").trim().to_string();
  let date_result = chrono::naive::NaiveDateTime::parse_from_str(clean_dt.as_str(), "%Y-%m-%d %H:%M:%S");
  let dt = match date_result {
      Ok(d) => d,
      _ => chrono::naive::NaiveDateTime::from_timestamp(chrono::offset::Utc::now().timestamp(), 0)
  };
  let ts = dt.timestamp();
  ts
}

pub fn natural_tz_offset_from_utc(lng: f64) -> i32 {
  let lng360 = (lng + 540f64) % 360f64;
  let lng180 = lng360 - 180f64;
  (lng180 * 4f64 * 60f64) as i32
}

pub fn natural_hours_offset_from_utc(lng: f64) -> i32 {
  let zone_deg_offset = if lng < 7.5f64 { -7.5f64 } else { 7.5f64 };
  let secs = if lng >= 172.5f64 { 12i32 * 3600i32 } else { natural_tz_offset_from_utc(lng + zone_deg_offset) };
  secs / 3600
}