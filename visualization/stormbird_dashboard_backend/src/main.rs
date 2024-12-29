use std::sync::{Arc, Mutex};

use clap::Parser;

use actix_web::{post, get, web, App, HttpResponse, HttpServer, Result, http::KeepAlive};
use actix_files as fs;
use actix_cors::Cors;

use stormbird::io_structs::result::SimulationResult;

use std::collections::HashMap;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    bind_address: String,
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[derive(Debug, Clone, Default)]
pub struct SimulationData {
    pub results: Vec<SimulationResult>,
    pub wake_shape: HashMap<String, Vec<f64>>
}

#[post("/update-result-data")]
async fn update_result_data(
    encoded_data: web::Bytes,
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    println!("Received data");
    let result: SimulationResult = bincode::deserialize(&encoded_data[..]).unwrap();

    let mut data = data.lock().unwrap();

    data.results.push(result);

    Ok(HttpResponse::Ok().finish())
}

#[post("/update-wake-shape")]
async fn update_wake_shape(
    encoded_data: web::Bytes,
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    println!("Received wake shape data");
    let wake_shape: HashMap<String, Vec<f64>> = bincode::deserialize(&encoded_data[..]).unwrap();

    let mut data = data.lock().unwrap();

    data.wake_shape = wake_shape;

    Ok(HttpResponse::Ok().finish())
}

#[post("/clear-data")]
async fn clear_data(
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    let mut data = data.lock().unwrap();

    data.results.clear();

    Ok(HttpResponse::Ok().finish())
}

#[get("/get-forces")]
async fn get_forces(
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    let mut out_data: HashMap<String, Vec<f64>> = HashMap::new();
    out_data.insert("time".to_string(), Vec::new());
    out_data.insert("force_x".to_string(), Vec::new());
    out_data.insert("force_y".to_string(), Vec::new());
    out_data.insert("force_z".to_string(), Vec::new());

    if data.lock().unwrap().results.is_empty() {
        return Ok(HttpResponse::Ok().json(out_data));
    }

    let data = data.lock().unwrap();

    for (index, result) in data.results.iter().enumerate() {
        let forces = result.integrated_forces_sum();

        out_data.get_mut("time").unwrap().push(index as f64);
        out_data.get_mut("force_x").unwrap().push(forces[0]);
        out_data.get_mut("force_y").unwrap().push(forces[1]);
        out_data.get_mut("force_z").unwrap().push(forces[2]);
    }

    Ok(HttpResponse::Ok().json(out_data))
}

#[get("/get-wake-shape")]
async fn get_wake_shape(
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(&data.lock().unwrap().wake_shape))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let data: Arc<Mutex<SimulationData>> = Arc::new(
        Mutex::new(SimulationData::default())
    );

    println!(
        "Starting server. Binding to address {} and port {}",
        &args.bind_address,
        args.port
    );

    HttpServer::new(move || {
        App::new()
        .wrap(
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
            )
            .app_data(web::Data::new(data.clone()))
            .service(update_result_data)
            .service(update_wake_shape)
            .service(clear_data)
            .service(get_forces)
            .service(get_wake_shape)
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind((args.bind_address, args.port))?
    .keep_alive(KeepAlive::Os)
    .run()
    .await
}
