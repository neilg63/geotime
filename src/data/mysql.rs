use mysql::*;
use clap::Parser;
use super::super::args::*;

#[derive(Parser, Debug)]
pub struct DbParameters {
    host: String,
    port: u16,
    db: String,
    user: String,
    pass: String,
}

pub fn parse_db_options(args: Args) -> DbParameters {
  DbParameters{
      host: args.host,
      port: args.port,
      db: args.db,
      user: args.user,
      pass: args.pass,
  }
}


pub fn connect_mysql() -> Result<PooledConn> {
  let args = Args::parse();
  let db_params = parse_db_options(args);
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