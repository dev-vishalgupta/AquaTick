import React, { useState, useEffect, useRef } from "react";
import type { ReminderWindowProps } from "../types";
import { ReminderOverlay } from "./ReminderOverlay";
import { ReminderCard } from "./ReminderCard";
import { ActionButton } from "./ActionButton";
import { ButtonGroup } from "./ButtonGroup";
import { SnoozeSelector } from "./SnoozeSelector";
import { useReminderUI } from "../hooks/useReminderUI";

export const ReminderWindow: React.FC<ReminderWindowProps> = ({
  isOpen,
  isProcessing,
  onDrink,
  onSnooze,
  onIgnore,
}) => {
  const { isSnoozeOpen, toggleSnooze } = useReminderUI();
  const [shouldRender, setShouldRender] = useState(isOpen);
  const [isMounted, setIsMounted] = useState(isOpen);
  const containerRef = useRef<HTMLDivElement>(null);

  // Sync animation mounting lifecycles
  useEffect(() => {
    if (isOpen) {
      setShouldRender(true);
      const timer = setTimeout(() => setIsMounted(true), 10);
      return () => clearTimeout(timer);
    } else {
      setIsMounted(false);
      const timer = setTimeout(() => setShouldRender(false), 200); // Matches animation duration
      return () => clearTimeout(timer);
    }
  }, [isOpen]);

  // Focus trap and accessibility management
  useEffect(() => {
    if (!isOpen || !shouldRender) return;

    const previouslyFocusedElement = document.activeElement as HTMLElement;

    // Async delay to ensure element is rendered in DOM before attempting focus
    const timer = setTimeout(() => {
      if (containerRef.current) {
        const focusable = containerRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), [tabindex="0"]:not([disabled])'
        );
        if (focusable.length > 0) {
          focusable[0].focus();
        }
      }
    }, 50);

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      if (!containerRef.current) return;

      const list = containerRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [tabindex="0"]:not([disabled])'
      );
      if (list.length === 0) return;

      const first = list[0];
      const last = list[list.length - 1];

      if (e.shiftKey) {
        if (document.activeElement === first) {
          last.focus();
          e.preventDefault();
        }
      } else {
        if (document.activeElement === last) {
          first.focus();
          e.preventDefault();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      clearTimeout(timer);
      window.removeEventListener("keydown", handleKeyDown);
      if (previouslyFocusedElement) {
        previouslyFocusedElement.focus();
      }
    };
  }, [isOpen, shouldRender]);

  if (!shouldRender) return null;

  return (
    <>
      <style dangerouslySetInnerHTML={{ __html: `
        @keyframes aquatick-fade-in {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes aquatick-fade-out {
          from { opacity: 1; }
          to { opacity: 0; }
        }
        @keyframes aquatick-scale-in {
          from { transform: scale(0.95); opacity: 0; }
          to { transform: scale(1); opacity: 1; }
        }
        @keyframes aquatick-scale-out {
          from { transform: scale(1); opacity: 1; }
          to { transform: scale(0.95); opacity: 0; }
        }
        
        .aquatick-overlay-enter {
          animation: aquatick-fade-in 0.2s forwards ease-out;
        }
        .aquatick-overlay-exit {
          animation: aquatick-fade-out 0.2s forwards ease-in;
        }
        
        .aquatick-card-enter {
          animation: aquatick-scale-in 0.2s forwards cubic-bezier(0.34, 1.56, 0.64, 1);
        }
        .aquatick-card-exit {
          animation: aquatick-scale-out 0.2s forwards ease-in;
        }
      ` }} />

      <ReminderOverlay isOpen={shouldRender} isMounted={isMounted}>
        <div ref={containerRef}>
          <ReminderCard isMounted={isMounted}>
            <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
              <h2
                id="aquatick-reminder-title"
                style={{
                  margin: 0,
                  fontSize: "24px",
                  fontWeight: 700,
                  color: "#89b4fa", // Pastel blue
                }}
              >
                Hydration Time
              </h2>
              <p
                id="aquatick-reminder-description"
                style={{
                  margin: 0,
                  fontSize: "16px",
                  lineHeight: 1.5,
                  color: "#cdd6f4", // Light text
                }}
              >
                Time to take a break and drink some water!
              </p>

              <ButtonGroup>
                <ActionButton
                  onClick={onDrink}
                  disabled={isProcessing}
                  variant="primary"
                >
                  Drink Water
                </ActionButton>
                <ActionButton
                  onClick={toggleSnooze}
                  disabled={isProcessing}
                  variant="secondary"
                >
                  {isSnoozeOpen ? "Cancel" : "Snooze"}
                </ActionButton>
                <ActionButton
                  onClick={onIgnore}
                  disabled={isProcessing}
                  variant="danger"
                >
                  Ignore
                </ActionButton>
              </ButtonGroup>

              <SnoozeSelector
                isOpen={isSnoozeOpen}
                onSelect={onSnooze}
                disabled={isProcessing}
              />

              {isProcessing && (
                <div
                  style={{
                    fontSize: "14px",
                    color: "#a6e3a1", // Pastel green
                    fontWeight: 500,
                    marginTop: "8px",
                  }}
                >
                  Processing action...
                </div>
              )}
            </div>
          </ReminderCard>
        </div>
      </ReminderOverlay>
    </>
  );
};


