import { useEffect, useState } from "react";

import { fetchGround } from "../../services/groundService";
import type { GroundView } from "../../store/generated/GroundView";

export type GroundState =
  | { status: "loading" }
  | { status: "ready"; view: GroundView }
  | { status: "failed"; reason: string };

/** The manager's club's ground, asked for again whenever `careerKey` (date and career) changes. */
export function useGround(careerKey: string): GroundState {
  const [state, setState] = useState<GroundState>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;
    fetchGround()
      .then((view) => {
        if (!cancelled) setState({ status: "ready", view });
      })
      .catch((reason: unknown) => {
        // Commands refuse with a translation key; anything else is shown as the screen's own error.
        if (!cancelled) setState({ status: "failed", reason: String(reason) });
      });
    return () => {
      cancelled = true;
    };
  }, [careerKey]);

  return state;
}
