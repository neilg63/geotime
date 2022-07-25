use serde_json::*;

pub fn extract_f64_from_value_map(row: &Map<String, Value>, key: &str) -> f64 {
  match row.get(key) {
      Some(num_str_val) => match num_str_val {
          Value::String(num_str) =>  num_str.parse::<f64>().unwrap_or(0f64),
          Value::Number(num_ref) =>  num_ref.as_f64().unwrap_or(0f64),
          _ => 0f64,
      },
      _ => 0f64,
  }
}

pub fn extract_string_from_value_map(row: &Map<String, Value>, key: &str) -> String {
  match row.get(key) {
      Some(num_str_val) => match num_str_val {
          Value::String(num_str) =>  num_str.to_owned(),
          Value::Number(num_ref) =>  num_ref.to_string(),
          _ => "".to_owned(),
      },
      _ => "".to_owned(),
  }
}

pub fn extract_u32_from_value_map(row: &Map<String, Value>, key: &str) -> u32 {
  match row.get(key) {
      Some(num_str_val) => match num_str_val {
          Value::String(num_str) =>  num_str.parse::<u32>().unwrap_or(0u32),
          Value::Number(num_ref) =>  num_ref.as_i64().unwrap_or(0i64) as u32,
          _ => 0u32,
      },
      _ => 0u32,
  }
}