import React from "react";

interface ActionButtonProps {
  onClick: () => void;
  disabled?: boolean;
  variant?: "primary" | "secondary" | "danger";
  children: React.ReactNode;
}

export const ActionButton: React.FC<ActionButtonProps> = ({
  onClick,
  disabled = false,
  variant = "primary",
  children,
}) => {
  const getColors = () => {
    switch (variant) {
      case "secondary":
        return {
          bg: "rgba(255, 255, 255, 0.05)",
          hoverBg: "rgba(255, 255, 255, 0.15)",
          text: "#cdd6f4",
          border: "1px solid rgba(255, 255, 255, 0.1)",
        };
      case "danger":
        return {
          bg: "#f38ba8", // Red/Pink accent
          hoverBg: "#eba0b2",
          text: "#11111b",
          border: "none",
        };
      case "primary":
      default:
        return {
          bg: "#a6e3a1", // Green accent
          hoverBg: "#b4befe", // Soft pastel blue/indigo
          text: "#11111b",
          border: "none",
        };
    }
  };

  const colors = getColors();

  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        backgroundColor: disabled ? "rgba(255, 255, 255, 0.03)" : colors.bg,
        color: disabled ? "#7f849c" : colors.text,
        border: colors.border,
        borderRadius: "8px",
        padding: "10px 20px",
        fontSize: "14px",
        fontWeight: 600,
        cursor: disabled ? "not-allowed" : "pointer",
        transition: "all 0.15s ease-in-out",
        outline: "none",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        flex: 1,
      }}
      onMouseEnter={(e) => {
        if (!disabled) {
          e.currentTarget.style.backgroundColor = colors.hoverBg;
        }
      }}
      onMouseLeave={(e) => {
        if (!disabled) {
          e.currentTarget.style.backgroundColor = colors.bg;
        }
      }}
    >
      {children}
    </button>
  );
};
