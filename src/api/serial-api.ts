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
  timestamp_ms: number;
  current_temp_fluke: number;
  current_temp_sensor: number[]; // Array di campioni letti nell'ultimo secondo
  is_stable: boolean;
  current_step: number;
  total_steps: number;
  elapsed_time: number; // Secondi passati dall'inizio della stabilità
  total_time: number;   // Secondi totali di dwell previsti
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


export function getCalibrationErrorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  if (typeof err === "object" && err && "message" in err) {
    return String((err as any).message);
  }
  return "Errore sconosciuto nella calibrazione";
}
