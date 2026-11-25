import type { Section } from "../../store/generated/Section";
import type { Stand } from "../../store/generated/Stand";
import type { StandRoof } from "../../store/generated/StandRoof";
import type { Tier } from "../../store/generated/Tier";

/**
 * The tiers a stand is described by: its section with the most rows, since a stand's sections can
 * differ by a row or two where the ground was built to an exact capacity. Empty for an open bank.
 */
export function representativeTiers(stand: Stand): Tier[] {
  const rowsOf = (section: Section) => section.tiers.reduce((sum, tier) => sum + tier.rows, 0);
  const deepest = stand.bays.reduce<Section | null>(
    (best, section) => (best === null || rowsOf(section) > rowsOf(best) ? section : best),
    null,
  );
  return deepest?.tiers ?? [];
}

/** The stand's roof if every section shares it, or `"mixed"`. */
export function standRoof(stand: Stand): StandRoof | "mixed" {
  const roofs = new Set(stand.bays.map((section) => section.roof));
  const [only] = roofs;
  return roofs.size === 1 && only !== undefined ? only : "mixed";
}

/** A share of the capacity as a whole percentage, for display beside the count. */
export function percentOf(count: number, capacity: number): number {
  return capacity > 0 ? Math.round((count / capacity) * 100) : 0;
}
