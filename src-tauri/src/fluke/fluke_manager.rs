use crate::serial::serial_manager::SerialDevice;

pub struct Fluke9142 {
    device: SerialDevice,
}

impl Fluke9142 {
    /// Inizializza la connessione usando VID/PID fissi
    pub fn new() -> Result<Self, String> {
        let device = SerialDevice::open_fluke()?;
        Ok(Self { device })
    }

    /// Imposta il Set Point
    pub fn set_temperature(&mut self, temp: f32) {
        self.device.query(&format!("SOUR:SPO {:.2}", temp));
    }

    /// Abilita il riscaldamento (OUTP:STAT 1)
    pub fn start_heating(&mut self) {
        self.device.query("OUTP:STAT 1");
    }

    /// Disabilita il riscaldamento (OUTP:STAT 0)
    pub fn stop_heating(&mut self) {
        self.device.query("OUTP:STAT 0");
    }

    /// Legge la temperatura attuale del pozzetto
    pub fn read_temperature(&mut self) -> Option<f32> {
        self.device
            .query("SOUR:SENS:DATA?")
            .and_then(|res| res.parse::<f32>().ok())
    }

    /// Verifica il flag interno di stabilità
    pub fn is_stable(&mut self) -> bool {
        self.device.query("SOUR:STAB:TEST?") == Some("1".to_string())
    }
}
