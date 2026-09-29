//! The night's awards: small honours (and dishonours) for each player,
//! shown on the outcome screen. Pure: the session counts the deeds, this
//! decides who earned what, and the outcome card names them.

use serde::{Deserialize, Serialize};

pub type PlayerId = u64;

/// What one player did tonight, counted by the session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Deeds {
    /// Frights that froze them (sustos).
    pub sustos: u16,
    /// Bundles put down or let fall.
    pub drops: u16,
    /// Bundles laid at the ceiba.
    pub delivered: u16,
    pub downs: u16,
    /// Teammates they got back on their feet.
    pub revives: u16,
    /// Peppers thrown.
    pub aji: u16,
    /// Times they set the cattle bellowing.
    pub cattle: u16,
    /// Times he began to warn them.
    pub warned: u16,
    /// Times they went into his sack.
    pub sacked: u16,
    /// Seconds into the night they first went down.
    pub first_down: Option<f32>,
}

/// An honour. Wording belongs to the outcome card, not here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Award {
    LeftBehind,
    SackRider,
    FirstToFall,
    Screamer,
    Butterfingers,
    HisFavourite,
    GuardianAngel,
    BoneBearer,
    PepperHand,
    Stampede,
    TurecosFriend,
    Untouched,
}

/// Everything the awards look at.
pub struct Night<'a> {
    pub party: &'a [(PlayerId, Deeds)],
    pub left_behind: &'a [PlayerId],
    /// Whoever untied Tureco.
    pub dog_friend: Option<PlayerId>,
}

/// At most this many awards each, the rarer first.
pub const PER_PLAYER: usize = 2;

