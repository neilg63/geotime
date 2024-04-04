use crate::services::{timezonedb::*, geonames::*};
use serde_json::*;
use actix_web::{get, Responder, web::{Query, Json}};
use crate::query_params::*;
use crate::app::coords::*;

pub async fn welcome() -> impl Responder {
  Json(json!({ "message": "Welcome to GeoTImeZone" }))
}

pub async fn route_not_found() -> impl Responder {
  Json( json!({ "valid": false, "error": "route not found" }))
}

#[get("/geotime")]
pub async fn geo_time_info(params: Query<InputOptions>) -> impl Responder {
  let mut coords_option = match_coords_from_params(&params);
  let has_coords = coords_option.is_some();
  if !has_coords { 
    let tz_info_opt = extract_zone_name_from_place_params(&params).await;
    if let Some((_tz_info, matched_coords)) = tz_info_opt {
      coords_option = Some(matched_coords);
    }
  }
  let coords = match coords_option {
    Some(cs) => cs,
    _ => Coords::zero()
  };
  let (corrected_dt, local) = match_datetime_from_params(&params);
  let enforce_dst = params.dst.unwrap_or(1) > 0;
  reset_override();
  let adjusted_dt = if local { fetch_adjusted_date_str(coords.lat, coords.lng, &corrected_dt, enforce_dst).await } else { corrected_dt.clone() };

  let info = fetch_geo_time_info(coords.lat, coords.lng, &adjusted_dt, enforce_dst).await;
  Json(json!(info))
}

#[get("/geotz")]
pub async fn geo_tz_info(params: Query<InputOptions>) -> impl Responder {
  let mut coords_option = match_coords_from_params(&params);
  let has_coords = coords_option.is_some();
  if !has_coords { 
    let tz_info_opt = extract_zone_name_from_place_params(&params).await;
    if let Some((_tz_info, matched_coords)) = tz_info_opt {
      coords_option = Some(matched_coords);
    }
  }
  let coords = match coords_option {
    Some(cs) => cs,
    _ => Coords::zero()
  };
  let (corrected_dt, local) = match_datetime_from_params(&params);
  let enforce_dst = params.dst.unwrap_or(1) > 0;
  reset_override();
  let adjusted_dt = if local { fetch_adjusted_date_str(coords.lat, coords.lng, &corrected_dt, enforce_dst).await } else { corrected_dt.clone() };

  let info = fetch_geo_tz_info(coords.lat, coords.lng, &adjusted_dt, enforce_dst).await;
  Json(json!(info))
}

#[get("/timezone")]
pub async fn tz_info(params: Query<InputOptions>) -> impl Responder {
  let mut zn: String = params.zn.clone().unwrap_or("".to_string());
  let mut has_zn = is_valid_zone_name(&zn);
  let coords_option = match_coords_from_params(&params);
  let (corrected_dt, local) = match_datetime_from_params(&params);
  let has_coords = coords_option.is_some();
  
  if !has_zn && !has_coords { 
    let tz_info_opt = extract_zone_name_from_place_params(&params).await;
    if let Some((tz_info, _coords)) = tz_info_opt {
      zn = tz_info.tz;
      has_zn = is_valid_zone_name(&zn);
    }
  }
  let enforce_dst = params.dst.unwrap_or(1) > 0;
  reset_override();
  let result = match has_zn {
    true => match_current_time_zone(&zn, &corrected_dt, None, enforce_dst),
    _ => {
      let ref_coords = if let Some(coords) = coords_option {
        coords
      } else {
        Coords::zero()
      };
      fetch_time_info_from_coords_adjusted(ref_coords, &corrected_dt, local, enforce_dst).await
    }
  };
  let json_info = if let Some(data) = result {
    json!(data)
  } else {
    json!({ "valid": false, "message": "Cannot identify a time zone from the query parameters" })
  };
  Json(json_info)
}

#[get("/nearby")]
pub async fn nearby_info(params: Query<InputOptions>) -> impl Responder {
  let coords_option = match_coords_from_params(&params);
  let mut result = json!({"valid": false });
  if let Some(coords) = coords_option {
    let tolerance = params.fuzzy.unwrap_or(1) as f64;
    if let Some(data) = match_toponym_proximity(coords.lat, coords.lng, tolerance, true) {
      result = json!(data);
    }
  }
  Json(result)
}

#[get("/search")]
pub async fn search_by_name(params: Query<InputOptions>) -> impl Responder {
  let place: String = params.place.clone().unwrap_or("".to_string());
  let has_search = place.len() > 1;
  let fuzzy_100 = params.fuzzy.unwrap_or(100);
  let fuzzy_opt = if fuzzy_100 < 100 && fuzzy_100 > 0 { Some(fuzzy_100 as f32 / 100f32) } else { None };
  let cc_str = params.cc.clone().unwrap_or("".to_string());
  let cc_len = cc_str.len();
  let cc = if cc_len > 1 && cc_len < 4 { 
    Some(cc_str.to_uppercase())
   } else { None };
   let region = params.reg.clone();
   let max_ref = params.max.unwrap_or(50);
   let max = if max_ref > 0 { max_ref } else { 50 };
  let included = params.included.unwrap_or(1) != 0;
  let results = if has_search {
    search_by_fuzzy_names(&place, &cc, &region, fuzzy_opt, false, included, max).await
  } else {
    vec![]
  };
  let count = results.len();
  let message = if has_search {
    "OK"
  } else {
    "Please enter a place name search string with 2 or more letters via ?place=NAME"
  };
  let info = json!({
    "count": count,
    "message": message,
    "results": results
  });  
  Json(json!(info))
}

#[get("/lookup")]
pub async fn lookup_by_name(params: Query<InputOptions>) -> impl Responder {
  let place: String = params.place.clone().unwrap_or("".to_string());
  let has_search = place.len() > 1;
  let fuzzy_100 = params.fuzzy.unwrap_or(100);
  let fuzzy_opt = if fuzzy_100 < 100 && fuzzy_100 > 0 { Some(fuzzy_100 as f32 / 100f32) } else { None };
  let cc_str = params.cc.clone().unwrap_or("".to_string());
  let cc_len = cc_str.len();
  let max_ref = params.max.unwrap_or(20);
  let max = if max_ref > 0 { max_ref } else { 20 };
  let cc = if cc_len > 1 && cc_len < 4 { 
    Some(cc_str.to_uppercase())
   } else { None };
  let region = params.reg.clone();
  let results = if has_search {
    //list_by_fuzzy_name_match(&place, &cc, &region, fuzzy_opt, max).await
    list_by_fuzzy_localities(&place, &cc, &region, fuzzy_opt, max).await
  } else {
    vec![]
  };
  Json(json!(results))
}

#[get("/localities")]
pub async fn lookup_by_locality_name(params: Query<InputOptions>) -> impl Responder {
  let place: String = params.place.clone().unwrap_or("".to_string());
  let has_search = place.len() > 1;
  let cc_str = params.cc.clone().unwrap_or("".to_string());
  let cc_len = cc_str.len();
  let max_ref = params.max.unwrap_or(20);
  let max = if max_ref > 0 { max_ref } else { 20 };
  let cc = if cc_len > 1 && cc_len < 4 { 
    Some(cc_str.to_uppercase())
   } else { None };
  let results = if has_search {
    match_locality(&place, &cc, max)
  } else {
    vec![]
  };
  Json(json!(results))
}