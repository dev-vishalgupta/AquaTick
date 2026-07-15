import React from "react";

interface ReminderOverlayProps {
  isOpen: boolean;
  isMounted: boolean;
  children: React.ReactNode;
}

export const ReminderOverlay: React.FC<ReminderOverlayProps> = ({ isOpen, isMounted, children }) => {
  if (!isOpen) return null;

  return (
    <div
      className={isMounted ? "aquatick-overlay-enter" : "aquatick-overlay-exit"}
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: "rgba(17, 17, 27, 0.7)", // Muted glassmorphism backing
        backdropFilter: "blur(4px)",
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        zIndex: 9999,
      }}
    >
      {children}
    </div>
  );
};
