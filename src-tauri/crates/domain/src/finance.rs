use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::Arc;

use crate::team::FinancialTransactionKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CashPost {
    pub id: String,
    pub club_id: String,
    /// Positive is income, negative is expense. Integer euros.
    pub amount: i64,
    pub kind: CashKind,
    /// Game-clock date (`YYYY-MM-DD`), never wall-clock.
    pub date: String,
}

/// Chart of accounts. Additive — never `#[serde(other)]`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CashKind {
    OpeningBalance,
    PlayerWages,
    StaffWages,
    Matchday,
    Sponsorship,
    CommercialCampaign,
    PrizeMoney,
    TransferFeeOut,
    TransferFeeIn,
    LoanFee,
    ContractTermination,
    BoardSupport,
    Facilities,
    Upkeep,
    Other,
}

impl CashKind {
    pub fn counts_toward_season_totals(self) -> bool {
        !matches!(
            self,
            Self::OpeningBalance | Self::TransferFeeIn | Self::TransferFeeOut | Self::LoanFee
        )
    }

    pub fn from_legacy_ledger(kind: FinancialTransactionKind) -> Self {
        match kind {
            FinancialTransactionKind::PrizeMoney => Self::PrizeMoney,
            FinancialTransactionKind::ContractTermination => Self::ContractTermination,
            FinancialTransactionKind::BoardSupport => Self::BoardSupport,
            FinancialTransactionKind::CommercialCampaign => Self::CommercialCampaign,
        }
    }
}

/// Append-only cash journal. `Clone` is a pointer bump; writers copy-on-write.
///
/// A career adds tens of thousands of posts a season, so the journal also keeps where each
/// club's posts are and which ids it holds: asking about one club or one post never scans
/// every post the world has made. Both are derived from `posts` and rebuilt with it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CashJournal {
    posts: Arc<Vec<CashPost>>,
    /// Each club's posts, as positions in `posts`, in posting order.
    by_club: Arc<HashMap<String, Vec<usize>>>,
    /// Every post's id. Post ids are unique keys, so this is a set of them, nothing more.
    ids: Arc<HashSet<String>>,
}

impl Deref for CashJournal {
    type Target = [CashPost];

    fn deref(&self) -> &[CashPost] {
        &self.posts
    }
}

impl CashJournal {
    pub fn from_vec(posts: Vec<CashPost>) -> Self {
        let mut journal = Self::default();
        journal.extend(posts);
        journal
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.posts, &other.posts)
    }

    pub fn contains_club(&self, club_id: &str) -> bool {
        self.by_club.contains_key(club_id)
    }

    /// Whether a post with this id has been made.
    pub fn contains_post(&self, post_id: &str) -> bool {
        self.ids.contains(post_id)
    }

    /// One club's posts, in the order they were made.
    pub fn posts_for<'a>(&'a self, club_id: &str) -> impl Iterator<Item = &'a CashPost> + 'a {
        self.by_club
            .get(club_id)
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|&position| &self.posts[position])
    }

    pub fn extend(&mut self, posts: impl IntoIterator<Item = CashPost>) {
        let extra: Vec<CashPost> = posts.into_iter().collect();
        if extra.is_empty() {
            return;
        }
        let first = self.posts.len();
        let by_club = Arc::make_mut(&mut self.by_club);
        let ids = Arc::make_mut(&mut self.ids);
        for (offset, post) in extra.iter().enumerate() {
            by_club
                .entry(post.club_id.clone())
                .or_default()
                .push(first + offset);
            ids.insert(post.id.clone());
        }
        Arc::make_mut(&mut self.posts).extend(extra);
    }

    /// Sum of posts for one club. Folded in `i128` so prefix order cannot panic.
    pub fn cash_for(&self, club_id: &str) -> i64 {
        let sum = self
            .posts_for(club_id)
            .fold(0i128, |acc, post| acc + i128::from(post.amount));
        i64::try_from(sum).expect("journal overflow is a bug: post_all rejected it")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cash_kind_serializes_as_snake_case() {
        let json = serde_json::to_string(&CashKind::TransferFeeOut).unwrap();
        assert_eq!(json, "\"transfer_fee_out\"");
        let back: CashKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, CashKind::TransferFeeOut);
    }

    #[test]
    fn unknown_kind_fails_to_deserialize() {
        let err = serde_json::from_str::<CashKind>("\"not_a_kind\"").unwrap_err();
        assert!(err.to_string().contains("unknown variant"));
    }

    fn post(id: &str, amount: i64) -> CashPost {
        CashPost {
            id: id.to_string(),
            club_id: "alpha".to_string(),
            amount,
            kind: CashKind::Other,
            date: "2026-02-16".to_string(),
        }
    }

    #[test]
    fn cash_for_does_not_depend_on_prefix_order() {
        let credits_first =
            CashJournal::from_vec(vec![post("1", i64::MAX), post("2", 1), post("3", -1)]);
        let debit_first =
            CashJournal::from_vec(vec![post("1", i64::MAX), post("3", -1), post("2", 1)]);
        assert_eq!(credits_first.cash_for("alpha"), i64::MAX);
        assert_eq!(debit_first.cash_for("alpha"), i64::MAX);
    }

    /// Given a journal read from a save and then added to,
    /// When it is asked whether a post id is in it,
    /// Then it knows both the saved posts and the added ones, and no others.
    #[test]
    fn the_journal_knows_which_post_ids_it_holds() {
        let mut journal = CashJournal::from_vec(vec![post("saved", 10)]);
        journal.extend(vec![post("added", 5)]);

        assert!(journal.contains_post("saved"));
        assert!(journal.contains_post("added"));
        assert!(!journal.contains_post("never-posted"));
    }

    /// Given a journal holding two clubs' posts, read from a save and then added to,
    /// When one club's posts are asked for,
    /// Then they are that club's, in the order they were posted, and its cash adds them up.
    #[test]
    fn a_clubs_posts_are_its_own_in_posting_order() {
        let other = |id: &str, amount: i64| CashPost {
            club_id: "bravo".to_string(),
            ..post(id, amount)
        };
        let mut journal = CashJournal::from_vec(vec![post("a1", 10), other("b1", 7)]);
        journal.extend(vec![other("b2", 1), post("a2", -3)]);

        let alpha: Vec<&str> = journal
            .posts_for("alpha")
            .map(|post| post.id.as_str())
            .collect();

        assert_eq!(alpha, ["a1", "a2"]);
        assert_eq!(journal.cash_for("alpha"), 7);
        assert_eq!(journal.posts_for("nobody").count(), 0);
    }
}
