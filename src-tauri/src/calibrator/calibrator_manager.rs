use crate::fluke::fluke_manager::Fluke9142;
use crate::mqtt::mqtt_manager::publish_calibration_report;
use crate::serial::serial_manager::SerialDevice;
use chrono::{DateTime, Local, Utc};
use core::f32;
use once_cell::sync::Lazy;
use std::collections::VecDeque;
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
    pub current_temp_sensor: f32,
    pub samples_count: usize,
    pub is_stable: bool,
    pub current_step: usize,
    pub total_steps: usize,
    pub elapsed_time: u32,
    pub total_time: u32,
    pub status: String,
}
#[derive(serde::Serialize)]
pub struct StepData {
    pub target: f32,
    pub step_index: usize,
    pub minutes: u32,
    pub start_time: DateTime<Utc>,
    pub start_time_dwell: DateTime<Utc>,
    // Qui mettiamo le liste piatte per questo specifico step
    pub ref_readings: Vec<f32>, // 1 lettura al secondo
    pub sensor: Vec<u8>,        // N letture al secondo (es. 100Hz)
    pub sensor_sampling_freq: u16,
}

#[derive(serde::Serialize)]
pub struct FinalCompactReport {
    pub calibration_id: String,
    pub sensor_id: u32,
    pub sensor_freq_hz: u32,
    pub steps: Vec<StepData>, // Ogni elemento è un blocco Target + Dati
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
        port.connect_mu(0x01)
            .map_err(|e| format!("MU non risponde: {}", e))?;

        *guard = Some(port);
    }

    Ok(SHARED_SENSOR_PORT.clone())
}

