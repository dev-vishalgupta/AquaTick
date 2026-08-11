/**
 * @file Phase8FAuditRunner.tsx
 *
 * Phase 8F — Audit Runner Component (Development Only)
 *
 * Renders a floating audit panel in the Overlay Window that executes the
 * Phase 8F audit harness on demand. This component must only be mounted
 * in development / QA builds. It must never ship in production.
 *
 * To use: import and render <Phase8FAuditRunner /> inside OverlayOrchestrator
 * or any dev entry point, guarded by import.meta.env.DEV.
 */

import { useState } from "react";
import { runPhase8FAudit } from "./Phase8FAudit";

interface AuditSummary {
  passed: number;
  failed: number;
  total: number;
}

export default function Phase8FAuditRunner() {
  const [running, setRunning] = useState(false);
  const [summary, setSummary] = useState<AuditSummary | null>(null);

  const handleRun = async () => {
    setRunning(true);
    setSummary(null);
    try {
      const result = await runPhase8FAudit();
      setSummary(result);
    } finally {
      setRunning(false);
    }
  };

  const allPassed = summary ? summary.failed === 0 : null;

  return (
    <div
      style={{
        position: "fixed",
        bottom: 16,
        right: 16,
        zIndex: 9999,
        background: "#1a1a2e",
        border: "1px solid #444",
        borderRadius: 8,
        padding: "12px 16px",
        minWidth: 240,
        fontFamily: "monospace",
        fontSize: 13,
        color: "#e0e0e0",
        boxShadow: "0 4px 20px rgba(0,0,0,0.5)",
      }}
    >
      <div style={{ fontWeight: "bold", marginBottom: 8, color: "#90caf9" }}>
        Phase 8F Audit
      </div>

      {summary && (
        <div
          style={{
            marginBottom: 10,
            padding: "6px 10px",
            borderRadius: 4,
            background: allPassed ? "#1b5e20" : "#b71c1c",
            color: allPassed ? "#a5d6a7" : "#ffcdd2",
            fontWeight: "bold",
          }}
        >
          {allPassed
            ? `✅ All ${summary.total} checks passed`
            : `❌ ${summary.failed}/${summary.total} checks FAILED`}
        </div>
      )}

      {summary && (
        <div style={{ marginBottom: 10, fontSize: 11, color: "#aaa" }}>
          Passed: {summary.passed} · Failed: {summary.failed} · Total:{" "}
          {summary.total}
          <br />
          See browser console for full detail.
        </div>
      )}

      <button
        onClick={handleRun}
        disabled={running}
        style={{
          width: "100%",
          padding: "7px 0",
          background: running ? "#37474f" : "#0d47a1",
          color: running ? "#78909c" : "#fff",
          border: "none",
          borderRadius: 4,
          cursor: running ? "not-allowed" : "pointer",
          fontFamily: "monospace",
          fontWeight: "bold",
          fontSize: 13,
          transition: "background 0.2s",
        }}
      >
        {running ? "Running audit…" : "Run Phase 8F Audit"}
      </button>
    </div>
  );
}
