use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WeekdayCode {
  pub iso: u8,
  pub sun: u8,
  pub abbr: String,
}

impl WeekdayCode {
  pub fn new(iso: u8, abbr: &str) -> Self {
    let sun = if iso < 7 { iso + 1 } else { 1 };
    return WeekdayCode {
      iso,
      sun,
      abbr: abbr.to_string()
    }
  }

}