pub async fn start_thermal_calibration(
    app: AppHandle,
    steps: Vec<CalibrationStep>,
) -> Result<(), String> {
    let port_arc = get_or_init_sensor_port().map_err(|e| format!("Hardware non pronto: {}", e))?;
    let mut fluke = Fluke9142::new().map_err(|e| format!("Fluke non trovato: {}", e))?;

    IS_RUNNING.store(true, Ordering::SeqCst);

    let app_sensor = app.clone();
    let app_master = app.clone();

    let freq_campionamento = 1; //Hz

    let current_step_idx = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stop_signal = IS_RUNNING.clone();

    // Canale per i byte grezzi: Sensore -> Master
    let (tx_sensor, rx_sensor) = mpsc::channel::<Vec<u8>>();

    // --- THREAD 1: LETTURA SENSORE (Ultra-Light) ---
    let port_arc_thread = port_arc.clone();
    let stop_sensor = stop_signal.clone();
    std::thread::spawn(move || {
        let mut guard = port_arc_thread.lock().unwrap();
        if let Some(ref mut sensor_port) = *guard {
            if let Err(e) = sensor_port.start_mu_calibration(0x04, freq_campionamento, 128) {
                eprintln!("Errore avvio streaming: {}", e);
                handle_fatal_error(&app_sensor, "Errore Avvio MU");
                return;
            }

            while stop_sensor.load(Ordering::SeqCst) {
                match sensor_port.read_packet(128) {
                    Ok(data) => {
                        let _ = tx_sensor.send(data);
                    }
                    Err(e) => {
                        // Se è un timeout, semplicemente riprova il ciclo
                        if e.contains("timed out") {
                            continue;
                        }
                        eprintln!("Errore critico seriale: {}", e);
                        break;
                    }
                }
            }
            let res = sensor_port.stop_mu_calibration(0x00);
            println!("Esito stop: {:?}", res);
        }
    });

    // --- THREAD 2: MASTER (Logica e Assemblaggio) ---
    let stop_master = stop_signal.clone();
    let total_steps = steps.len();

    std::thread::spawn(move || {
        let mut final_steps_reports = Vec::new();
        //let mut fluke = Fluke9142::new().expect("Errore Apertura Fluke9142");
        let start_time = chrono::Utc::now();
        fluke.start_heating();

        for (index, step) in steps.into_iter().enumerate() {
            current_step_idx.store(index, Ordering::SeqCst);
            fluke.set_temperature(step.target_value);

            let mut current_step_ref = Vec::new();
            //let mut current_step_raw = Vec::new();
            let mut byte_accumulator = Vec::new();
            let mut is_stable_reached = false;
            let mut start_dwell: Option<std::time::Instant> = None;
            let total_dwell_secs = (step.tempo_per_step * 60) as u64;
            let mut start_time_dwell = None;

            // BUFFER: conterrà le medie al secondo degli ultimi 60 secondi
            let mut second_averages_buffer: VecDeque<f32> = VecDeque::with_capacity(60);
            let mut last_receive_time = std::time::Instant::now();

            while stop_master.load(Ordering::SeqCst) {
                let f_temp = match fluke.read_temperature() {
                    Some(t) => t,

                    None => {
                        handle_fatal_error(&app_master, "Persa connessione con Fluke9142");

                        return; // Esce dal thread
                    }
                };
                let f_stable = fluke.is_stable();

                // 1. Prendi i 100 campioni del secondo attuale e fai la media
                let mut samples_this_second = Vec::new();
                while let Ok(data) = rx_sensor.try_recv() {
                    let now = std::time::Instant::now();
                    let _duration = now.duration_since(last_receive_time);

                    //println!("DEBUG: Tempo dall'ultima ricezione: {}ms | Campioni accumulati nel buffer: {}", _duration.as_millis(), data.len());

                    last_receive_time = now;

                    let readings = process_ntc_packet(&data);
                    samples_this_second.extend(readings);
                    println!("RAW HEX: {:02x?}", data);

                    if is_stable_reached {
                        byte_accumulator.extend(data);
                    }
                }

                // Calcolo della media del secondo attuale
                if !samples_this_second.is_empty() {
                    let avg_second =
                        samples_this_second.iter().sum::<f32>() / samples_this_second.len() as f32;

                    if second_averages_buffer.len() >= 60 {
                        second_averages_buffer.pop_front();
                    }
                    second_averages_buffer.push_back(avg_second);
                }

                // 2. Calcolo Deviazione Standard sulla finestra di 60 medie
                let sensor_std_dev =
                    calculate_std_dev(second_averages_buffer.make_contiguous()).unwrap_or(999.0);

                let mut status = "RAMPA".to_string();
                let mut elapsed = 0u32;

                // 3. Controllo Stabilità combinata
                if !is_stable_reached {
                    // Passa a DWELL se Fluke è OK e la variazione delle medie al secondo è minima (< 0.1)
                    // Richiediamo almeno 30/40 secondi di dati per una statistica significativa
                    if f_stable && sensor_std_dev < 0.1 && second_averages_buffer.len() >= 45 {
                        is_stable_reached = true;
                        start_dwell = Some(std::time::Instant::now());
                        start_time_dwell = Some(chrono::Utc::now());
                        current_step_ref.clear();
                        status = "DWELL".to_string();
                    }
                } else if let Some(start) = start_dwell {
                    elapsed = start.elapsed().as_secs() as u32;
                    status = "DWELL".to_string();
                    current_step_ref.push(f_temp);
                }

                let samples_count = samples_this_second.len();
                let current_avg = if samples_count > 0 {
                    samples_this_second.iter().sum::<f32>() / samples_count as f32
                } else {
                    // Se non abbiamo ricevuto nulla, prendiamo l'ultima media nota o 0.0
                    second_averages_buffer.back().cloned().unwrap_or(0.0)
                };

                println!("ARRIVATO: {}", current_avg);

                // 4. Update UI
                let _ = app.emit(
                    "calibration-update",
                    &CalibrationPayload {
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                        current_temp_fluke: f_temp,
                        current_temp_sensor: current_avg,
                        samples_count, // Feedback immediato sulla salute della seriale
                        is_stable: is_stable_reached,
                        current_step: index + 1,
                        total_steps,
                        elapsed_time: elapsed,
                        total_time: step.tempo_per_step * 60,
                        status,
                    },
                );

                if is_stable_reached && elapsed >= total_dwell_secs as u32 {
                    break;
                }

                std::thread::sleep(Duration::from_secs(1));
            }
            if !stop_master.load(Ordering::SeqCst) {
                break;
            }

            // Salvataggio pacchetto Step

            final_steps_reports.push(StepData {
                target: step.target_value,
                start_time,
                start_time_dwell: start_time_dwell.expect("START TIME DWELL MUST BE NOT NULL"),
                step_index: index,
                minutes: step.tempo_per_step,
                ref_readings: current_step_ref,
                sensor: byte_accumulator,
                sensor_sampling_freq: freq_campionamento,
            });
        }

        fluke.stop_heating();
        IS_RUNNING.store(false, Ordering::SeqCst);

        // 4. Report Finale
        let calib_id = format!("calib-1-1-{}", Local::now().format("%Y%m%dT%H%M%S"));
        let final_report = FinalCompactReport {
            calibration_id: calib_id,
            sensor_id: 1,
            sensor_freq_hz: 100,
            steps: final_steps_reports,
        };

        //let report_to_send = serde_json::to_string(&final_report).unwrap();

        // Invio MQTT
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            if let Err(e) = publish_calibration_report(final_report).await {
                eprintln!("Fallimento invio finale: {}", e);
            }
        });
    });

    Ok(())
}
pub fn stop_thermal_calibration() {
    IS_RUNNING.store(false, Ordering::SeqCst);
}