/// Who earned what tonight, in the order to show them. Every award has a
/// floor, so nothing is handed out for doing nothing; the "most" awards go
/// to one player (ties to the earliest in the party); awards that compare
/// players need company.
pub fn awards(night: &Night) -> Vec<(PlayerId, Award)> {
    let shared = night.party.len() > 1;
    let mut out: Vec<(PlayerId, Award)> = Vec::new();
    let give = |id: PlayerId, award: Award, out: &mut Vec<(PlayerId, Award)>| {
        if out.iter().filter(|(p, _)| *p == id).count() < PER_PLAYER {
            out.push((id, award));
        }
    };
    // The one with the most of something, if it reaches `floor`.
    let most = |f: &dyn Fn(&Deeds) -> u16, floor: u16| -> Option<PlayerId> {
        night
            .party
            .iter()
            .filter(|(_, d)| f(d) >= floor)
            .fold(None::<(PlayerId, u16)>, |best, (id, d)| match best {
                Some((_, v)) if v >= f(d) => best,
                _ => Some((*id, f(d))),
            })
            .map(|(id, _)| id)
    };

    for (id, _) in night.party {
        if night.left_behind.contains(id) {
            give(*id, Award::LeftBehind, &mut out);
        }
    }
    for (id, d) in night.party {
        if d.sacked > 0 {
            give(*id, Award::SackRider, &mut out);
        }
    }
    if shared
        && let Some((id, _)) = night
            .party
            .iter()
            .filter_map(|(id, d)| d.first_down.map(|t| (*id, t)))
            .fold(None::<(PlayerId, f32)>, |best, (id, t)| match best {
                Some((_, b)) if b <= t => best,
                _ => Some((id, t)),
            })
    {
        give(id, Award::FirstToFall, &mut out);
    }
    let comparisons: [(Award, &dyn Fn(&Deeds) -> u16, u16, bool); 7] = [
        (Award::Screamer, &|d| d.sustos, if shared { 2 } else { 3 }, false),
        (Award::Butterfingers, &|d| d.drops, 2, false),
        (Award::HisFavourite, &|d| d.warned, 3, false),
        (Award::GuardianAngel, &|d| d.revives, 1, true),
        (Award::BoneBearer, &|d| d.delivered, 2, true),
        (Award::PepperHand, &|d| d.aji, 2, false),
        (Award::Stampede, &|d| d.cattle, 1, false),
    ];
    for (award, f, floor, needs_company) in comparisons {
        if needs_company && !shared {
            continue;
        }
        if let Some(id) = most(f, floor) {
            give(id, award, &mut out);
        }
    }
    if let Some(id) = night.dog_friend.filter(|id| night.party.iter().any(|(p, _)| p == id)) {
        give(id, Award::TurecosFriend, &mut out);
    }
    for (id, d) in night.party {
        if d.sustos == 0 && d.downs == 0 && d.warned == 0 && !night.left_behind.contains(id) {
            give(*id, Award::Untouched, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn night<'a>(party: &'a [(PlayerId, Deeds)], left: &'a [PlayerId]) -> Night<'a> {
        Night {
            party,
            left_behind: left,
            dog_friend: None,
        }
    }

    #[test]
    fn nothing_is_given_for_nothing_and_the_untroubled_are_noticed() {
        let party = [(1, Deeds::default()), (2, Deeds::default())];
        let got = awards(&night(&party, &[]));
        assert_eq!(got, vec![(1, Award::Untouched), (2, Award::Untouched)]);
        // One fright is not a scream; a warning is not untroubled.
        let party = [(
            1,
            Deeds {
                sustos: 1,
                warned: 1,
                ..Deeds::default()
            },
        )];
        assert!(awards(&night(&party, &[])).is_empty());
    }

    #[test]
    fn the_most_goes_to_one_player_and_ties_to_the_earliest() {
        let scared = Deeds {
            sustos: 4,
            ..Deeds::default()
        };
        let party = [
            (
                1,
                Deeds {
                    sustos: 2,
                    ..Deeds::default()
                },
            ),
            (2, scared),
            (3, scared),
        ];
        let got = awards(&night(&party, &[]));
        let screamers: Vec<_> = got.iter().filter(|(_, a)| *a == Award::Screamer).collect();
        assert_eq!(screamers, vec![&(2, Award::Screamer)]);
        // Deterministic: the same night, the same awards.
        assert_eq!(got, awards(&night(&party, &[])));
    }

    #[test]
    fn nobody_gets_more_than_their_share_and_the_left_behind_are_named_first() {
        let busy = Deeds {
            sustos: 5,
            drops: 3,
            warned: 6,
            aji: 4,
            cattle: 2,
            sacked: 1,
            first_down: Some(100.0),
            downs: 1,
            ..Deeds::default()
        };
        let party = [(1, busy), (2, Deeds::default())];
        let got = awards(&night(&party, &[1]));
        let mine: Vec<Award> = got.iter().filter(|(p, _)| *p == 1).map(|(_, a)| *a).collect();
        assert_eq!(mine.len(), PER_PLAYER);
        assert_eq!(mine[0], Award::LeftBehind);
    }

    #[test]
    fn awards_that_compare_players_need_company() {
        let solo = [(
            1,
            Deeds {
                delivered: 5,
                first_down: Some(30.0),
                downs: 1,
                ..Deeds::default()
            },
        )];
        let got = awards(&night(&solo, &[]));
        assert!(
            !got.iter()
                .any(|(_, a)| matches!(a, Award::BoneBearer | Award::FirstToFall | Award::GuardianAngel)),
            "{got:?}"
        );
        let pair = [
            (
                1,
                Deeds {
                    delivered: 3,
                    first_down: Some(90.0),
                    downs: 1,
                    ..Deeds::default()
                },
            ),
            (
                2,
                Deeds {
                    delivered: 2,
                    first_down: Some(40.0),
                    downs: 1,
                    revives: 1,
                    ..Deeds::default()
                },
            ),
        ];
        let got = awards(&night(&pair, &[]));
        assert!(got.contains(&(1, Award::BoneBearer)));
        assert!(got.contains(&(2, Award::FirstToFall)));
    }
}
