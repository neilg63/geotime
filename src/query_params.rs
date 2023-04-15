
use serde::{Deserialize};
use actix_web::{web::{Query}};
use super::lib::{date_conv::*, coords::*};

#[derive(Deserialize)]
pub struct InputOptions {
  pub dt: Option<String>, // primary UTC date string
  pub dtl: Option<String>, // primary date string in local time
  pub jd: Option<f64>, // primary jd as a float
  pub un: Option<i64>, // primary unix timestamp as an integer
  pub zn: Option<String>, // comma-separated lat,lng(,alt) numeric string
  pub loc: Option<String>, // comma-separated lat,lng(,alt) numeric string
  pub place: Option<String>, // simple string
  pub cc: Option<String>, // country code
  pub mode: Option<String>, // all: all features, default cities and regions / countries only for search endpoint
  pub fuzzy: Option<u8>, // fuzziness on a scale from 0 to 100
  pub max: Option<u8>, // max rows returned in the /lookup route, default is 20
  pub included: Option<u8>, // Default: 1 (true), 0: false. Place name includes the search string, not just a district of a larger metropolis or region
}

fn is_valid_date_string(dt_str: &str) -> bool {
  dt_str.contains("-") && dt_str.len() > 6 && dt_str.chars().into_iter().filter(|c| c.is_numeric()).collect::<Vec<char>>().len() >= 6
}

pub fn match_datetime_from_params(params:&Query<InputOptions>) -> (String, bool) {
  let mut dt_str: String = params.dt.clone().unwrap_or("".to_string());
  let mut has_dt = is_valid_date_string(&dt_str);
  let mut local = false;
  if !has_dt {
    dt_str = params.dtl.clone().unwrap_or("".to_string());
    has_dt = is_valid_date_string(&dt_str);
    local = true;
  }
  let jd = if has_dt { 0f64 } else { params.jd.clone().unwrap_or(0f64) };
  let has_jd = jd > 2_000_000f64;
  if has_jd {
    dt_str = julian_day_to_iso_datetime(jd);
  } else if !has_dt {
    let max_unix_ts = 4_000_000_000i64;
    let min_unix_ts = -5_000_000_000i64;
    let un = params.un.clone().unwrap_or(min_unix_ts);
    if un > min_unix_ts && un <= max_unix_ts {
      dt_str = unixtime_to_utc(un);
    } else {
      dt_str = current_datetime_string();
    }
  }
  (iso_string_to_datetime(dt_str.as_str()).to_string().replace(" ", "T"), local)
}

pub fn match_coords_from_params(params:&Query<InputOptions>) -> Option<Coords> {
  let coord_str: String = params.loc.clone().unwrap_or("".to_string());
  let has_coords = coord_str.contains(",");
  if has_coords {
    Some(loc_string_to_coords(&coord_str))
  } else {
    None
  }
}