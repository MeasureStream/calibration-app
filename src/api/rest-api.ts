import { invoke } from '@tauri-apps/api/core';

// Definizione del sensore semplificato
export interface SensorDTO {
  modelName: string;
  sensorIndex: number;
}

// Definizione della Measurement Unit
export interface MeasurementUnitDTO {
  id: number;
  networkId: number;
  model: number;
  nodeId: number | null;
  sensors: SensorDTO[];
}

/**
 * Chiama il comando Rust per sincronizzare e ottenere le info della MU
 */
export const getMUInfo = async (): Promise<MeasurementUnitDTO> => {
  try {
    // Il nome deve corrispondere esattamente al nome della funzione Rust
    // (senza il prefisso _ se l'hai tolto come suggerito)
    const response = await invoke<MeasurementUnitDTO>('get_muinfo');
    console.log("MU Info ricevuta con successo:", response);
    return response;
  } catch (error) {
    console.error("Errore durante il recupero della MU:", error);
    throw error;
  }
};
