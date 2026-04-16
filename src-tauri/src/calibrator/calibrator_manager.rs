use crate::fluke::fluke_manager::Fluke9142;
use crate::mqtt::mqtt_manager::publish_calibration_report;
use crate::serial::serial_manager::SerialDevice;
use chrono::Local;
use core::f32;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(serde::Deserialize, Clone)]
pub struct CalibrationStep {
    pub target_value: f32,
    pub tempo_per_step: u32, // in minuti
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct CalibrationPayload {
    pub timestamp: u64,
    pub current_temp_fluke: f32,
    pub current_temp_sensor: Vec<f32>,
    pub is_stable: bool,
    pub current_step: usize,
    pub total_steps: usize,
    pub elapsed_time: u32,
    pub total_time: u32,
    pub status: String, // "RAMPA", "DWELL"
}

#[derive(serde::Serialize, Clone)]
pub struct FinalCalibrationReport {
    pub calibration_id: String,
    pub calibrator_id: u32,
    pub mu_id: u32,
    pub sensor_id: u32,
    pub steps: Vec<(f32, u32)>, // (Target, Minuti)
    pub reference_temperature_samples: Vec<RefSample>,
    pub sensor_raw_samples: Vec<RawSample>,
}

#[derive(serde::Serialize, Clone)]
pub struct RefSample {
    pub index_step: usize,
    pub timestamp: String, // ISO 8601
    pub target: f32,
    pub reading: f32,
    pub stable_hw: bool,
}

#[derive(serde::Serialize, Clone)]
pub struct RawSample {
    pub index_step: usize,
    pub timestamp: String, // ISO 8601
    pub value_hex: String,
}

static IS_RUNNING: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

static SHARED_SENSOR_PORT: Lazy<Arc<Mutex<Option<SerialDevice>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

pub fn get_or_init_sensor_port() -> Result<Arc<Mutex<Option<SerialDevice>>>, String> {
    let mut guard = SHARED_SENSOR_PORT.lock().map_err(|_| "Lock failed")?;

    // Se la porta non esiste o è stata chiusa (None), la inizializziamo
    if guard.is_none() {
        println!("Tentativo di apertura seriale e Handshake...");
        let mut port = SerialDevice::open_by_vid_pid(0x1a86, 0x7523, 115200, 0)
            .map_err(|e| format!("Errore hardware: {}", e))?;

        // Eseguiamo l'handshake SOLO QUI (una volta sola)
        port.connect_mu(0x00)
            .map_err(|e| format!("MU non risponde: {}", e))?;

        *guard = Some(port);
    }

    Ok(SHARED_SENSOR_PORT.clone())
}

pub async fn start_thermal_calibration(
    app: AppHandle,
    steps: Vec<CalibrationStep>,
) -> Result<(), String> {
    let port_arc = get_or_init_sensor_port()?;

    IS_RUNNING.store(true, Ordering::SeqCst);

    let ref_samples = Arc::new(Mutex::new(Vec::<RefSample>::new()));
    let raw_samples = Arc::new(Mutex::new(Vec::<RawSample>::new()));
    let current_step_idx = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Copia degli step per il report finale (visto che steps verrà consumato dal ciclo)
    let steps_for_report: Vec<(f32, u32)> = steps
        .iter()
        .map(|s| (s.target_value, s.tempo_per_step))
        .collect();

    // Canale unico per il sensore -> Master
    let (tx_sensor, rx_sensor) = mpsc::channel::<Vec<u8>>();

    // --- THREAD 1: LETTURA SENSORE ---
    let stop_sensor = IS_RUNNING.clone();
    let port_arc_thread = port_arc.clone();
    let raw_samples_thread1 = raw_samples.clone(); // Clone per il Thread 1
    let step_idx_raw = current_step_idx.clone();

    std::thread::spawn(move || {
        let mut local_raw_buffer = Vec::new();
        let mut guard = port_arc_thread.lock().unwrap();

        if let Some(ref mut sensor_port) = *guard {
            if let Err(e) = sensor_port.start_mu_calibration(0x04, 100, 128) {
                eprintln!("Errore avvio streaming: {}", e);
                return;
            }

            while stop_sensor.load(Ordering::SeqCst) {
                let data = sensor_port.read_available();
                if !data.is_empty() {
                    local_raw_buffer.push(RawSample {
                        index_step: step_idx_raw.load(Ordering::SeqCst),
                        timestamp: Local::now().to_rfc3339(),
                        value_hex: data.iter().map(|b| format!("{:02x}", b)).collect(),
                    });
                    let _ = tx_sensor.send(data);
                }
                std::thread::sleep(Duration::from_millis(10));
            }

            // Versamento finale nel Mutex
            let mut guard_samples = raw_samples_thread1.lock().unwrap();
            *guard_samples = local_raw_buffer;

            let _ = sensor_port.stop_mu_calibration(0x01, 0x04);
        }
    });

    // --- THREAD 2: MASTER ---
    let ref_samples_thread2 = ref_samples.clone(); // Clone per il Thread 2
    let raw_samples_thread2 = raw_samples.clone(); // Clone per l'assemblaggio finale nel Thread 2
    let step_idx_master = current_step_idx.clone();
    let stop_master = IS_RUNNING.clone();
    let _total_steps = steps.len();

    std::thread::spawn(move || {
        let mut local_ref_buffer = Vec::new();
        let mut fluke = Fluke9142::new().expect("Impossibile connettersi al Fluke");
        fluke.start_heating();

        for (index, step) in steps.into_iter().enumerate() {
            step_idx_master.store(index, Ordering::SeqCst);
            fluke.set_temperature(step.target_value);

            let mut is_stable_reached = false;
            let mut start_dwell: Option<std::time::Instant> = None;
            let total_dwell_secs = (step.tempo_per_step * 60) as u64;

            while stop_master.load(Ordering::SeqCst) {
                let f_temp = fluke.read_temperature().unwrap_or(0.0);
                let f_stable = fluke.is_stable();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                // Svuota canale sensore per la UI
                let mut all_data = Vec::new();
                while let Ok(p) = rx_sensor.try_recv() {
                    all_data.extend(p);
                }
                let s_temp = if !all_data.is_empty() {
                    process_ntc_packet(all_data)
                } else {
                    vec![]
                };

                let mut status = "RAMPA".to_string();
                let mut elapsed = 0u32;

                if !is_stable_reached {
                    if f_stable {
                        is_stable_reached = true;
                        start_dwell = Some(std::time::Instant::now());
                        status = "DWELL".to_string();
                    }
                } else if let Some(start) = start_dwell {
                    elapsed = start.elapsed().as_secs() as u32;
                    status = "DWELL".to_string();

                    // Salvataggio campioni durante DWELL
                    local_ref_buffer.push(RefSample {
                        index_step: index,
                        timestamp: Local::now().to_rfc3339(),
                        target: step.target_value,
                        reading: f_temp,
                        stable_hw: f_stable,
                    });

                    if elapsed >= total_dwell_secs as u32 {
                        break;
                    }
                }

                app.emit(
                    "calibration-update",
                    &CalibrationPayload {
                        timestamp: now,
                        current_temp_fluke: f_temp,
                        current_temp_sensor: s_temp,
                        is_stable: f_stable,
                        current_step: index + 1,
                        total_steps: _total_steps,
                        elapsed_time: elapsed,
                        total_time: step.tempo_per_step * 60,
                        status,
                    },
                )
                .unwrap();

                std::thread::sleep(Duration::from_millis(1000));
            }
            if !stop_master.load(Ordering::SeqCst) {
                break;
            }
        }

        fluke.stop_heating();
        IS_RUNNING.store(false, Ordering::SeqCst);

        // --- ASSEMBLAGGIO FINALE ---
        {
            let mut guard = ref_samples_thread2.lock().unwrap();
            *guard = local_ref_buffer;
        }

        let calib_id = format!("calib-1-1-{}", Local::now().format("%Y-%m-%dT%H:%M:%S"));

        let final_report = FinalCalibrationReport {
            calibration_id: calib_id,
            calibrator_id: 1,
            mu_id: 1,
            sensor_id: 1,
            steps: steps_for_report, // Ora popolato correttamente
            reference_temperature_samples: ref_samples_thread2.lock().unwrap().clone(),
            sensor_raw_samples: raw_samples_thread2.lock().unwrap().clone(),
        };

        let report_to_send = serde_json::to_string(&final_report).unwrap();
        println!("Calibrazione completata. Report generato.");
        // json_final a MQTT
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            if let Err(e) = publish_calibration_report(report_to_send).await {
                eprintln!("Fallimento invio finale: {}", e);
            }
        });
    });

    Ok(())
}
pub fn stop_thermal_calibration() {
    IS_RUNNING.store(false, Ordering::SeqCst);
}

