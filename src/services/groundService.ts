import { invoke } from "@tauri-apps/api/core";
import type { GroundView } from "../store/generated/GroundView";

/** A club's ground and what the game counts from it; the manager's club when none is named. */
export async function fetchGround(teamId?: string): Promise<GroundView> {
  return invoke<GroundView>("get_ground", { teamId: teamId ?? null });
}
