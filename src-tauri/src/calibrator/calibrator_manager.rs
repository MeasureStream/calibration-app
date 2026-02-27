use crate::fluke::fluke_manager::Fluke9142;
use crate::serial::serial_manager::SerialDevice;
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

static IS_RUNNING: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

static SHARED_SENSOR_PORT: Lazy<Arc<Mutex<Option<SerialDevice>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

pub fn get_or_init_sensor_port() -> Result<Arc<Mutex<Option<SerialDevice>>>, String> {
    let mut guard = SHARED_SENSOR_PORT.lock().map_err(|_| "Lock failed")?;

    // Se la porta non esiste o è stata chiusa (None), la inizializziamo
    if guard.is_none() {
        println!("Tentativo di apertura seriale e Handshake...");
        let mut port = SerialDevice::open_by_vid_pid(0x10c4, 0xea60, 115200, 0)
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

    // Canale unico per il sensore -> Master
    let (tx_sensor, rx_sensor) = mpsc::channel::<Vec<u8>>();

    // --- THREAD 1: LETTURA SENSORE (Produttore ad alta velocità) ---
    let stop_sensor = IS_RUNNING.clone();
    let port_arc_thread = port_arc.clone();

    std::thread::spawn(move || {
        let mut guard = port_arc_thread.lock().unwrap();

        if let Some(ref mut sensor_port) = *guard {
            if let Err(e) = sensor_port.start_mu_calibration(0x04, 100, 128) {
                eprintln!("Errore avvio streaming: {}", e);
                return;
            }

            while stop_sensor.load(Ordering::SeqCst) {
                // 3. Leggiamo i dati
                let data = sensor_port.read_available();

                if !data.is_empty() {
                    // Se riceviamo un errore di IO (es. USB staccata),
                    // read_available dovrebbe idealmente restituire un errore o un buffer vuoto.
                    // Se vuoi gestire il distacco fisico qui:
                    let _ = tx_sensor.send(data);
                }

                std::thread::sleep(Duration::from_millis(10));
            }

            let _ = sensor_port.stop_mu_calibration(0x01, 0x04);
            println!("Streaming MU terminato correttamente.");
        } else {
            eprintln!("Errore: La porta seriale non è inizializzata.");
        }
    });

    let _total_steps = steps.len();

    // --- THREAD 2: MASTER (Regolatore, Assemblatore e Notificatore) ---
    let stop_master = IS_RUNNING.clone();
    std::thread::spawn(move || {
        let mut fluke = Fluke9142::new().expect("Impossibile connettersi al Fluke");
        fluke.start_heating();

        for (index, step) in steps.into_iter().enumerate() {
            fluke.set_temperature(step.target_value);

            let mut is_stable_reached = false;
            let mut start_dwell: Option<std::time::Instant> = None;
            let total_dwell_secs = (step.tempo_per_step * 60) as u64;

            while stop_master.load(Ordering::SeqCst) {
                // 1. Lettura Fluke + Timestamp
                let f_temp = fluke.read_temperature().unwrap_or(0.0);
                let f_stable = fluke.is_stable();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let mut all_data = Vec::new();
                while let Ok(p) = rx_sensor.try_recv() {
                    all_data.extend(p);
                }

                let s_temp = if !all_data.is_empty() {
                    process_ntc_packet(all_data)
                } else {
                    vec![]
                };

                // 3. Gestione Stati e Tempo
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
                    if elapsed >= total_dwell_secs as u32 {
                        break;
                    }
                }

                // 4. Creazione Payload Unificato con Timestamp
                let payload = CalibrationPayload {
                    timestamp: now, // Timestamp univoco per il sync
                    current_temp_fluke: f_temp,
                    current_temp_sensor: s_temp,
                    is_stable: f_stable,
                    current_step: index + 1,
                    total_steps: _total_steps,
                    elapsed_time: elapsed,
                    total_time: step.tempo_per_step * 60,
                    status,
                };
                //println!("{:?}", payload);

                // 5. Invio UI e Log (Kafka/File)
                app.emit("calibration-update", &payload).unwrap();

                if payload.status == "DWELL" {
                    // Esegui qui il salvataggio ad ogni secondo di dwell
                    // save_to_log(&payload);
                }

                std::thread::sleep(Duration::from_millis(1000));
            }
            if !stop_master.load(Ordering::SeqCst) {
                break;
            }
        }
        fluke.stop_heating();
        IS_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok(())
}

pub fn stop_thermal_calibration() {
    IS_RUNNING.store(false, Ordering::SeqCst);
}

fn process_ntc_packet(packet: Vec<u8>) -> Vec<f32> {
    packet
        .chunks_exact(2)
        .map(|chunk| {
            // Uniamo i due byte in un intero a 16 bit (Big Endian come da tua tabella precedente)
            let raw_val = u16::from_be_bytes([chunk[0], chunk[1]]);
            let l = (3.3 / (raw_val as f32) - 1.0).ln();
            1.0 / (l / 4190.0 + 1.0 / 298.15)
        })
        .collect()
}
