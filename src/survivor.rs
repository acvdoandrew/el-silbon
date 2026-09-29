//! The four people of the llano a player can be, and how a party shares
//! them out: each player asks for one, and whoever hosts gives each their
//! wish unless someone already has it, then the first one still free.
//! Who you are is only how the others see you; it changes nothing else.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Survivor {
    /// Ranch hand, in a cream liquiliqui and a black hat.
    #[default]
    Llanero,
    /// Song keeper, in a white blouse and a flowered red skirt.
    Coplera,
    /// The old ranch caretaker, straw hat and rubber boots.
    Encargado,
    /// A young local, plaid shirt over a tee, jeans.
    Muchacho,
}

impl Survivor {
    pub const ALL: [Survivor; 4] = [
        Survivor::Llanero,
        Survivor::Coplera,
        Survivor::Encargado,
        Survivor::Muchacho,
    ];

    pub fn code(self) -> u8 {
        self as u8
    }

    /// The command line's name for them.
    pub fn key(self) -> &'static str {
        match self {
            Survivor::Llanero => "llanero",
            Survivor::Coplera => "coplera",
            Survivor::Encargado => "encargado",
            Survivor::Muchacho => "muchacho",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.key() == s)
    }

    /// An unknown code (a newer build's) falls back to the first.
    pub fn from_code(code: u8) -> Self {
        Self::ALL.get(usize::from(code)).copied().unwrap_or_default()
    }

    pub fn name(self) -> &'static str {
        match self {
            Survivor::Llanero => "El Llanero",
            Survivor::Coplera => "La Coplera",
            Survivor::Encargado => "El Encargado",
            Survivor::Muchacho => "El Muchacho",
        }
    }

    pub fn role(self) -> &'static str {
        match self {
            Survivor::Llanero => "Ranch hand",
            Survivor::Coplera => "Song keeper",
            Survivor::Encargado => "Ranch caretaker",
            Survivor::Muchacho => "Young local",
        }
    }

    /// The next one along (the menu's ‹ ›).
    pub fn cycle(self, step: i32) -> Self {
        let n = Self::ALL.len() as i32;
        Self::ALL[(i32::from(self.code()) + step).rem_euclid(n) as usize]
    }

    /// Who a joining player becomes: their wish when nobody has it, else the
    /// first one free (a full party of four always has one left for each).
    pub fn assign(wanted: Survivor, taken: &[Survivor]) -> Survivor {
        if !taken.contains(&wanted) {
            return wanted;
        }
        Self::ALL.into_iter().find(|s| !taken.contains(s)).unwrap_or(wanted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_party_gets_four_different_people_and_wishes_come_first() {
        // Everyone wants the llanero: the first gets him, the rest what is free.
        let mut party = Vec::new();
        for _ in 0..4 {
            let s = Survivor::assign(Survivor::Llanero, &party);
            assert!(!party.contains(&s));
            party.push(s);
        }
        assert_eq!(party[0], Survivor::Llanero);
        // A free wish is granted as asked.
        assert_eq!(
            Survivor::assign(Survivor::Muchacho, &[Survivor::Llanero]),
            Survivor::Muchacho
        );
        // Codes survive the wire; an unknown one is still someone.
        for s in Survivor::ALL {
            assert_eq!(Survivor::from_code(s.code()), s);
        }
        assert_eq!(Survivor::from_code(200), Survivor::Llanero);
        assert_eq!(Survivor::Llanero.cycle(-1), Survivor::Muchacho);
        assert_eq!(Survivor::Muchacho.cycle(1), Survivor::Llanero);
    }
}