fn process_ntc_packet(packet: &[u8]) -> Vec<f32> {
    const R0: f32 = 10000.0;
    const B: f32 = 4190.0;
    const T0: f32 = 298.15;
    const ADC_MAX: f32 = 4095.0;

    let mut results = Vec::new();

    // Usiamo un ciclo manuale invece di chunks_exact per avere più controllo
    for chunk in packet.chunks_exact(2) {
        // Proviamo a leggere. Se raw_val > ADC_MAX, significa che siamo disallineati
        // (il byte basso è finito al posto di quello alto)
        let mut raw_val = u16::from_be_bytes([chunk[0], chunk[1]]);

        // RECUPERO: Se il valore è assurdo, proviamo l'altra endianness (riallineamento software)
        if raw_val > ADC_MAX as u16 {
            raw_val = u16::from_le_bytes([chunk[0], chunk[1]]);
        }

        // Se dopo il tentativo è ancora fuori range o zero, scartiamo il campione
        if raw_val == 0 || raw_val >= ADC_MAX as u16 {
            continue;
        }

        let r_ntc = R0 * (ADC_MAX / raw_val as f32 - 1.0);

        // Protezione logaritmo: r_ntc deve essere > 0
        if r_ntc <= 0.0 {
            continue;
        }

        let ln_r = (r_ntc / R0).ln();
        let t_kelvin = 1.0 / (1.0 / T0 + ln_r / B);
        let t_celsius = t_kelvin - 273.15;

        // Un'ultima protezione: se il sensore segna temperature assurde (es. > 200°C o < -50°C)
        // probabilmente è ancora rumore di allineamento
        if t_celsius > -50.0 && t_celsius < 200.0 {
            results.push(t_celsius);
        }
    }

    results
}
// Una funzione helper per chiudere tutto e avvisare il frontend
fn handle_fatal_error(app: &AppHandle, msg: &str) {
    eprintln!("FATAL: {}", msg);
    IS_RUNNING.store(false, Ordering::SeqCst);
    let _ = app.emit("calibration-error", msg);
}

fn calculate_std_dev(data: &[f32]) -> Option<f32> {
    let count = data.len();
    if count < 2 {
        return None;
    }

    let mean = data.iter().sum::<f32>() / count as f32;
    let variance = data
        .iter()
        .map(|value| {
            let diff = mean - value;
            diff * diff
        })
        .sum::<f32>()
        / count as f32;

    Some(variance.sqrt())
}

pub fn discover_hardware_id() -> Result<i64, String> {
    let port_arc = get_or_init_sensor_port().map_err(|e| e.to_string())?;
    let mut guard = port_arc.lock().map_err(|_| "Porta occupata")?;

    if let Some(ref mut device) = *guard {
        // Se l'UID è già in memoria lo restituisce, altrimenti interroga la MU
        if device.extended_uid == 0 {
            device.connect_mu(0x01).map_err(|e| e.to_string())?;
        }
        Ok(device.extended_uid as i64)
    } else {
        Err("Dispositivo non inizializzato".into())
    }
}
