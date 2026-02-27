import { createContext, useContext, useEffect, useState, ReactNode } from "react";
import { app } from "@tauri-apps/api";

interface ContextAppType {
  version: string;
}

const ContextApp = createContext<ContextAppType>({ version: "" });

export const ContextProvider = ({ children }: { children: ReactNode }) => {
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      const v = await app.getVersion();
      setVersion(v);
    };
    fetchVersion();
  }, []);

  return (
    <ContextApp.Provider value={{ version }}>
      {children}
    </ContextApp.Provider>
  );
};

export const useContextApp = () => useContext(ContextApp);


