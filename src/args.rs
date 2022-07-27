use clap::Parser;
use super::constants::*;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
  // -h MySql/MariaDB host
  #[clap(short, long, value_parser, default_value_t = MYSQL_HOST_DEFAULT.to_string() )]
  pub host: String,
  // -P MySql/MariaDB port
  #[clap(short = 'P', long, value_parser, default_value_t = MYSQL_PORT_DEFAULT )]
  pub port: u16,
  // -d MySql/MariaDB database name
  #[clap(short, long, value_parser, default_value_t = MYSQL_DB_DEFAULT.to_string() )]
  pub db: String,
  // -u MySql/MariaDB user name
  #[clap(short, long, value_parser, default_value_t = MYSQL_USER_DEFAULT.to_string() )]
  pub user: String,
  // -p MySql/MariaDB password
  #[clap(short, long, value_parser, default_value_t = MYSQL_PASS_DEFAULT.to_string() )]
  pub pass: String,
  // -g Geonames user name
  #[clap(short, long, value_parser, default_value_t = GEONAMES_USERNAME_DEFAULT.to_string() )]
  pub geoname: String,
  // -w GeoTimes service port
  #[clap(short, long, value_parser, default_value_t = 8089 )]
  pub webport: u16,
}
