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
  let coord_str: String = params.loc.clone().unwrap_or("0,0".to_string());
  let coords: Coords = loc_string_to_coords(coord_str.as_str());
  let corrected_dt = match_datetime_from_params(&params);
  let info = fetch_geo_time_info(coords.lat, coords.lng, corrected_dt).await;
  Json(json!(info))
}

#[get("/timezone")]
pub async fn tz_info(params: Query<InputOptions>) -> impl Responder {
  let zn: String = params.zn.clone().unwrap_or("".to_string());
  let has_zn = zn.len() > 4 && zn.contains("/");
  let coord_str: String = params.loc.clone().unwrap_or("".to_string());
  let has_coords = coord_str.contains(",");
  let coords: Coords = loc_string_to_coords(coord_str.as_str());
  let corrected_dt = match_datetime_from_params(&params);
  let info = match has_zn {
    true => match_current_time_zone(zn.as_str(), corrected_dt.as_str()),
    _ => match has_coords {
        true => fetch_ime_info_from_coords(coords.lat, coords.lng, corrected_dt).await,
        _ => None
    }
  };  
  Json(json!(info))
}
