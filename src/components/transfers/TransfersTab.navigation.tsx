import { TrendingUp, ShoppingCart, Handshake, UserPlus } from "lucide-react";
import type { TransferTabView } from "./TransfersTab.model";
import type { TFunction } from "i18next";
import type { TransferCollections } from "./TransfersTab.model";
import type { PlayerData } from "../../store/gameStore";
interface Input {
  collections: TransferCollections;
  myListedPlayers: PlayerData[];
  t: TFunction<"translation", undefined>;
}
export function getTransferNavigation({ collections, myListedPlayers, t }: Input) {
  const { availablePlayers, freeAgentPlayers, playersWithOffers } = collections;
  const tabs: {
    id: TransferTabView;
    label: string;
    icon: React.ReactNode;
    count: number;
  }[] = [
    {
      id: "players",
      label: t("dashboard.players"),
      icon: <TrendingUp className="w-4 h-4" />,
      count: availablePlayers.length,
    },
    {
      id: "free_agents",
      label: t("transfers.freeAgents"),
      icon: <UserPlus className="w-4 h-4" />,
      count: freeAgentPlayers.length,
    },
    {
      id: "my_list",
      label: t("transfers.myTransferList"),
      icon: <ShoppingCart className="w-4 h-4" />,
      count: myListedPlayers.length,
    },
    {
      id: "offers",
      label: t("transfers.offers"),
      icon: <Handshake className="w-4 h-4" />,
      count: playersWithOffers.length,
    },
  ];
  return { tabs };
}
