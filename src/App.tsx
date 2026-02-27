//import "./App.css";
import { Container } from "react-bootstrap";
import { Routes, Route } from "react-router-dom";
import Home from "./pages/Home/Home";
import FallbackPage from "./pages/Fallback/FallbackPage";
import TaraturaList from "./pages/Taratura/TaraturaList";
import ThermalCalibration from "./pages/Taratura/ThermalCalibration";

function App() {

  return (



    <Container fluid className="d-flex p-0" style={{ minHeight: "100vh" }}>
      <div style={{ flexGrow: 1, padding: "1rem" }}>
        <Routes>
          <Route path="*" element={<FallbackPage />} />
          <Route path="/" element={<Home />} />
          <Route path="/taratura" element={<TaraturaList />} />
          <Route path="/taratura/thermal" element={<ThermalCalibration />} />

        </Routes>
      </div>
    </Container >
  );
}

export default App;
