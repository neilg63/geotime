use crate::lib::date_conv::unixtime_to_utc;

use super::services::{timezonedb::*, geonames::*};
use serde_json::*;
use actix_web::{get, Responder, web::{Query, Json}};
use super::query_params::*;
use super::lib::{coords::*};

pub async fn welcome() -> impl Responder {
  Json(json!({ "message": "Welcome to GeoTImeZone" }))
}

pub async fn route_not_found() -> impl Responder {
  Json( json!({ "valid": false, "error": "route not found" }))
}

#[get("/geotime")]
pub async fn geo_time_info(params: Query<InputOptions>) -> impl Responder {
  let coords_option = match_coords_from_params(&params);
  let coords = match coords_option {
    Some(cs) => cs,
    _ => Coords::zero()
  };
  let (corrected_dt, local) = match_datetime_from_params(&params);
  let adjusted_dt = if local { fetch_adjusted_date_str(coords.lat, coords.lng, &corrected_dt).await } else { corrected_dt };
  let info = fetch_geo_time_info(coords.lat, coords.lng, adjusted_dt).await;
  Json(json!(info))
}

#[get("/timezone")]
pub async fn tz_info(params: Query<InputOptions>) -> impl Responder {
  let zn: String = params.zn.clone().unwrap_or("".to_string());
  let has_zn = zn.len() > 4 && zn.contains("/");
  let coords_option = match_coords_from_params(&params);
  let (corrected_dt, local) = match_datetime_from_params(&params);
  let info = match has_zn {
    true => match_current_time_zone(zn.as_str(), corrected_dt.as_str(), None),
    _ => match coords_option {
        Some(coords) => fetch_time_info_from_coords_local(coords.lat, coords.lng, corrected_dt, local).await,
        _ => None
    }
  };  
  Json(json!(info))
}
