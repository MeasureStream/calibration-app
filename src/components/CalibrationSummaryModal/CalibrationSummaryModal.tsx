import React from 'react';
import { Modal, Button } from 'react-bootstrap';
import { CheckCircleFill } from 'react-bootstrap-icons';

interface CalibrationSummaryModalProps {
  show: boolean;
  onDashboard: () => void;
}

const CalibrationSummaryModal: React.FC<CalibrationSummaryModalProps> = ({
  show,
  onDashboard
}) => {
  return (
    <Modal show={show} centered backdrop="static" keyboard={false}>
      <Modal.Header className="bg-success text-white border-0">
        <Modal.Title className="d-flex align-items-center">
          <CheckCircleFill className="me-2" />
          Fine Processo
        </Modal.Title>
      </Modal.Header>
      <Modal.Body className="text-center p-4">
        <h4 className="fw-bold">Taratura Completata!</h4>
        <p className="text-muted">
          Il progresso ha raggiunto il 100%. Tutti i dati sono stati trasmessi.
        </p>
        <div className="alert alert-info py-2 small">
          È ora sicuro scollegare la Measurement Unit (MU).
        </div>
      </Modal.Body>
      <Modal.Footer className="border-0 justify-content-center pb-4">
        <Button
          variant="primary"
          onClick={onDashboard}
          className="px-5 py-2 fw-bold"
          style={{ borderRadius: '10px' }}
        >
          TORNA ALLA DASHBOARD
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default CalibrationSummaryModal;
