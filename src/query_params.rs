
use serde::{Deserialize};
use actix_web::{web::{Query}};
use super::lib::date_conv::*;

#[derive(Deserialize)]
pub struct InputOptions {
  pub dt: Option<String>, // primary UTC date string
  pub dtl: Option<String>, // primary date string in local time (requires offset)
  pub jd: Option<f64>, // primary jd as a float
  pub zn: Option<String>, // comma-separated lat,lng(,alt) numeric string
  pub loc: Option<String>, // comma-separated lat,lng(,alt) numeric string
  pub place: Option<String>, // comma-separated lat,lng(,alt) numeric string
  pub cc: Option<String>, // country code
}

pub fn match_datetime_from_params(params:&Query<InputOptions>) -> String {
  let mut dt_str: String = params.dt.clone().unwrap_or("".to_string());
  let has_dt = dt_str.contains("-") && dt_str.len() > 6;
  let jd = if has_dt { 0f64 } else { params.jd.clone().clone().unwrap_or(0f64) };
  if jd > 2_000_000f64 { 
    dt_str = julian_day_to_iso_datetime(jd);
  }
  iso_string_to_datetime(dt_str.as_str()).to_string()
}