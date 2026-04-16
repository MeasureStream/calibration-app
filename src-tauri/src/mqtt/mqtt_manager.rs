use dotenvy::dotenv;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::env;

pub async fn publish_calibration_report(json_report: String) -> Result<(), String> {
    // Carica il file .env
    match dotenvy::dotenv() {
        Ok(path) => println!(".env caricato correttamente da: {:?}", path),
        Err(_) => println!(
            "ATTENZIONE: File .env non trovato nella cartella: {:?}",
            std::env::current_dir().unwrap()
        ),
    }

    // Recupera le variabili d'ambiente
    let broker_ip = env::var("MQTT_BROKER_IP").map_err(|_| "MQTT_BROKER_IP non settato in .env")?;
    let broker_port = env::var("MQTT_BROKER_PORT")
        .unwrap_or_else(|_| "1883".to_string())
        .parse::<u16>()
        .map_err(|_| "Porta MQTT non valida")?;
    let user = env::var("MQTT_USER").unwrap_or_default();
    let password = env::var("MQTT_PASSWORD").unwrap_or_default();
    let client_id = env::var("MQTT_CLIENT_ID").unwrap_or_else(|_| "raspi-default".to_string());
    let topic = env::var("MQTT_TOPIC").unwrap_or_else(|_| "default/topic".to_string());

    // Configurazione MQTT
    let mut mqttoptions = MqttOptions::new(client_id, broker_ip, broker_port);
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

    if !user.is_empty() {
        mqttoptions.set_credentials(user, password);
    }

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Event loop manager
    tokio::spawn(async move { while let Ok(_) = eventloop.poll().await {} });

    // Invio
    match client
        .publish(topic, QoS::AtLeastOnce, false, json_report)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Errore MQTT: {:?}", e)),
    }
}
