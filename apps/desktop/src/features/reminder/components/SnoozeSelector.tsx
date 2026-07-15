import React from "react";
import { ActionButton } from "./ActionButton";

interface SnoozeSelectorProps {
  isOpen: boolean;
  onSelect: (minutes: number) => void;
  disabled: boolean;
}

const SNOOZE_OPTIONS = [5, 10, 15, 30];

export const SnoozeSelector: React.FC<SnoozeSelectorProps> = ({
  isOpen,
  onSelect,
  disabled,
}) => {
  if (!isOpen) return null;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "8px",
        marginTop: "12px",
        padding: "16px",
        backgroundColor: "rgba(0, 0, 0, 0.25)",
        borderRadius: "10px",
        border: "1px solid rgba(255, 255, 255, 0.05)",
      }}
    >
      <div
        style={{
          fontSize: "13px",
          color: "#a6adc8",
          textAlign: "left",
          marginBottom: "6px",
          fontWeight: 600,
        }}
      >
        Select snooze duration:
      </div>
      <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
        {SNOOZE_OPTIONS.map((min) => (
          <ActionButton
            key={min}
            onClick={() => onSelect(min)}
            disabled={disabled}
            variant="secondary"
          >
            {min}m
          </ActionButton>
        ))}
      </div>
    </div>
  );
};
