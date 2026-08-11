import { BrowserRouter, Routes, Route } from "react-router-dom";
import StatisticsPage from "./pages/Statistics";
import Settings from "./pages/Settings";
import About from "./pages/About";
import AppShell from "./components/AppShell";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<StatisticsPage />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/about" element={<About />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
