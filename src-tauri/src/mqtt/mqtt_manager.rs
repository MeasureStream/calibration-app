use crate::calibrator::calibrator_manager::FinalCompactReport;
use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{DateTime, Utc};
use flate2::write::GzEncoder;
use flate2::Compression;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::Serialize;
use serde_json;
use std::env;
use std::io::Write;
use tokio::time::{timeout, Duration};
#[derive(Serialize)]
pub struct StepTransmissionDTO {
    pub calib_id: String,
    pub target: f32,
    pub step_summary: Vec<StepSummary>,
    pub step_index: usize,
    pub start_time: DateTime<Utc>,
    pub start_time_dwell: DateTime<Utc>,
    pub ref_readings: Vec<f32>,
    pub sensor_sampling_freq: u16,
    pub sensor_b64: String, // Usiamo una stringa per il JSON
}

#[derive(Serialize, Clone)]
pub struct StepSummary {
    pub target: f32,
    pub minutes: u32,
}

pub async fn publish_calibration_report(report: FinalCompactReport) -> Result<(), String> {
    // 1. Deserializziamo il report completo per poterlo dividere
    let step_summary: Vec<StepSummary> = report
        .steps
        .iter()
        .map(|s| StepSummary {
            target: s.target,
            minutes: s.minutes,
        })
        .collect();

    // 2. Setup variabili ambiente (come prima)
    match dotenvy::dotenv() {
        Ok(path) => println!("[DEBUG] .env caricato correttamente da: {:?}", path),
        Err(_) => println!("[DEBUG] ATTENZIONE: File .env non trovato"),
    }

    let broker_ip = env::var("MQTT_BROKER_IP").map_err(|_| "MQTT_BROKER_IP non settato")?;
    let broker_port = env::var("MQTT_BROKER_PORT")
        .unwrap_or_else(|_| "1883".to_string())
        .parse::<u16>()
        .unwrap_or(1883);
    let user = env::var("MQTT_USER").unwrap_or_default();
    let password = env::var("MQTT_PASSWORD").unwrap_or_default();
    let client_id = env::var("MQTT_CLIENT_ID").unwrap_or_else(|_| "raspi-default".to_string());
    let topic_base = env::var("MQTT_TOPIC").unwrap_or_else(|_| "calibration/reports".to_string());

    // 3. Configurazione MQTT
    let mut mqttoptions = MqttOptions::new(client_id, broker_ip, broker_port);
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(10));
    if !user.is_empty() {
        mqttoptions.set_credentials(user, &password);
    }
    mqttoptions.set_max_packet_size(250_000, 250_000);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // 4. INVIO INDIVIDUALE DI OGNI STEP
    for (index, step) in report.steps.into_iter().enumerate() {
        // --- CONVERSIONE PULITA E TIPATA ---
        let step_dto = StepTransmissionDTO {
            calib_id: report.calibration_id.clone(),
            target: step.target,
            step_index: step.step_index,
            step_summary: step_summary.clone(),
            start_time: step.start_time,
            start_time_dwell: step.start_time_dwell,
            ref_readings: step.ref_readings,
            sensor_b64: BASE64_STANDARD.encode(&step.sensor),
            sensor_sampling_freq: step.sensor_sampling_freq,
        };

        // Ora la serializzazione è diretta e sicura
        let json_payload = serde_json::to_string(&step_dto).map_err(|e| e.to_string())?;

        // 2. COMPRESSIONE GZIP
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(json_payload.as_bytes())
            .map_err(|e| e.to_string())?;
        let payload = encoder.finish().map_err(|e| e.to_string())?;

        let current_topic = format!("{}/gzip", topic_base);

        println!(
            "[DEBUG] Invio Step {} - Dimensione: {} bytes",
            index,
            payload.len()
        );

        // Pubblichiamo
        client
            .publish(current_topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| e.to_string())?;

        // Aspettiamo il PubAck per questo specifico messaggio
        let mut step_success = false;
        let _ = timeout(Duration::from_secs(5), async {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::PubAck(_))) => {
                        println!("[SUCCESS] Step {} confermato!", index);
                        step_success = true;
                        break;
                    }
                    Err(e) => {
                        eprintln!("[ERROR] Errore poll step {}: {:?}", index, e);
                        break;
                    }
                    _ => {} // Ignora altri eventi
                }
            }
        })
        .await;

        if !step_success {
            return Err(format!("Fallimento invio step {}", index));
        }
    }

    Ok(())
}
