import { useEffect, useRef } from "react";
import { CharacterController } from "../../character";
import type { CharacterScale } from "../../character";

interface CharacterPreviewProps {
  characterId: string;
  scale: CharacterScale;
}

export default function CharacterPreview({ characterId, scale }: CharacterPreviewProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const controllerRef = useRef<CharacterController | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const controller = new CharacterController(canvas);
    controllerRef.current = controller;

    const syncCanvasSize = () => {
      if (!canvas) return;
      const dpr = window.devicePixelRatio || 1;
      const rect = canvas.getBoundingClientRect();
      const physicalW = Math.round(rect.width * dpr);
      const physicalH = Math.round(rect.height * dpr);
      if (canvas.width !== physicalW || canvas.height !== physicalH) {
        canvas.width = physicalW;
        canvas.height = physicalH;
      }
    };

    syncCanvasSize();
    const observer = new ResizeObserver(syncCanvasSize);
    observer.observe(canvas);

    const loadCharacter = async () => {
      try {
        await controller.setCharacter(characterId);
        controller.setScale(scale);
        controller.show();
      } catch (err) {
        console.error("CharacterPreview: failed to load character", err);
      }
    };
    loadCharacter();

    return () => {
      observer.disconnect();
      controller.hide();
      controllerRef.current = null;
    };
  }, [characterId]);

  useEffect(() => {
    if (controllerRef.current) {
      controllerRef.current.setScale(scale);
    }
  }, [scale]);

  return (
    <div className="flex flex-col items-center justify-center p-4 border border-slate-200 dark:border-slate-800 rounded-xl bg-slate-50 dark:bg-slate-900/50 min-h-[180px] transition-all relative overflow-hidden">
      <div className="w-48 h-32 relative bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-lg shadow-sm overflow-hidden">
        <canvas
          ref={canvasRef}
          className="w-full h-full block"
          style={{ pointerEvents: "none" }}
        />
      </div>
      <div className="text-center space-y-1 mt-3">
        <p className="text-sm font-semibold text-slate-700 dark:text-slate-200">
          Character Viewport Preview
        </p>
        <p className="text-xs text-slate-500 dark:text-slate-400 font-mono">
          ID: {characterId} | Scale: {scale}
        </p>
      </div>
    </div>
  );
}
