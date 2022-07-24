extern crate serde_derive;
extern crate serde_json;
extern crate chrono;

mod args;
mod constants;
mod data;
mod lib;
mod services;
mod query_params;
mod routes;

use args::*;
use clap::Parser;
use actix_web::{App, HttpServer, web::{self}};
use routes::*;

#[actix_web::main]
async fn main()  -> std::io::Result<()> {
  
    let args = Args::parse();
    let port = args.webport as u16;
    HttpServer::new(move || {
        App::new()
        .route("/", web::get().to(welcome))
        .service(tz_info)
        .service(geo_time_info)
        .route("/{sec1}", web::get().to(route_not_found))
        .route("/{sec1}/{sec2}", web::get().to(route_not_found))
        .route("/{sec1}/{sec2}/{sec3}", web::get().to(route_not_found))
  })
  .bind(("127.0.0.1", port))?
  .run()
  .await
}
