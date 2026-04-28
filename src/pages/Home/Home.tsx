import { useState } from 'react';
import { Container, Button, Spinner, Alert } from 'react-bootstrap';
import { useNavigate } from 'react-router-dom';
import logo from '../../assets/pi-logo50x70.png'
import { useContextApp } from '../../context/ContextApp';
import { getMUInfo } from '../../api/rest-api'; // Importiamo il comando Tauri

import {
  GraphUp,
  ArrowRepeat,
  Search,
  Cpu // Nuova icona per l'hardware
} from 'react-bootstrap-icons';
import { discoverHardware } from '../../api/serial-api';

const Home = () => {
  const navigate = useNavigate();
  const { version, setMu } = useContextApp();

  // Stati per gestire la sincronizzazione
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const iconSize = 28;

  // Funzione che gestisce il recupero info prima di navigare
  const handleStartCalibration = async () => {

    try {
      setLoading(true);
      setError(null);

      const muId = await discoverHardware();

      // Chiamata al backend tramite Rust
      console.log("MUID: ", muId);
      const mu = await getMUInfo(muId);
      setMu(mu);
      console.log("Hardware sincronizzato:", mu);

      // Se tutto va bene, navighiamo alla pagina taratura
      navigate('/taratura');
    } catch (err) {
      console.error(err);
      setError("Errore sincronizzazione hardware. Controlla la connessione.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Container fluid className="vh-100 d-flex flex-column justify-content-center align-items-center">

      {/* Header */}
      <div className="text-center mb-5">
        <div className="d-flex justify-content-center align-items-center gap-3 mb-2">
          <img src={logo} alt="Logo" style={{ height: '70px' }} />
          <h1 className="fw-bold mb-0">
            Travelling Calibrator
          </h1>
        </div>
        <div className="text-secondary">
          Version {version}
        </div>
      </div>

      {/* Messaggio di Errore se il sync fallisce */}
      {error && (
        <Alert variant="danger" className="w-100 mb-4" style={{ maxWidth: '320px' }}>
          {error}
        </Alert>
      )}

      {/* Buttons */}
      <div
        className="d-grid gap-3"
        style={{ width: '100%', maxWidth: '320px' }}
      >
        <Button
          variant="primary"
          size="lg"
          disabled={loading} // Disabilita durante il caricamento
          className="fw-bold py-3 d-flex align-items-center justify-content-center gap-2"
          onClick={handleStartCalibration}
        >
          {loading ? (
            <>
              <Spinner animation="border" size="sm" />
              SINCRONIZZAZIONE...
            </>
          ) : (
            <>
              <GraphUp size={iconSize} />
              CALIBRATION
            </>
          )}
        </Button>

        <Button
          variant="secondary"
          size="lg"
          className="fw-bold py-3 d-flex align-items-center justify-content-center gap-2"
          onClick={() => navigate('/fw-update')}
        >
          <ArrowRepeat size={iconSize} />
          FW UPDATE
        </Button>

        <Button
          variant="outline-info"
          size="lg"
          className="fw-bold py-3 d-flex align-items-center justify-content-center gap-2"
          onClick={() => navigate('/dev-status')}
        >
          <Search size={iconSize} />
          DEV. STATUS
        </Button>
      </div>

    </Container>
  );
};

export default Home;
