import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

// --- INTERFACCE ---

export interface CalibrationStep {
  target_value: number;
  tempo_per_step: number; // in minuti (come richiesto dal backend)
}

/**
 * Rispecchia esattamente il CalibrationPayload inviato dal Master Thread in Rust
 */
export interface CalibrationPayload {
  timestamp: number;          // Rinomina timestamp_ms -> timestamp se hai cambiato in Rust
  current_temp_fluke: number;
  current_temp_sensor: number; // Modificato da number[] a number (la media)
  samples_count: number;       // Nuovo campo: frequenza campionamento reale
  is_stable: boolean;
  current_step: number;
  total_steps: number;
  elapsed_time: number;       // Secondi passati nello stato attuale
  total_time: number;         // Secondi totali di dwell previsti
  status: "RAMPA" | "DWELL";
}

export async function startThermalCalibration(steps: CalibrationStep[]): Promise<void> {
  return await invoke("_start_thermal_calibration", { steps });
}

export async function stopThermalCalibration(): Promise<void> {
  return await invoke("_stop_thermal_calibration");
}


export const onThermalCalibrationUpdate = async (
  callback: (data: CalibrationPayload) => void
): Promise<UnlistenFn> => {
  return await listen<CalibrationPayload>("calibration-update", (event) => {
    const p = event.payload;

    if (!p) {
      console.warn("Ricevuto payload calibrazione termica nullo");
      return;
    }

    try {
      // Passiamo il payload direttamente alla callback
      callback(p);
    } catch (e) {
      console.error("Errore durante la gestione dell'aggiornamento termico:", e);
    }
  });
};

export async function discoverHardware(): Promise<number> {
  return await invoke("discover_hardware");
}


export function getCalibrationErrorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  if (typeof err === "object" && err && "message" in err) {
    return String((err as any).message);
  }
  return "Errore sconosciuto nella calibrazione";
}
/**
 * Ascolta errori asincroni che avvengono durante la marcia (es. distacco hardware)
 */
export const onCalibrationError = async (
  callback: (errorMessage: string) => void
): Promise<UnlistenFn> => {
  return await listen<string>("calibration-error", (event) => {
    const msg = event.payload;
    if (!msg) return;
    try {
      callback(msg);
    } catch (e) {
      console.error("Errore callback error handler:", e);
    }
  });
};
