import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import type { GroundView } from "../../store/generated/GroundView";
import type { Stand } from "../../store/generated/Stand";
import { Card, CardBody, CardHeader } from "../ui";
import { percentOf, representativeTiers, standRoof } from "./GroundTab.helpers";
import { useGround } from "./useGround";

interface GroundTabProps {
  /** Changes when the date or the career does, so the screen asks for the ground again. */
  careerKey: string;
}

export default function GroundTab({ careerKey }: GroundTabProps) {
  const { t } = useTranslation();
  const ground = useGround(careerKey);

  if (ground.status === "loading") {
    return <p className="text-body text-fg-muted">{t("common.loading")}</p>;
  }
  if (ground.status === "failed") {
    return (
      <p role="alert" className="text-body text-danger">
        {t(ground.reason)}
      </p>
    );
  }
  return (
    <div className="space-y-5">
      <GroundFigures view={ground.view} />
      <StandsTable view={ground.view} />
      <PitchCard view={ground.view} />
    </div>
  );
}

function Figure({ label, children }: { label: string; children: ReactNode }) {
  return (
    <li className="rounded-lg bg-sunken px-4 py-3">
      <span className="block text-caption text-fg-muted">{label}</span>
      <span className="block font-heading text-title font-bold text-fg">{children}</span>
    </li>
  );
}

function GroundFigures({ view }: { view: GroundView }) {
  const { t, i18n } = useTranslation();
  const number = new Intl.NumberFormat(i18n.language);
  const { figures, layout } = view;
  const share = (count: number) =>
    t("ground.shareOfCapacity", {
      count: number.format(count),
      percent: percentOf(count, figures.capacity),
    });
  const roofSystem = layout.roofSystem;
  return (
    <Card accent="primary">
      <CardBody className="space-y-4">
        <div>
          <p className="font-heading text-caption font-bold uppercase tracking-label text-fg-muted">
            {t("ground.title")}
          </p>
          <h2 className="font-heading text-title font-bold uppercase tracking-label text-fg">
            {view.stadium_name}
          </h2>
          {roofSystem.type !== "stands" && (
            <p className="text-body text-fg-muted">
              {t(`ground.groundRoof.${roofSystem.type}`)}
              {roofSystem.retractable && (
                <span className="ml-2 rounded bg-info-tint px-2 py-0.5 text-caption text-info">
                  {t("ground.retractable")}
                </span>
              )}
            </p>
          )}
        </div>
        <ul
          aria-label={t("ground.title")}
          className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-6"
        >
          <Figure label={t("ground.capacity")}>{number.format(figures.capacity)}</Figure>
          <Figure label={t("ground.seats")}>{number.format(figures.seats)}</Figure>
          <Figure label={t("ground.standing")}>{number.format(figures.standing)}</Figure>
          <Figure label={t("ground.executiveBoxes")}>
            {number.format(figures.executiveBoxes)}
          </Figure>
          <Figure label={t("ground.underCover")}>{share(figures.covered)}</Figure>
          <Figure label={t("ground.restrictedView")}>{share(figures.restrictedView)}</Figure>
          <Figure label={t("ground.atmosphere")}>
            {t("ground.outOf100", { value: figures.atmosphere })}
          </Figure>
        </ul>
      </CardBody>
    </Card>
  );
}

function StandsTable({ view }: { view: GroundView }) {
  const { t } = useTranslation();
  const { sides } = view.layout;
  // Main stand, the far side, then the two ends: the order a ground is read in.
  const stands: Stand[] = [sides.north, sides.south, sides.west, sides.east];
  const cell = "px-4 py-2 text-left";
  return (
    <Card>
      <CardHeader>
        <span id="ground-stands-heading">{t("ground.stands")}</span>
      </CardHeader>
      <div className="overflow-x-auto">
        <table aria-labelledby="ground-stands-heading" className="w-full text-body text-fg">
          <thead className="text-caption text-fg-muted">
            <tr className="border-b border-line">
              <th scope="col" className={cell}>
                {t("ground.stand")}
              </th>
              <th scope="col" className={cell}>
                {t("ground.sections")}
              </th>
              <th scope="col" className={cell}>
                {t("ground.tiersRows")}
              </th>
              <th scope="col" className={cell}>
                {t("ground.roof")}
              </th>
            </tr>
          </thead>
          <tbody>
            {stands.map((stand) => (
              <StandRow key={stand.name} stand={stand} cell={cell} />
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
}

function StandRow({ stand, cell }: { stand: Stand; cell: string }) {
  const { t } = useTranslation();
  const tiers = representativeTiers(stand);
  const roof = standRoof(stand);
  return (
    <tr className="border-b border-line last:border-b-0">
      <th scope="row" className={`${cell} font-semibold`}>
        {stand.name}
      </th>
      <td className={cell}>{stand.bays.length}</td>
      <td className={cell}>
        {tiers.length === 0
          ? t("ground.openBank")
          : tiers
              .map((tier) =>
                t("ground.tier", { kind: t(`ground.tierKind.${tier.kind}`), rows: tier.rows }),
              )
              .join(" · ")}
      </td>
      <td className={cell}>
        {roof === "mixed" ? t("ground.roofMixed") : t(`ground.roofKind.${roof}`)}
      </td>
    </tr>
  );
}

function PitchCard({ view }: { view: GroundView }) {
  const { t } = useTranslation();
  const { pitch } = view.layout;
  const installed = (present: boolean) =>
    present ? t("ground.installed") : t("ground.notInstalled");
  return (
    <Card>
      <CardHeader>{t("ground.pitch")}</CardHeader>
      <CardBody>
        <ul aria-label={t("ground.pitch")} className="grid grid-cols-1 gap-3 md:grid-cols-3">
          <Figure label={t("ground.surface")}>{t(`ground.surfaceKind.${pitch.surface}`)}</Figure>
          <Figure label={t("ground.undersoilHeating")}>{installed(pitch.undersoilHeating)}</Figure>
          <Figure label={t("ground.growLights")}>{installed(pitch.growLights)}</Figure>
        </ul>
      </CardBody>
    </Card>
  );
}
