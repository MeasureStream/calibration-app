import { invoke } from '@tauri-apps/api/core';

export interface SensorTemplateDTO {
  modelName: string;
  type: string; // "accelerometer", "pressure", "humidity", "temperature"
  unit: string;
  conversion?: any;
  properties?: any;
  ranges?: any;
  metrology?: any;
}

export interface SensorDTO {
  id: number;
  modelName: string;
  sensorIndex: number;
  physVal: number;
  // Coefficienti per la taratura
  coeffA?: number | null;
  coeffB?: number | null;
  coeffC?: number | null;
  coeffD?: number | null;
  sensorTemplate: SensorTemplateDTO;
}

export interface MeasurementUnitDTO {
  id: number;
  extendedId: number; // Coerente con la modifica Rust
  localId: number;
  model: number;
  controlUnitId: number;
  sensors: SensorDTO[];
}

/**
 * Chiama il comando Rust per sincronizzare e ottenere le info della MU
 */
export const getMUInfo = async (): Promise<MeasurementUnitDTO> => {
  try {
    // Nota: Assicurati che il comando tauri ::command in Rust si chiami 'get_mu_info'
    const response = await invoke<MeasurementUnitDTO>('get_muinfo');
    console.log("MU Info ricevuta:", response);
    return response;
  } catch (error) {
    console.error("Errore durante il recupero della MU:", error);
    throw error;
  }
};
