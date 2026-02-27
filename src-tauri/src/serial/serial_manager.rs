use serialport;
use std::io::{BufRead, BufReader, Read, Write};
use std::time::Duration;

pub struct SerialDevice {
    port: Box<dyn serialport::SerialPort>,
    pub local_id: u8,
    pub extended_uid: u32,
}

impl SerialDevice {
    /// Cerca il Fluke usando VID e PID
    pub fn open_fluke() -> Result<Self, String> {
        // VID: 0403, PID: 6001, Baud: 9600

        Self::open_by_vid_pid(0x0403, 0x6001, 9600, 0)
            .map_err(|_| "Fluke 9142 non trovato (VID:0403 PID:6001)".into())
    }

    pub fn open_by_vid_pid(vid: u16, pid: u16, baud: u32, local_id: u8) -> Result<Self, String> {
        let ports = serialport::available_ports().map_err(|e| e.to_string())?;

        for p in ports {
            if let serialport::SerialPortType::UsbPort(info) = p.port_type {
                if info.vid == vid && info.pid == pid {
                    // Trovato il dispositivo, chiamiamo la new con il percorso della porta
                    let extenend_uid = ((vid as u32) << 16) | (pid as u32);
                    println!("portname: {}", &p.port_name);
                    return Self::new(&p.port_name, baud, extenend_uid, local_id);
                }
            }
        }

        Err(format!(
            "Dispositivo non trovato (VID: {:04x} PID: {:04x})",
            vid, pid
        ))
    }

    pub fn new(path: &str, baud: u32, extended_uid: u32, local_id: u8) -> Result<Self, String> {
        let port = serialport::new(path, baud)
            .timeout(Duration::from_secs(1)) // Timeout impostato a 1s come richiesto
            .open()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            port,
            extended_uid,
            local_id,
        })
    }

    /// Implementazione query per il Fluke (ASCII con \r)
    pub fn query(&mut self, cmd: &str) -> Option<String> {
        let full_cmd = format!("{}\r", cmd); // Utilizza terminatore CR
        self.port.write_all(full_cmd.as_bytes()).ok()?;

        // Piccola attesa per l'elaborazione hardware (come lo sleep 0.1 in Python)
        std::thread::sleep(Duration::from_millis(100));

        let mut reader = BufReader::new(&mut self.port);
        let mut response = String::new();
        // Legge fino a CR LF
        match reader.read_line(&mut response) {
            Ok(_) => Some(response.trim().to_string()),
            Err(_) => None,
        }
    }

    pub fn _send_command(&mut self, cmd: &[u8]) -> Result<(), String> {
        self.port.write_all(cmd).map_err(|e| e.to_string())
    }

    /// Avvia la taratura sul sensore specifico
    /// Basato sul documento: #Sensore, Freq, PacketSize
    pub fn start_mu_calibration(
        &mut self,
        sensor_id: u8,
        freq: u16,
        packet_size: u8,
    ) -> Result<(), String> {
        let mut packet = Vec::with_capacity(6);
        packet.push(self.local_id); // Byte 0: Local ID MU
        packet.push(0xF1); // Byte 1: Opcode fisso
        packet.push(sensor_id); // Byte 2: #Sensore
        packet.extend_from_slice(&freq.to_be_bytes()); // Byte 3-4: Freq (u16)
        packet.push(packet_size); // Byte 5: Packet size

        self.port.write_all(&packet).map_err(|e| e.to_string())
    }

    pub fn stop_mu_calibration(&mut self, mu_id: u8, sensor_id: u8) -> Result<(), String> {
        let packet = vec![mu_id, 0xF2, sensor_id];
        println!("Sensore Stoppato calibrazione");
        self.port.write_all(&packet).map_err(|e| e.to_string())
    }
    /// In modalità streaming è meglio usare bytes_to_read per non bloccare
    pub fn read_available(&mut self) -> Vec<u8> {
        let mut buffer = Vec::new();
        if let Ok(count) = self.port.bytes_to_read() {
            if count > 0 {
                let mut temp_buf = vec![0u8; count as usize];
                if self.port.read_exact(&mut temp_buf).is_ok() {
                    buffer = temp_buf;
                }
            }
        }
        buffer
    }

    pub fn connect_mu(&mut self, local_id: u8) -> Result<Vec<u8>, String> {
        let packet = vec![0x00, 0x01, local_id];

        // 1. PULIZIA: Eliminiamo residui per essere sicuri che i primi 4 byte siano la risposta
        self.port.clear(serialport::ClearBuffer::Input).ok();

        // 2. INVIO: Scriviamo e forziamo l'uscita dei dati
        self.port.write_all(&packet).map_err(|e| e.to_string())?;
        self.port.flush().map_err(|e| e.to_string())?;

        // 3. ATTESA: Diamo tempo alla MU di rispondere (fondamentale per evitare il Timeout)
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 4. LETTURA
        let mut response = vec![0u8; 4];
        match self.port.read_exact(&mut response) {
            Ok(_) => {
                self.extended_uid =
                    u32::from_be_bytes([response[0], response[1], response[2], response[3]]);
                println!("UID : {}", self.extended_uid);
                Ok(response)
            }
            Err(e) => Err(format!("MU non ha risposto all'handshake: {}", e)),
        }
    }
}
