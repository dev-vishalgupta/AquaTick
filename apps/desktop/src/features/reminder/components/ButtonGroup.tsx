import React from "react";

interface ButtonGroupProps {
  children: React.ReactNode;
}

export const ButtonGroup: React.FC<ButtonGroupProps> = ({ children }) => {
  return (
    <div
      style={{
        display: "flex",
        gap: "12px",
        width: "100%",
        justifyContent: "space-between",
        marginTop: "16px",
      }}
    >
      {children}
    </div>
  );
};
