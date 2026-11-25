//! A club's commercial side on the league economy: its sponsors, slot by slot, the merchandise
//! range it sells, the hospitality it offers in its boxes and premium seats, and the fan
//! experiences it runs.
//!
//! Data only. What each earns is worked out in `ofm_core::commercial`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A place a sponsor's name goes. Every club has one deal in each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SponsorSlot {
    Shirt,
    KitSupplier,
    StadiumNaming,
    Sleeve,
    TrainingKit,
    TrainingGround,
    Partners,
}

impl SponsorSlot {
    /// Every slot, in the order a club's sponsors are listed.
    pub const ALL: [SponsorSlot; 7] = [
        SponsorSlot::Shirt,
        SponsorSlot::KitSupplier,
        SponsorSlot::StadiumNaming,
        SponsorSlot::Sleeve,
        SponsorSlot::TrainingKit,
        SponsorSlot::TrainingGround,
        SponsorSlot::Partners,
    ];
}

/// One sponsor's deal with a club.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SponsorDeal {
    pub slot: SponsorSlot,
    /// A fictional company: world data, like a club's or a stand's name, not interface text.
    pub partner: String,
    /// Euros a season, paid weekly.
    pub annual_value: i64,
    /// The last season the deal runs; it is renegotiated at that season's end.
    pub last_season: u32,
}

/// The merchandise range a club stocks. Staff run the stock; the manager picks the range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MerchandiseRange {
    /// Cheap and steady: sells whatever the team does.
    Basic,
    #[default]
    Standard,
    /// Dear and fashionable: sells best when the team is winning.
    Premium,
}

/// The hospitality a club sells in its executive boxes and premium seats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum HospitalityPackage {
    /// Low prices that fill every box.
    Basic,
    #[default]
    Standard,
    /// High prices that only a famous club can fill.
    Premium,
}

/// The fan experiences a club runs, each with its own cost and takings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FanExperiences {
    #[serde(default)]
    pub stadium_tours: bool,
    #[serde(default)]
    pub museum: bool,
    #[serde(default)]
    pub matchday_events: bool,
}

/// A club's commercial set-up.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Commercial {
    #[serde(default)]
    pub sponsors: Vec<SponsorDeal>,
    #[serde(default)]
    pub merchandise: MerchandiseRange,
    #[serde(default)]
    pub hospitality: HospitalityPackage,
    #[serde(default)]
    pub fan_experiences: FanExperiences,
}
