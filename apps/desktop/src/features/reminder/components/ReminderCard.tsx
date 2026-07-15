import React from "react";

interface ReminderCardProps {
  isMounted: boolean;
  children: React.ReactNode;
}

export const ReminderCard: React.FC<ReminderCardProps> = ({ isMounted, children }) => {
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="aquatick-reminder-title"
      aria-describedby="aquatick-reminder-description"
      className={isMounted ? "aquatick-card-enter" : "aquatick-card-exit"}
      style={{
        backgroundColor: "#1e1e2e", // Mocha Dark theme background
        border: "1px solid rgba(255, 255, 255, 0.1)",
        borderRadius: "16px",
        padding: "32px",
        boxShadow: "0 12px 40px rgba(0, 0, 0, 0.6)",
        width: "380px",
        maxWidth: "90%",
        fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif",
        textAlign: "center",
      }}
    >
      {children}
    </div>
  );
};
