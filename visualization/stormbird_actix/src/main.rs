use clap::Parser;

use actix_web::{post, web, App, HttpResponse, HttpServer, Result, http::KeepAlive};
use actix_files as fs;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0")]
    bind_address: String,
    #[arg(short, long, default_value_t = 8050)]
    port: u16,
}

#[post("/update-data")]
async fn update_data(
    data: String,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    println!(
        "Starting server. Binding to address {} and port {}",
        &args.bind_address,
        args.port
    );

    HttpServer::new(move || {
        App::new()
            .service(fs::Files::new("/", "./static").index_file("index.html"))
            .service(update_data)
    })
    .bind((args.bind_address, args.port))?
    .keep_alive(KeepAlive::Os)
    .run()
    .await
}
