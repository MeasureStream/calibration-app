import { Container, Button } from "react-bootstrap";
import { useNavigate } from "react-router-dom";
import { ExclamationTriangle, House } from "react-bootstrap-icons";

const FallbackPage = () => {
  const navigate = useNavigate();

  return (
    <Container
      fluid
      className="vh-100 d-flex flex-column justify-content-center align-items-center text-center"
    >

      {/* Icon */}
      <ExclamationTriangle size={64} className="text-warning mb-4" />

      {/* Title */}
      <h1 className="fw-bold mb-3">
        Pagina non trovata
      </h1>

      {/* Description */}
      <p className="text-secondary mb-4" style={{ maxWidth: "400px" }}>
        La pagina richiesta non esiste oppure si è verificato un errore.
      </p>

      {/* Button */}
      <Button
        variant="primary"
        size="lg"
        className="d-flex align-items-center gap-2 px-4 py-2"
        onClick={() => navigate("/")}
      >
        <House size={20} />
        Torna alla Home
      </Button>

    </Container>
  );
};

export default FallbackPage;
