use clap::Parser;
use super::constants::*;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    // Ephemeris path
  #[clap(short, long, value_parser, default_value_t = MYSQL_HOST_DEFAULT.to_string() )]
  pub host: String,
  #[clap(short = 'P', long, value_parser, default_value_t = MYSQL_PORT_DEFAULT )]
  pub port: u16,
  #[clap(short, long, value_parser, default_value_t = MYSQL_DB_DEFAULT.to_string() )]
  pub db: String,
  #[clap(short, long, value_parser, default_value_t = MYSQL_USER_DEFAULT.to_string() )]
  pub user: String,
  #[clap(short, long, value_parser, default_value_t = MYSQL_PASS_DEFAULT.to_string() )]
  pub pass: String,
  #[clap(short, long, value_parser, default_value_t = GEONAMES_USERNAME_DEFAULT.to_string() )]
  pub geoname: String,
  #[clap(short, long, value_parser, default_value_t = 8089 )]
  pub webport: u16,
}
