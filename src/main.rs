extern crate serde_derive;
extern crate serde_json;
extern crate chrono;

mod args;
mod constants;
mod data;
mod app;
mod services;
mod query_params;
mod routes;

use args::*;
use clap::Parser;
use actix_web::{App, HttpServer, web::{self}};
use routes::*;

fn match_port() -> u16 {
  let args = Args::parse();
  let mut port = args.webport as u16;
  if port < 1 {
    let env_port = dotenv::var("port").unwrap_or(format!("{:04}", constants::DEFAULT_WEB_PORT));
    if let Ok(port_num) = env_port.parse::<u16>() {
      if port_num > 0 {
        port = port_num;
      }
    }
  }
  port
}

#[actix_web::main]
async fn main()  -> std::io::Result<()> {
    let port = match_port();
    
    HttpServer::new(move || {
        App::new()
        .route("/", web::get().to(welcome))
        .service(tz_info)
        .service(geo_time_info)
        .service(search_by_name)
        .service(lookup_by_name)
        .service(lookup_by_locality_name)
        .service(nearby_info)
        .route("/{sec1}", web::get().to(route_not_found))
        .route("/{sec1}/{sec2}", web::get().to(route_not_found))
        .route("/{sec1}/{sec2}/{sec3}", web::get().to(route_not_found))
  })
  .bind(("127.0.0.1", port))?
  .run()
  .await
}
