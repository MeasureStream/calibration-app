use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SensorTemplateDTO {
    pub model_name: String,
    pub r#type: String,
    pub unit: String,
    pub conversion: Option<serde_json::Value>,
    pub properties: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SensorDTO {
    pub id: i64,
    pub model_name: String,
    pub sensor_index: i32,
    pub phys_val: f32,
    pub elec_val: f32,
    pub sampling_f: f32,
    // Coefficienti per la taratura
    pub coeff_a: Option<f32>,
    pub coeff_b: Option<f32>,
    pub coeff_c: Option<f32>,
    pub coeff_d: Option<f32>,
    pub cal_date: Option<String>,
    // Aggancio al template ricevuto dal server
    pub sensor_template: SensorTemplateDTO,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MeasurementUnitDTO {
    pub id: i64,
    pub extended_id: i64,
    pub local_id: i32,
    pub model: i32,
    pub control_unit_id: i64,
    pub sensors: Vec<SensorDTO>,
}

#[derive(Deserialize, Debug)]
pub struct KeycloakToken {
    pub access_token: String,
}

pub async fn get_access_token(
    client_id: &str,
    client_secret: &str,
    realm_url: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let token_url = format!("{}/protocol/openid-connect/token", realm_url);

    println!("DEBUG: Invio richiesta a {}", token_url);

    let params = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];

    let response = client.post(&token_url).form(&params).send().await?;

    let status = response.status();
    println!("DEBUG: Keycloak Status: {}", status);

    if status.is_success() {
        let token_data: KeycloakToken = response.json().await?;
        println!("JSON Token decodificato correttamente!"); // <-- AGGIUNGI QUESTO
        Ok(token_data.access_token)
    } else {
        // Leggiamo il testo dell'errore per capire se è "Invalid Client", "Unknown Realm", ecc.
        let error_text = response.text().await?;
        println!("Errore Keycloak DETTAGLIATO: {}", error_text);
        Err(format!("Errore Keycloak ({}): {}", status, error_text).into())
    }
}
async fn get_or_create_mu(
    client: &reqwest::Client,
    mu_to_check: &MeasurementUnitDTO,
    token: &str,
) -> Result<MeasurementUnitDTO, Box<dyn std::error::Error>> {
    // Usa HTTPS e assicurati che l'URL sia corretto
    // aggiungi ip diretto a tailascale measurestream e metti ports 8080:8081
    let base_url = "https://www.christiandellisanti.uk/API/measurementunits";
    let auth_value = format!("Bearer {}", token);
    //println!("DEBUG: Authorization Header: {}...", &auth_value);

    // Costruiamo l'URL con il parametro networkId
    let url_with_query = format!("{}?extendedId={}", base_url, mu_to_check.extended_id);

    println!("DEBUG: Invio GET a: {}", url_with_query);

    let response = client
        .get(&url_with_query)
        .header("Authorization", &auth_value)
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = response.status();
    println!("DEBUG: Risposta API Status: {}", status);

    if status.is_success() {
        let body_text = response.text().await?;
        //println!("DEBUG: Body ricevuto: {}", body_text); // Decommenta per vedere il JSON grezzo

        let units: Vec<MeasurementUnitDTO> = serde_json::from_str(&body_text)?;
        println!("DEBUG: Numero unità trovate: {}", units.len());

        if let Some(existing_unit) = units.first() {
            println!("Unità esistente trovata: ID {}", existing_unit.id);
            Ok(existing_unit.clone())
        } else {
            println!(
                "Errore MU non trovata extended_id: {} non esitente",
                mu_to_check.extended_id
            );
            Err(format!(
                "Errore MU non trovata extended_id: {} non esitente",
                mu_to_check.extended_id
            )
            .into())
        }
    } else {
        let err_text = response.text().await?;
        println!("Errore API (GET): {} - {}", status, err_text);
        Err(format!("Errore API: {}", status).into())
    }
    /*
        // 2. Se arriviamo qui, l'array era vuoto. Creiamo la MU.
        println!("DEBUG: MU non trovata, invio POST di creazione...");

        let response_post = client
            .post(base_url) // Senza query string per la POST
            .header("Authorization", &auth_value)
            .json(mu_to_check)
            .send()
            .await?;

        let status_post = response_post.status();
        println!("DEBUG: Status POST: {}", status_post);

        if status_post.is_success() || status_post == reqwest::StatusCode::CREATED {
            let new_unit: MeasurementUnitDTO = response_post.json().await?;
            println!("Nuova unità creata con ID: {}", new_unit.id);
            Ok(new_unit)
        } else {
            let err_text = response_post.text().await?;
            println!("Errore durante la POST: {}", err_text);
            Err(format!("Errore creazione MU: {}", status_post).into())
        }
    */
}
pub async fn run_sync_process(
    mu_serial_id: i64,
) -> Result<MeasurementUnitDTO, Box<dyn std::error::Error>> {
    // Prova a caricare il .env
    match dotenvy::dotenv() {
        Ok(path) => println!("File .env caricato da: {:?}", path),
        Err(e) => println!("Attenzione: Impossibile caricare il file .env: {}", e),
    }

    // Lettura e stampa delle variabili
    let client_id = env::var("KC_CLIENT_ID").unwrap_or_else(|_| "NON DEFINITO".to_string());
    let client_secret = env::var("KC_CLIENT_SECRET").unwrap_or_else(|_| "NON DEFINITO".to_string());
    let realm_url = env::var("KC_REALM_URL").unwrap_or_else(|_| "NON DEFINITO".to_string());

    println!("--- DEBUG ENV ---");
    println!("KC_CLIENT_ID:  {}", client_id);
    println!("KC_REALM_URL:  {}", realm_url);
    // Mostra solo i primi 4 caratteri del secret per sicurezza
    if client_secret != "NON DEFINITO" && client_secret.len() > 4 {
        println!("KC_SECRET:     {}****", &client_secret[..4]);
    } else {
        println!("KC_SECRET:     {}", client_secret);
    }
    println!("-----------------");

    // Se una variabile è "NON DEFINITO", l'errore ? qui sotto fermerà l'esecuzione
    let client_id = env::var("KC_CLIENT_ID")?;
    let client_secret = env::var("KC_CLIENT_SECRET")?;
    let realm_url = env::var("KC_REALM_URL")?;

    println!("Tentativo di ottenere il token da Keycloak...");
    let token = get_access_token(&client_id, &client_secret, &realm_url).await?;
    println!("Token ottenuto!");

    //TODO("Questo è da cambiare l'applicazione deve controllare se esiste, se non esiste deve
    //creare l'entità sul server, perché il server è l'unico che ha la conoscenza dei template")
    let mu_to_check = MeasurementUnitDTO {
        id: 0,
        extended_id: mu_serial_id,
        local_id: 0,
        model: 0,
        control_unit_id: 0,
        sensors: vec![],
    };

    let client = reqwest::Client::new();

    println!(
        "Tentativo get_or_create per extended_id: {}",
        mu_to_check.extended_id
    );
    let mu = get_or_create_mu(&client, &mu_to_check, &token).await?;
    println!("Processo completato con successo per MU ID: {}", mu.id);

    Ok(mu)
}
