/**
 * @file CharacterCanvas.tsx
 *
 * React host component for the Character System.
 *
 * # Responsibilities (this file)
 *   - Render a transparent, fixed-position <canvas> overlay.
 *   - Synchronise the canvas pixel buffer with the viewport size and DPI.
 *   - Tear down cleanly on unmount, halting any active render loop or animation.
 *   - Instantiate and hold the CharacterController instance.
 *   - Expose the public CharacterSystemAPI ref handle to parent components/Event Coordinator.
 *
 * # Non-responsibilities (enforced)
 *   - No direct drawing, clearing, or rendering of frames inside the component.
 *   - No direct animation or movement mathematics.
 *   - No business logic or backend Tauri subscriptions.
 */

import { useEffect, useRef, forwardRef, useImperativeHandle } from "react";
import type { CharacterSystemAPI } from "../types";
import { CharacterController } from "../core/CharacterController";

// ---------------------------------------------------------------------------
// Public handle type
// ---------------------------------------------------------------------------

/**
 * The ref handle exposed to parent components (such as the Event Coordinator).
 */
export interface CharacterCanvasHandle {
  /** The public Character System API instance, or null if not yet mounted. */
  readonly character: CharacterSystemAPI | null;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/**
 * CharacterCanvas
 *
 * Transparent full-viewport canvas overlay. Instantiates the CharacterController
 * on mount and exposes the CharacterSystemAPI ref.
 */
const CharacterCanvas = forwardRef<CharacterCanvasHandle>(
  function CharacterCanvas(_props, ref) {
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const controllerRef = useRef<CharacterController | null>(null);

    // Expose the CharacterSystemAPI through the forwarded ref.
    useImperativeHandle(
      ref,
      (): CharacterCanvasHandle => ({
        get character() {
          return controllerRef.current;
        },
      }),
      [],
    );

    useEffect(() => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      // Instantiate the framework-agnostic CharacterController
      const controller = new CharacterController(canvas);
      controllerRef.current = controller;

      /**
       * Synchronises the canvas pixel buffer with the current CSS layout
       * dimensions and device pixel ratio.
       */
      function syncCanvasSize(): void {
        if (!canvas) return;

        const dpr = window.devicePixelRatio || 1;
        const rect = canvas.getBoundingClientRect();

        const physicalW = Math.round(rect.width * dpr);
        const physicalH = Math.round(rect.height * dpr);

        if (canvas.width !== physicalW || canvas.height !== physicalH) {
          canvas.width = physicalW;
          canvas.height = physicalH;
        }
      }

      // Run once immediately so the buffer is sized before the first paint.
      syncCanvasSize();

      // Observe size changes to ensure DPI adjustments are made on window resize/zoom.
      const observer = new ResizeObserver(syncCanvasSize);
      observer.observe(canvas);

      return () => {
        observer.disconnect();
        // Clean up the controller, stopping loop, promises, and clearing canvas.
        controller.hide();
        controllerRef.current = null;
      };
    }, []);

    return (
      <canvas
        ref={canvasRef}
        style={{
          // Full-viewport overlay — sits above all page content.
          position: "fixed",
          top: 0,
          left: 0,
          width: "100%",
          height: "100%",
          // Prevent the canvas from blocking any UI interaction underneath.
          pointerEvents: "none",
          // Ensure block display.
          display: "block",
          // Render above all application UI layers.
          zIndex: 9999,
        }}
        aria-hidden="true"
      />
    );
  },
);

CharacterCanvas.displayName = "CharacterCanvas";

export default CharacterCanvas;
