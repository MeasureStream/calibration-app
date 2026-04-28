import { Container, Button, Badge, Card } from 'react-bootstrap';
import { useNavigate } from 'react-router-dom';
import logo from '../../assets/pi-logo50x70.png'
import { useContextApp } from '../../context/ContextApp';
import { GraphUp, Cpu, Activity } from 'react-bootstrap-icons';
import { BackButton } from '../../components/BackButton/BackButton';

const TaraturaList = () => {
  const navigate = useNavigate();
  const { version, mu } = useContextApp();

  const iconSize = 24;

  // Estraiamo i tipi di sensori unici per creare i bottoni dinamici
  // Ad esempio: se ci sono 3 sensori di temperatura, vogliamo un solo bottone "Taratura Termica"
  const sensorTypes = mu ? Array.from(new Set(mu.sensors.map(s => s.sensorTemplate.type))) : [];

  // Funzione per mappare il tipo di sensore al testo e alla rotta corretta
  const getCalibrationInfo = (type: string) => {
    switch (type) {
      case 'temperature':
        return { label: 'Taratura Termica', route: '/taratura/thermal', variant: 'primary' };
      case 'pressure':
        return { label: 'Taratura Pressione', route: '/taratura/pressure', variant: 'info' };
      case 'humidity':
        return { label: 'Taratura Umidità', route: '/taratura/humidity', variant: 'outline-primary' };
      case 'accelerometer':
        return { label: 'Taratura Accelerometro', route: '/taratura/accel', variant: 'secondary' };
      default:
        return { label: `Taratura ${type}`, route: `/taratura/${type}`, variant: 'outline-primary' };
    }
  };

  return (
    <Container fluid className="vh-100 d-flex flex-column justify-content-center align-items-center">

      {/* Header */}
      <div className="text-center mb-4">
        <div className="d-flex justify-content-center align-items-center gap-3 mb-2">
          <img src={logo} alt="Logo" style={{ height: '60px' }} />
          <h1 className="fw-bold mb-0">Travelling Calibrator</h1>
        </div>
        <div className="text-secondary">Version {version}</div>
      </div>

      {/* Info Box MU Selezionata */}
      {mu && (
        <Card className="mb-4 shadow-sm border-0 bg-light" style={{ width: '100%', maxWidth: '360px' }}>
          <Card.Body className="py-2 px-3">
            <div className="d-flex align-items-center justify-content-between">
              <div className="d-flex align-items-center gap-2">
                <Cpu size={20} className="text-primary" />
                <span className="fw-bold text-dark">MU: {mu.extendedId}</span>
              </div>
              <Badge bg="success" pill>Hardware Ready</Badge>
            </div>
            <div className="mt-1 text-muted" style={{ fontSize: '0.8rem' }}>
              <Activity size={14} className="me-1" />
              {mu.sensors.length} sensori rilevati su Local ID {mu.localId}
            </div>
          </Card.Body>
        </Card>
      )}

      {/* Buttons Dynamici */}
      <div className="d-grid gap-3" style={{ width: '100%', maxWidth: '320px' }}>

        {sensorTypes.length > 0 ? (
          sensorTypes.map((type) => {
            const config = getCalibrationInfo(type);
            return (
              <Button
                key={type}
                variant={config.variant}
                size="lg"
                className="fw-bold py-3 d-flex align-items-center justify-content-center gap-3 shadow-sm"
                onClick={() => navigate(config.route)}
              >
                <GraphUp size={iconSize} />
                {config.label.toUpperCase()}
              </Button>
            );
          })
        ) : (
          <div className="text-center text-danger mb-3">
            Nessun sensore tarabile trovato nella MU.
          </div>
        )}

        <div className="mt-2 text-center">
          <BackButton />
        </div>

      </div>

    </Container>
  );
};

export default TaraturaList;
