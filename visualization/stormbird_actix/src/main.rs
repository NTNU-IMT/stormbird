use std::sync::{Arc, Mutex};

use clap::Parser;

use actix_web::{post, get, web, App, HttpResponse, HttpServer, Result, http::KeepAlive};
use actix_files as fs;

use stormbird::io_structs::result::SimulationResult;

use std::collections::HashMap;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    bind_address: String,
    #[arg(short, long, default_value_t = 8050)]
    port: u16,
}

#[post("/update-data")]
async fn update_data(
    encoded_data: web::Bytes,
    data: web::Data<Arc<Mutex<Vec<SimulationResult>>>>
) -> Result<HttpResponse> {
    println!("Received data");
    let result: SimulationResult = bincode::deserialize(&encoded_data[..]).unwrap();

    let mut data = data.lock().unwrap();

    data.push(result);

    Ok(HttpResponse::Ok().finish())
}

#[post("/clear-data")]
async fn clear_data(
    data: web::Data<Arc<Mutex<Vec<SimulationResult>>>>
) -> Result<HttpResponse> {
    let mut data = data.lock().unwrap();

    data.clear();

    Ok(HttpResponse::Ok().finish())
}

#[get("/get-forces")]
async fn get_forces(
    data: web::Data<Arc<Mutex<Vec<SimulationResult>>>>
) -> Result<HttpResponse> {
    let mut out_data: HashMap<String, Vec<f64>> = HashMap::new();
    out_data.insert("time".to_string(), Vec::new());
    out_data.insert("force_x".to_string(), Vec::new());
    out_data.insert("force_y".to_string(), Vec::new());
    out_data.insert("force_z".to_string(), Vec::new());

    if data.lock().unwrap().is_empty() {
        return Ok(HttpResponse::Ok().json(out_data));
    }

    let data = data.lock().unwrap();

    for (index, result) in data.iter().enumerate() {
        let forces = result.integrated_forces_sum();

        out_data.get_mut("time").unwrap().push(index as f64);
        out_data.get_mut("force_x").unwrap().push(forces[0]);
        out_data.get_mut("force_y").unwrap().push(forces[1]);
        out_data.get_mut("force_z").unwrap().push(forces[2]);
    }

    Ok(HttpResponse::Ok().json(out_data))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let data: Arc<Mutex<Vec<SimulationResult>>> = Arc::new(
        Mutex::new(Vec::new())
    );

    println!(
        "Starting server. Binding to address {} and port {}",
        &args.bind_address,
        args.port
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(data.clone()))
            .service(update_data)
            .service(clear_data)
            .service(get_forces)
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind((args.bind_address, args.port))?
    .keep_alive(KeepAlive::Os)
    .run()
    .await
}
