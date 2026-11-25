import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { fetchGround } from "./groundService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("groundService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  /** Given no club named, when the ground is fetched, then the manager's club is asked for. */
  it("asks for the manager's club's ground by default", async () => {
    const view = { team_id: "t1" };
    mockedInvoke.mockResolvedValueOnce(view);

    await expect(fetchGround()).resolves.toBe(view);

    expect(mockedInvoke).toHaveBeenCalledWith("get_ground", { teamId: null });
  });

  /** Given a club named, when the ground is fetched, then that club's ground is asked for. */
  it("asks for a named club's ground", async () => {
    mockedInvoke.mockResolvedValueOnce({ team_id: "t2" });

    await fetchGround("t2");

    expect(mockedInvoke).toHaveBeenCalledWith("get_ground", { teamId: "t2" });
  });

  /** Given the backend refuses, when the ground is fetched, then its error key reaches the caller. */
  it("passes the backend's error key on", async () => {
    mockedInvoke.mockRejectedValueOnce("be.error.teamNotFound");

    await expect(fetchGround("nobody")).rejects.toBe("be.error.teamNotFound");
  });
});
