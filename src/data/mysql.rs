use mysql::*;
use clap::Parser;
use super::super::args::*;
use super::super::constants::*;

#[derive(Parser, Debug)]
pub struct DbParameters {
    host: String,
    port: u16,
    db: String,
    user: String,
    pass: String,
}

pub fn parse_db_options(args: &Args) -> DbParameters {
  DbParameters{
      host: match_host(args),
      port: match_db_port(args),
      db: match_db(args),
      user: match_user(args),
      pass: match_pass(args),
  }
}

fn match_db(args: &Args) -> String {
  let arg_name = args.db.clone();
  if arg_name.len() < 1 {
    dotenv::var("db_name").unwrap_or(MYSQL_DB_DEFAULT.to_string())
  } else {
    arg_name
  }
}

fn match_user(args: &Args) -> String {
  let arg_var = args.user.clone();
  if arg_var.len() < 1 {
    dotenv::var("db_user").unwrap_or(MYSQL_USER_DEFAULT.to_string())
  } else {
    arg_var
  }
}

fn match_pass(args: &Args) -> String {
  let arg_var = args.pass.clone();
  if arg_var.len() < 1 {
    dotenv::var("db_pass").unwrap_or(MYSQL_PASS_DEFAULT.to_string())
  } else {
    arg_var
  }
}

fn match_host(args: &Args) -> String {
  let arg_var = args.host.clone();
  if arg_var.len() < 1 {
    dotenv::var("db_host").unwrap_or(MYSQL_HOST_DEFAULT.to_string())
  } else {
    arg_var
  }
}

fn match_db_port(args: &Args) -> u16 {
  let arg_var = args.port;
  if arg_var < 1 {
    let env_port = dotenv::var("db_port").unwrap_or(MYSQL_PORT_DEFAULT.to_string());
    if let Ok(port_num) = env_port.parse::<u16>() {
      port_num
    } else {
      MYSQL_PORT_DEFAULT
    }
  } else {
    arg_var
  }
}

pub fn connect_mysql() -> Result<PooledConn> {
  let args = Args::parse();
  let db_params = parse_db_options(&args);
  let url = format!("mysql://{}:{}@{}:{}/{}", db_params.user, db_params.pass, db_params.host, db_params.port, db_params.db);
  let conn_result = Pool::new(url.as_str());
  match conn_result {
      Ok(pool) => pool.get_conn(),
      Err(e) => {
          println!("{:?}", e);
          Err(e)
      }
  }
}