fn process_ntc_packet(packet: Vec<u8>) -> Vec<f32> {
    const R0: f32 = 10000.0; // Esempio: 10k Ohm (Valore di riferimento NTC)
    const B: f32 = 4190.0; // Beta coefficient
    const T0: f32 = 298.15; // 25°C in Kelvin
    const ADC_MAX: f32 = 4095.0; // Per un ADC a 12 bit

    packet
        .chunks_exact(2)
        .map(|chunk| {
            // Lettura Big Endian (corretto, allineato a Python)
            let raw_val = u16::from_le_bytes([chunk[0], chunk[1]]);

            // 1. Protezione contro la divisione per zero
            if raw_val == 0 {
                return -273.15; // O un valore di errore predefinito
            }

            // 2. Calcolo Resistenza NTC (allineato a Python)
            // Rntc = R0 * ( (ADC_MAX / sample) - 1 )
            let r_ntc = R0 * (ADC_MAX / raw_val as f32 - 1.0);

            // 3. Equazione di Steinhart-Hart (Beta)
            let ln_r = (r_ntc / R0).ln();
            let t_kelvin = 1.0 / (1.0 / T0 + ln_r / B);

            // 4. Conversione in Celsius (come nel print di Python)
            t_kelvin - 273.15
        })
        .collect()
}
