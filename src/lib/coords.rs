use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Coords {
  pub lat: f64,
  pub lng: f64,
}

impl Coords {
  pub fn new(lat: f64, lng: f64) -> Self {
    return Coords {
      lat: lat,
      lng: lng,
    }
  }

  /*
  * 0º N, 0ºS as default and GeoPos is required
  */
  pub fn zero() -> Self {
    return Coords {
      lat: 0f64,
      lng: 0f64,
    }
  }

}

pub fn loc_string_to_coords(loc: &str) -> Coords {
  let parts: Vec<f64> = loc.split(",").into_iter().map(|p| p.parse::<f64>()).filter(|p| match p { Ok(_n) => true, _ => false } ).map(|p| p.unwrap()).collect();
  if parts.len() >= 2 {
    Coords::new(parts[0], parts[1])
  } else {
    Coords::zero()
  }
}
