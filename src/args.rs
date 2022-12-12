use clap::Parser;

fn empty_string() -> String {
  "".to_string()
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
  // -h MySql/MariaDB host
  #[clap(short, long, value_parser, default_value_t = empty_string() )]
  pub host: String,
  // -P MySql/MariaDB port
  #[clap(short = 'P', long, value_parser, default_value_t = 0 )]
  pub port: u16,
  // -d MySql/MariaDB database name
  #[clap(short, long, value_parser, default_value_t = empty_string() )]
  pub db: String,
  // -u MySql/MariaDB user name
  #[clap(short, long, value_parser, default_value_t = empty_string() )]
  pub user: String,
  // -p MySql/MariaDB password
  #[clap(short, long, value_parser, default_value_t = empty_string() )]
  pub pass: String,
  // -g Geonames user name
  #[clap(short, long, value_parser, default_value_t = empty_string() )]
  pub geoname: String,
  // -w GeoTimes service port
  #[clap(short, long, value_parser, default_value_t = 0 )]
  pub webport: u16,
}
