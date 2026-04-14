import { createContext, useContext, useEffect, useState, ReactNode } from "react";
import { app } from "@tauri-apps/api";
import { MeasurementUnitDTO } from "../api/rest-api";

interface ContextAppType {
  version: string;
  mu: MeasurementUnitDTO | null;
  setMu: (mu: MeasurementUnitDTO | null) => void;
}

const ContextApp = createContext<ContextAppType>({
  version: "",
  mu: null,
  setMu: () => { },
});

export const ContextProvider = ({ children }: { children: ReactNode }) => {
  const [version, setVersion] = useState("");
  const [mu, setMu] = useState<MeasurementUnitDTO | null>(null)

  useEffect(() => {
    const fetchVersion = async () => {
      const v = await app.getVersion();
      setVersion(v);
    };
    fetchVersion();
  }, []);

  return (
    <ContextApp.Provider value={{ version, mu, setMu }}>
      {children}
    </ContextApp.Provider>
  );
};

export const useContextApp = () => useContext(ContextApp);


