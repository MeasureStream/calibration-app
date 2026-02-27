import { Button } from 'react-bootstrap';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft } from 'react-bootstrap-icons'; // Assicurati di aver installato react-bootstrap-icons

export const BackButton = () => {
  const navigate = useNavigate();

  return (
    <Button
      variant="outline-light" // Contrasta bene sullo sfondo dark
      size="lg"
      className="d-flex align-items-center justify-content-center shadow-sm"
      style={{
        width: '60px',
        height: '60px',
        borderRadius: '50%' // Lo rendiamo tondo per il touch
      }}
      onClick={() => navigate(-1)} // "-1" torna alla pagina precedente della cronologia
    >
      <ArrowLeft size={30} />
    </Button>
  );
};

