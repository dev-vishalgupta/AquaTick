import { useEffect } from "react";
import { EventCoordinator } from "../features/coordinator";

export default function SystemOrchestrator() {
  useEffect(() => {
    const initCoordinator = async () => {
      try {
        await EventCoordinator.initialize();
      } catch (err) {
        console.error("SystemOrchestrator: failed to initialize EventCoordinator.", err);
      }
    };

    initCoordinator();

    return () => {
      EventCoordinator.destroy();
    };
  }, []);

  return null;
}
