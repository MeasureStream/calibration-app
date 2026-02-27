import { Container, Button } from 'react-bootstrap';
import { useNavigate } from 'react-router-dom';
import logo from '../../assets/pi-logo50x70.png'
import { useContextApp } from '../../context/ContextApp';

import {
  GraphUp,
} from 'react-bootstrap-icons';
import { BackButton } from '../../components/BackButton/BackButton';

const TaraturaList = () => {
  const navigate = useNavigate();
  const { version } = useContextApp();

  const iconSize = 28;

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


      {/* Buttons */}
      <div
        className="d-grid gap-3"
        style={{ width: '100%', maxWidth: '320px' }}
      >

        <Button
          variant="primary"
          size="lg"
          className="fw-bold py-3 d-flex align-items-center justify-content-center gap-2"
          onClick={() => navigate('/taratura/thermal')}
        >
          <GraphUp size={iconSize} />
          Taratura Termica
        </Button>

        <BackButton />

      </div>

    </Container >
  );
};

export default TaraturaList;

