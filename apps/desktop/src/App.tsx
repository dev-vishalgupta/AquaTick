import { BrowserRouter, Routes, Route } from "react-router-dom";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import About from "./pages/About";
import SystemOrchestrator from "./components/SystemOrchestrator";

export default function App() {
  return (
    <BrowserRouter>
      <SystemOrchestrator />
      <Routes>
        <Route path="/" element={<Dashboard />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="/about" element={<About />} />
      </Routes>
    </BrowserRouter>
  );
}
