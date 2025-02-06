use std::sync::{Arc, Mutex};

use clap::Parser;

use actix_web::{post, get, web, App, HttpResponse, HttpServer, Result, http::KeepAlive};
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
    pub fmu_output_dict: Vec<HashMap<String, f64>>,
    pub wake_shape: HashMap<String, Vec<f64>>
}

#[post("/update-result-data")]
async fn update_result_data(
    encoded_data: web::Bytes,
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    let result: SimulationResult = bincode::deserialize(&encoded_data[..]).unwrap();

    let mut data = data.lock().unwrap();

    data.results.push(result);

    Ok(HttpResponse::Ok().finish())
}

#[post("/update-fmu-output-data")]
async fn update_fmu_output_data(
    encoded_data: web::Bytes,
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    println!("Received FMU output data");

    let fmu_output_data: HashMap<String, f64> = bincode::deserialize(&encoded_data[..]).unwrap();

    let mut data = data.lock().unwrap();

    if data.fmu_output_dict.len() > 0 {
        if data.fmu_output_dict.last().unwrap()["time"] > fmu_output_data["time"] {
            data.fmu_output_dict.clear();
        }
    }
    
    data.fmu_output_dict.push(fmu_output_data);

    Ok(HttpResponse::Ok().finish())
}

#[post("/update-wake-shape")]
async fn update_wake_shape(
    encoded_data: web::Bytes,
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
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
    data.fmu_output_dict.clear();

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

    if data.lock().unwrap().fmu_output_dict.is_empty() {
        return Ok(HttpResponse::Ok().json(out_data));
    }

    let data = data.lock().unwrap();

    for (_, fmu_output) in data.fmu_output_dict.iter().enumerate() {
        out_data.get_mut("time").unwrap().push(fmu_output["time"]);
        out_data.get_mut("force_x").unwrap().push(fmu_output["force_x"]);
        out_data.get_mut("force_y").unwrap().push(fmu_output["force_y"]);
        out_data.get_mut("force_z").unwrap().push(fmu_output["force_z"]);
    }

    Ok(HttpResponse::Ok().json(out_data))
}

#[get("/get-wake-shape")]
async fn get_wake_shape(
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(&data.lock().unwrap().wake_shape))
}

#[get("/get-circulation-distribution")]
async fn get_circulation_distribution(
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    let mut out_data: Vec<HashMap<String, Vec<f64>>> = Vec::new();
    
    if data.lock().unwrap().results.is_empty() {
        return Ok(HttpResponse::Ok().json(out_data));
    }

    let last_data = data.lock().unwrap().results.last().unwrap().clone();

    let nr_of_wings = last_data.nr_of_wings();

    let nr_span_lines_total = last_data.nr_span_lines();
    let nr_span_lines_per_wing = nr_span_lines_total / nr_of_wings;

    for wing_index in 0..last_data.nr_of_wings() {
        let mut current_wing_data: HashMap<String, Vec<f64>> = HashMap::new();

        current_wing_data.insert("ctrl_points_x".to_string(), Vec::new());
        current_wing_data.insert("ctrl_points_y".to_string(), Vec::new());
        current_wing_data.insert("ctrl_points_z".to_string(), Vec::new());
        current_wing_data.insert("circulation_strength".to_string(), Vec::new());

        let start_index = wing_index * nr_span_lines_per_wing;
        let end_index = (wing_index + 1) * nr_span_lines_per_wing;

        for index in start_index..end_index {
            current_wing_data.get_mut("ctrl_points_x").unwrap().push(last_data.ctrl_points[index][0]);
            current_wing_data.get_mut("ctrl_points_y").unwrap().push(last_data.ctrl_points[index][1]);
            current_wing_data.get_mut("ctrl_points_z").unwrap().push(last_data.ctrl_points[index][2]);
            current_wing_data.get_mut("circulation_strength").unwrap().push(last_data.force_input.circulation_strength[index]);
        }

        out_data.push(current_wing_data);
    }

    Ok(HttpResponse::Ok().json(out_data))
}

#[get("/get-average-angles-of-attack")]
async fn get_average_angles_of_attack(
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    let mut out_data: Vec<HashMap<String, Vec<f64>>> = Vec::new();
    
    if data.lock().unwrap().results.is_empty() {
        return Ok(HttpResponse::Ok().json(out_data));
    }

    let data = data.lock().unwrap();

    let last_data = data.results.last().unwrap().clone();

    let nr_wings = last_data.nr_of_wings();
    let nr_span_lines_total = last_data.nr_span_lines();
    let nr_span_lines_per_wing = nr_span_lines_total / nr_wings;

    for wing_index in 0..nr_wings {
        let mut current_wing_data: HashMap<String, Vec<f64>> = HashMap::new();

        current_wing_data.insert("time".to_string(), Vec::new());
        current_wing_data.insert("angles_of_attack".to_string(), Vec::new());

        let start_index = wing_index * nr_span_lines_per_wing;
        let end_index = (wing_index + 1) * nr_span_lines_per_wing;

        for (index, result) in data.results.iter().enumerate() {
            let average_angle_of_attack = result.force_input.angles_of_attack[start_index..end_index].iter()
                .sum::<f64>() / nr_span_lines_per_wing as f64;

            current_wing_data.get_mut("time").unwrap().push(index as f64);
            current_wing_data.get_mut("angles_of_attack").unwrap().push(average_angle_of_attack.to_degrees());
        }

        out_data.push(current_wing_data);
    }

    Ok(HttpResponse::Ok().json(out_data))
}

#[get("/get-angle-of-attack-measurements")]
async fn get_angle_of_attack_measurements(
    data: web::Data<Arc<Mutex<SimulationData>>>
) -> Result<HttpResponse> {
    let mut out_data: Vec<HashMap<String, Vec<f64>>> = Vec::new();

    let data = data.lock().unwrap();
    
    if data.fmu_output_dict.is_empty() {
        let mut current_wing_data: HashMap<String, Vec<f64>> = HashMap::new();

        current_wing_data.insert("time".to_string(), Vec::new());
        current_wing_data.insert("angle_of_attack_measurement".to_string(), Vec::new());

        out_data.push(current_wing_data);

        return Ok(HttpResponse::Ok().json(out_data));
    }

    let nr_wings = if data.results.is_empty() {
        1
    } else {
        data.results.last().unwrap().nr_of_wings()
    };

    for wing_index in 0..nr_wings {
        let mut current_wing_data: HashMap<String, Vec<f64>> = HashMap::new();

        current_wing_data.insert("time".to_string(), Vec::new());
        current_wing_data.insert("angle_of_attack_measurement".to_string(), Vec::new());

        let key = format!("angle_of_attack_measurement_{}", wing_index + 1);

        for (_, fmu_dict) in data.fmu_output_dict.iter().enumerate() {
            current_wing_data.get_mut("time").unwrap().push(fmu_dict["time"]);

            if !fmu_dict.contains_key(&key) {
                println!("Key {} not found in FMU output data", key);
            }
            current_wing_data.get_mut("angle_of_attack_measurement").unwrap().push(fmu_dict[&key]);
        }

        out_data.push(current_wing_data);
    }

    Ok(HttpResponse::Ok().json(out_data))
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
            .service(update_fmu_output_data)
            .service(update_wake_shape)
            .service(clear_data)
            .service(get_forces)
            .service(get_wake_shape)
            .service(get_circulation_distribution)
            .service(get_average_angles_of_attack)
            .service(get_angle_of_attack_measurements)
    })
    .bind((args.bind_address, args.port))?
    .keep_alive(KeepAlive::Os)
    .run()
    .await
}
