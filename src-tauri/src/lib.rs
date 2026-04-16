mod calibrator;
mod fluke;
mod measurement_unit;
mod mqtt;
mod serial;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn _start_thermal_calibration(
    app: tauri::AppHandle,
    steps: Vec<calibrator::calibrator_manager::CalibrationStep>,
) -> Result<(), String> {
    calibrator::calibrator_manager::start_thermal_calibration(app, steps).await
}

#[tauri::command]
async fn _stop_thermal_calibration() {
    calibrator::calibrator_manager::stop_thermal_calibration()
}

#[tauri::command]
async fn get_muinfo(
) -> Result<measurement_unit::measurement_unit_processor::MeasurementUnitDTO, String> {
    measurement_unit::measurement_unit_processor::run_sync_process(65537)
        .await
        .map_err(|e| e.to_string()) // Converte l'errore in stringa per il frontend
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            _start_thermal_calibration,
            _stop_thermal_calibration,
            get_muinfo
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
