import { useState, useEffect, useRef } from 'react';
import { Container, Row, Col, Button, ProgressBar, Card } from 'react-bootstrap';
import { PlayFill, StopFill, ArrowLeft, Cpu, ThermometerHalf } from 'react-bootstrap-icons';
import { useNavigate } from 'react-router-dom';

// Importiamo le API che abbiamo definito
import {
  startThermalCalibration,
  stopThermalCalibration,
  onThermalCalibrationUpdate,
  CalibrationPayload,
  getCalibrationErrorMessage
} from '../../api/serial-api.ts';

const ThermalCalibration = () => {
  const navigate = useNavigate();
  const [isRunning, setIsRunning] = useState(false);
  const [data, setData] = useState<CalibrationPayload | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const logEndRef = useRef<HTMLDivElement>(null);

  const steps = [
    { target_value: 20.0, tempo_per_step: 1 },
    { target_value: 45.0, tempo_per_step: 2 }
  ];

  // Auto-scroll per i log
  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  // Sottoscrizione agli eventi Rust
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await onThermalCalibrationUpdate((payload) => {
        setData(payload);

        // Esempio di log dinamico quando cambia lo stato
        if (payload.elapsed_time === 1) {
          addLog(`Step ${payload.current_step}: Inizio fase ${payload.status}`);
        }
      });
    };

    setupListener();
    return () => { if (unlisten) unlisten(); };
  }, []);

  const addLog = (msg: string) => {
    const time = new Date().toLocaleTimeString();
    setLogs(prev => [...prev, `[${time}] ${msg}`].slice(-50)); // Ultimi 50 messaggi
  };

  const handleStart = async () => {
    try {
      setLogs([]); // Reset log
      addLog("Inizializzazione strumenti...");



      await startThermalCalibration(steps);
      setIsRunning(true);
      addLog("Taratura avviata con successo.");
    } catch (err) {
      const msg = getCalibrationErrorMessage(err);
      addLog(`ERRORE: ${msg}`);
    }
  };

  const handleStop = async () => {
    await stopThermalCalibration();
    setIsRunning(false);
    addLog("Processo interrotto dall'utente.");
  };

  // Calcolo media sensore (visto che passiamo un array di campioni)
  const getAverageSensorTemp = () => {
    if (!data || data.current_temp_sensor.length === 0) return "--";
    const sum = data.current_temp_sensor.reduce((a, b) => a + b, 0);
    return (sum / data.current_temp_sensor.length).toFixed(2);
  };

  // Calcolo progresso globale (basato sugli step totali)
  const globalProgress = data
    ? Math.round(((data.current_step - 1) / data.total_steps) * 100 + (data.elapsed_time / data.total_time / data.total_steps * 100))
    : 0;

  return (
    <Container fluid className="vh-100 p-0" style={{ backgroundColor: '#f4f7f9', color: '#333' }}>

      {/* Header */}
      <div className="d-flex align-items-center bg-white border-bottom px-3 py-2 shadow-sm">
        <Button variant="light" onClick={() => navigate(-1)} className="me-3 border-0 rounded-circle">
          <ArrowLeft size={24} />
        </Button>
        <h4 className="mb-0 fw-bold text-primary">Taratura Termica</h4>
        <div className="ms-auto d-flex align-items-center">
          {data?.status === "DWELL" && (
            <span className="me-3 text-success fw-bold blink">● REGISTRAZIONE DATI</span>
          )}
          <span className={`badge ${isRunning ? 'bg-success' : 'bg-secondary'} p-2`}>
            {isRunning ? 'SISTEMA ATTIVO' : 'SISTEMA PRONTO'}
          </span>
        </div>
      </div>

      <div className="p-3">
        <Row className="g-3">
          {/* Temperatura Fluke (Riferimento) */}
          <Col xs={4}>
            <Card className="border-0 shadow-sm mb-3 text-center p-3 h-100">
              <ThermometerHalf size={30} className="text-danger mx-auto mb-2" />
              <div className="text-muted small">TEMP. FLUKE (REF)</div>
              <h2 className="fw-bold">{data ? `${data.current_temp_fluke.toFixed(2)}°C` : "--"}</h2>
              <small className={data?.is_stable ? "text-success" : "text-warning"}>
                {data?.is_stable ? "STABILE" : "IN RAMPA"}
              </small>
            </Card>
          </Col>

          {/* Temperatura Sensore (Media campioni) */}
          <Col xs={4}>
            <Card className="border-0 shadow-sm mb-3 text-center p-3 h-100">
              <Cpu size={30} className="text-primary mx-auto mb-2" />
              <div className="text-muted small">TEMP. SENSORE (AVG)</div>
              <h2 className="fw-bold">{getAverageSensorTemp()}°C</h2>
              <small className="text-muted">Campioni/sec: {data?.current_temp_sensor.length || 0}</small>
            </Card>
          </Col>

          {/* Dettaglio Step */}
          <Col xs={4}>
            <Card className="border-0 shadow-sm mb-3 text-center p-3 h-100">
              <div className="text-muted small mb-2">TARGET STEP {data?.current_step || "--"}</div>
              <h3 className="fw-bold text-success">
                {data?.status === "DWELL" ? "DWELL TIME" : "RAMPA"}
              </h3>
              <div className="fw-bold fs-4">
                {data ? `${data.elapsed_time}s / ${data.total_time}s` : "--"}
              </div>
            </Card>
          </Col>
        </Row>

        {/* Barra di Progresso Globale */}
        <Card className="border-0 shadow-sm p-4 my-3">
          <div className="d-flex justify-content-between fw-bold mb-2">
            <span>Avanzamento Totale (Step {data?.current_step || 0} di {data?.total_steps || 0})</span>
            <span>{globalProgress}%</span>
          </div>
          <ProgressBar
            now={globalProgress}
            style={{ height: '40px', borderRadius: '10px' }}
            variant={data?.status === "DWELL" ? "success" : "primary"}
            animated={isRunning && data?.status === "RAMPA"}
          />
        </Card>

        {/* Terminale Log Seriale */}
        <div className="bg-dark text-light border rounded p-3 mb-3 shadow-sm"
          style={{ height: '150px', overflowY: 'auto', fontSize: '0.85rem', fontFamily: 'monospace' }}>
          {logs.map((log, i) => (
            <div key={i} className={log.includes("ERRORE") ? "text-danger" : ""}>
              {log}
            </div>
          ))}
          <div ref={logEndRef} />
        </div>

        {/* Pulsante Azione Gigante */}
        <Row>
          <Col>
            {!isRunning ? (
              <Button
                variant="primary"
                className="w-100 py-3 fw-bold shadow border-0"
                style={{ fontSize: '1.8rem', borderRadius: '15px' }}
                onClick={handleStart}
              >
                <PlayFill size={40} /> AVVIA TARATURA
              </Button>
            ) : (
              <Button
                variant="danger"
                className="w-100 py-3 fw-bold shadow border-0"
                style={{ fontSize: '1.8rem', borderRadius: '15px' }}
                onClick={handleStop}
              >
                <StopFill size={40} /> ARRESTA PROCESSO
              </Button>
            )}
          </Col>
        </Row>
      </div>

      <style>{`
        .blink { animation: blinker 1.5s linear infinite; }
        @keyframes blinker { 50% { opacity: 0; } }
      `}</style>
    </Container>
  );
};

export default ThermalCalibration;
