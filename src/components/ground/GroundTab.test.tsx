import { render, screen, waitFor, within } from "@testing-library/react";
import { createInstance } from "i18next";
import { beforeEach, describe, expect, it, vi } from "vitest";

import en from "../../i18n/locales/en.json";
import { fetchGround } from "../../services/groundService";
import type { GroundView } from "../../store/generated/GroundView";
import type { Section } from "../../store/generated/Section";
import type { Stand } from "../../store/generated/Stand";
import GroundTab from "./GroundTab";

const translations = createInstance();
await translations.init({
  lng: "en",
  resources: { en: { translation: en } },
  interpolation: { escapeValue: false },
});
vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({ t: translations.t.bind(translations), i18n: translations }),
}));
vi.mock("../../services/groundService", () => ({ fetchGround: vi.fn() }));
const mockedFetchGround = vi.mocked(fetchGround);

const section = (overrides: Partial<Section> = {}): Section => ({
  tiers: [{ kind: "seated", rows: 22 }],
  roof: "cantilever",
  boxes: false,
  rake: "standard",
  balcony: false,
  ...overrides,
});
const stand = (name: string, bays: Section[]): Stand => ({ name, bays });

function ground(overrides: Partial<GroundView> = {}): GroundView {
  return {
    team_id: "team-1",
    stadium_name: "Canal Street",
    layout: {
      version: 1,
      bowl: false,
      roofSystem: { type: "stands", retractable: false, closed: false },
      floodlights: "pylons",
      cladding: "brick",
      sides: {
        north: stand("Main Stand", [
          section({
            tiers: [
              { kind: "seated", rows: 22 },
              { kind: "rail", rows: 14 },
            ],
          }),
        ]),
        south: stand("Paddock", [section({ roof: "posts" }), section({ roof: "none" })]),
        west: stand("Kop", [section({ tiers: [{ kind: "terrace", rows: 30 }], roof: "goalpost" })]),
        east: stand("Canal End", [section({ tiers: [], roof: "none" })]),
      },
      corners: { nw: "open", ne: "open", sw: "open", se: "open" },
      pitch: { surface: "hybrid", undersoilHeating: true, growLights: false },
    },
    figures: {
      capacity: 18240,
      seats: 14000,
      railSeats: 1600,
      standing: 4240,
      boxSeats: 348,
      executiveBoxes: 29,
      covered: 13680,
      wellCovered: 11000,
      restrictedView: 610,
      distantView: 0,
      poorView: 0,
      atmosphere: 64,
    },
    ...overrides,
  };
}

const renderTab = () => render(<GroundTab careerKey="2026-08-01|1" />);

describe("GroundTab", () => {
  beforeEach(() => {
    mockedFetchGround.mockReset();
  });

  /**
   * Given the manager's club has a ground
   * When the Ground screen opens
   * Then it shows the ground's name and the figures the game counted, as the backend sent them.
   */
  it("shows the ground's name and what it holds", async () => {
    mockedFetchGround.mockResolvedValueOnce(ground());

    renderTab();

    expect(await screen.findByRole("heading", { name: "Canal Street" })).toBeInTheDocument();
    const figures = screen.getByRole("list", { name: "Ground" });
    const value = (label: string) =>
      within(figures).getByText(label).closest("li")?.textContent ?? "";
    expect(value("Capacity")).toContain("18,240");
    expect(value("Standing")).toContain("4,240");
    expect(value("Executive boxes")).toContain("29");
    expect(value("Under cover")).toContain("13,680 (75%)");
    expect(value("Atmosphere")).toContain("64 / 100");
    expect(mockedFetchGround).toHaveBeenCalledWith();
  });

  /**
   * Given a ground of four stands with mixed roofs and an open bank
   * When the Ground screen shows its stands
   * Then each stand is a row with its sections, its tiers and its roof, a stand whose sections
   *      differ saying so, and an unbuilt end showing as an open bank.
   */
  it("lists each stand with its sections, tiers and roof", async () => {
    mockedFetchGround.mockResolvedValueOnce(ground());

    renderTab();

    const table = await screen.findByRole("table", { name: "Stands" });
    const row = (name: string) => within(table).getByRole("row", { name: new RegExp(name) });
    expect(row("Main Stand")).toHaveTextContent("Seated 22");
    expect(row("Main Stand")).toHaveTextContent("Rail seats 14");
    expect(row("Main Stand")).toHaveTextContent("Cantilever");
    expect(row("Paddock")).toHaveTextContent("2");
    expect(row("Paddock")).toHaveTextContent("Mixed");
    expect(row("Kop")).toHaveTextContent("Terrace 30");
    expect(row("Canal End")).toHaveTextContent("Open bank");
  });

  /**
   * Given a hybrid pitch with undersoil heating and no grow lights
   * When the Ground screen shows the pitch
   * Then it says so.
   */
  it("shows the pitch and what is installed under and over it", async () => {
    mockedFetchGround.mockResolvedValueOnce(ground());

    renderTab();

    const pitch = await screen.findByRole("list", { name: "Pitch" });
    expect(within(pitch).getByText("Hybrid grass")).toBeInTheDocument();
    expect(within(pitch).getByText("Undersoil heating").closest("li")).toHaveTextContent(
      "Installed",
    );
    expect(within(pitch).getByText("Grow lights").closest("li")).toHaveTextContent("None");
  });

  /**
   * Given a ground with a roof over the whole ground
   * When the Ground screen opens
   * Then it names that roof among the figures.
   */
  it("names a roof over the whole ground", async () => {
    const view = ground();
    view.layout.roofSystem = { type: "arch", retractable: true, closed: false };
    mockedFetchGround.mockResolvedValueOnce(view);

    renderTab();

    expect(await screen.findByText("Arch roof over the whole ground")).toBeInTheDocument();
    expect(screen.getByText("Retractable")).toBeInTheDocument();
  });

  /**
   * Given the backend cannot find the ground
   * When the Ground screen opens
   * Then the player reads the translated reason, not a blank screen.
   */
  it("shows the backend's reason when the ground cannot be loaded", async () => {
    mockedFetchGround.mockRejectedValueOnce("be.error.teamNotFound");

    renderTab();

    expect(await screen.findByRole("alert")).toHaveTextContent("Team not found");
  });

  /**
   * Given the screen is open
   * When the career moves on (another day or another career)
   * Then the ground is asked for again, since building may have changed it.
   */
  it("asks again when the career moves on", async () => {
    mockedFetchGround.mockResolvedValue(ground());
    const { rerender } = renderTab();
    await screen.findByRole("heading", { name: "Canal Street" });

    rerender(<GroundTab careerKey="2026-08-02|1" />);

    await waitFor(() => expect(mockedFetchGround).toHaveBeenCalledTimes(2));
  });
});
