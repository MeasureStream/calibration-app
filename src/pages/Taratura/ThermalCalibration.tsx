import { useState, useEffect, } from 'react';
import { Container, Row, Col, Button, ProgressBar, Card, Alert, Spinner } from 'react-bootstrap';
import { PlayFill, StopFill, ArrowLeft, Cpu, ThermometerHalf, InfoCircleFill } from 'react-bootstrap-icons';
import { useNavigate } from 'react-router-dom';
import CalibrationSummaryModal from '../../components/CalibrationSummaryModal/CalibrationSummaryModal.tsx';
import { UnlistenFn } from '@tauri-apps/api/event';

import {
  startThermalCalibration,
  stopThermalCalibration,
  onThermalCalibrationUpdate,
  onCalibrationError,
  CalibrationPayload,
  getCalibrationErrorMessage
} from '../../api/serial-api.ts';

const ThermalCalibration = () => {
  const navigate = useNavigate();
  const [isRunning, setIsRunning] = useState(false);
  const [data, setData] = useState<CalibrationPayload | null>(null);
  const [showFinishModal, setShowFinishModal] = useState(false);

  // Gestione messaggi e errori
  const [error, setError] = useState<string | null>(null);
  const [infoMessage, setInfoMessage] = useState<string | null>(null);

  const steps = [
    { target_value: -22.5, tempo_per_step: 1 },
    { target_value: 94.5, tempo_per_step: 1 }
  ];

  // Calcolo progresso globale
  const globalProgress = data && data.total_steps > 0
    ? Math.min(100, Math.round(
      ((data.current_step - 1) / data.total_steps * 100) +
      (data.elapsed_time / data.total_time * (100 / data.total_steps))
    ))
    : 0;

  // Monitoraggio fine processo
  useEffect(() => {
    if (globalProgress === 100 && isRunning) {
      setIsRunning(false);
      setShowFinishModal(true);
      setInfoMessage("Processo completato con successo.");
    }
  }, [globalProgress, isRunning]);

  // Sottoscrizione Eventi Rust
  useEffect(() => {
    let unlistenUpdate: UnlistenFn;
    let unlistenError: UnlistenFn;

    const setupListeners = async () => {
      unlistenUpdate = await onThermalCalibrationUpdate((payload) => {
        setData(payload);
        // Se passiamo a DWELL, informiamo l'utente
        if (payload.status === "DWELL" && payload.elapsed_time === 1) {
          setInfoMessage(`Step ${payload.current_step}: Temperatura stabile, registrazione dati in corso...`);
        }
      });

      unlistenError = await onCalibrationError((msg) => {
        setError(msg);
        setIsRunning(false);
        setInfoMessage(null); // Rimuoviamo info se c'è un errore critico
      });
    };

    setupListeners();
    return () => {
      if (unlistenUpdate) unlistenUpdate();
      if (unlistenError) unlistenError();
    };
  }, []);

  // Timer per far scomparire i messaggi Info
  useEffect(() => {
    if (infoMessage) {
      const timer = setTimeout(() => {
        setInfoMessage(null);
      }, 5000); // Scompare dopo 5 secondi

      return () => clearTimeout(timer);
    }
  }, [infoMessage]);

  const handleStart = async () => {
    try {
      setError(null);
      setInfoMessage("Inizializzazione strumenti in corso...");
      await startThermalCalibration(steps);
      setIsRunning(true);
    } catch (err) {
      const msg = getCalibrationErrorMessage(err);
      setError(`Errore durante l'avvio: ${msg}`);
      setInfoMessage(null);
    }
  };

  const handleStop = async () => {
    await stopThermalCalibration();
    setIsRunning(false);
    setInfoMessage("Processo interrotto manualmente.");
  };

  const getAverageSensorTemp = () => {
    // Verifichiamo che i dati esistano. 
    // Usiamo typeof per permettere la visualizzazione dello 0.00°C
    if (!data || typeof data.current_temp_sensor !== 'number') return "--";

    return data.current_temp_sensor.toFixed(2);
  };

  return (
    <Container fluid className="vh-100 p-0" style={{ backgroundColor: '#f4f7f9', color: '#333' }}>

      {/* Header */}
      <div className="d-flex align-items-center bg-white border-bottom px-3 py-2 shadow-sm">
        <Button
          variant="light"
          onClick={() => !isRunning && navigate(-1)}
          disabled={isRunning} className={`me-3 border-0 rounded-circle ${isRunning ? 'opacity-50' : ''}`}
          style={{ cursor: isRunning ? 'not-allowed' : 'pointer' }}
        >
          <ArrowLeft size={24} />
        </Button>
        <h4 className="mb-0 fw-bold text-primary">Thermal Calibration</h4>
        <div className="ms-auto d-flex align-items-center">
          {data?.status === "DWELL" && (
            <span className="me-3 text-success fw-bold blink">● RECORDING</span>
          )}
          <span className={`badge ${isRunning ? 'bg-success' : 'bg-secondary'} p-2`}>
            {isRunning ? 'ACTIVE' : 'READY'}
          </span>
        </div>
      </div>

      <div className="p-4">

        {/* AREA MESSAGGI (Sostituisce il Terminale) */}
        <div style={{ minHeight: '80px' }}>
          {error && (
            <Alert variant="danger" onClose={() => setError(null)} dismissible className="shadow-sm border-0 border-start border-5 border-danger">
              <div className="d-flex align-items-center">
                <strong className="me-2">CRITICAL ERROR:</strong> {error}
              </div>
            </Alert>
          )}

          {infoMessage && !error && (
            <Alert variant="info" onClose={() => setInfoMessage(null)} dismissible className="shadow-sm border-0 border-start border-5 border-info">
              <div className="d-flex align-items-center">
                {isRunning && <Spinner animation="border" size="sm" className="me-3" />}
                <InfoCircleFill className="me-2" />
                {infoMessage}
              </div>
            </Alert>
          )}
        </div>

        <Row className="g-3 mt-2">
          <Col xs={4}>
            <Card className="border-0 shadow-sm text-center p-3 h-100">
              <ThermometerHalf size={30} className="text-danger mx-auto mb-2" />
              <div className="text-muted small">TEMP. FLUKE (REF)</div>
              <h2 className="fw-bold">{data ? `${data.current_temp_fluke.toFixed(2)}°C` : "--"}</h2>
              <small className={data?.is_stable ? "text-success fw-bold" : "text-warning fw-bold"}>
                {data?.is_stable ? "STABLE" : "NOT STABLE"}
              </small>
            </Card>
          </Col>

          <Col xs={4}>
            <Card className="border-0 shadow-sm text-center p-3 h-100">
              <Cpu size={30} className="text-primary mx-auto mb-2" />
              <div className="text-muted small">Sensor Tempetature (AVG)</div>
              <h2 className="fw-bold">{getAverageSensorTemp()}°C</h2>
              <small className="text-muted">Frequency: {data?.samples_count || 0} Hz</small>
            </Card>
          </Col>

          <Col xs={4}>
            <Card className="border-0 shadow-sm text-center p-3 h-100">
              <div className="text-muted small mb-2">TARGET STEP {data?.current_step || "--"}</div>
              <h3 className={`fw-bold ${data?.status === "DWELL" ? "text-success" : "text-primary"}`}>
                {data?.status === "DWELL" ? "DWELL TIME" : "NOT TARGET"}
              </h3>
              <div className="fw-bold fs-4">
                {data ? `${data.elapsed_time}s / ${data.total_time}s` : "--"}
              </div>
            </Card>
          </Col>
        </Row>

        <Card className="border-0 shadow-sm p-4 my-4">
          <div className="d-flex justify-content-between fw-bold mb-2">
            <span>Total Progress (Step {data?.current_step || 0} di {data?.total_steps || 0})</span>
            <span>{globalProgress}%</span>
          </div>
          <ProgressBar
            now={globalProgress}
            style={{ height: '45px', borderRadius: '12px' }}
            variant={data?.status === "DWELL" ? "success" : "primary"}
            animated={isRunning && data?.status === "RAMPA"}
          />
        </Card>

        <Row className="mt-4">
          <Col>
            {!isRunning ? (
              <Button
                variant="primary"
                className="w-100 py-3 fw-bold shadow-lg border-0"
                style={{ fontSize: '1.8rem', borderRadius: '15px' }}
                onClick={handleStart}
              >
                <PlayFill size={40} /> START CALIBRATION
              </Button>
            ) : (
              <Button
                variant="danger"
                className="w-100 py-3 fw-bold shadow-lg border-0"
                style={{ fontSize: '1.8rem', borderRadius: '15px' }}
                onClick={handleStop}
              >
                <StopFill size={40} /> STOP CALIBRATION
              </Button>
            )}
          </Col>
        </Row>
      </div>

      <CalibrationSummaryModal
        show={showFinishModal}
        onDashboard={() => navigate('/')}
      />

      <style>{`
        .blink { animation: blinker 1.5s linear infinite; }
        @keyframes blinker { 50% { opacity: 0; } }
      `}</style>
    </Container>
  );
};

export default ThermalCalibration;
