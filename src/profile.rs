//! What the game remembers between launches: the player's settings, the
//! pages of the tale they have found, and a tally of their nights.
//!
//! Stored as JSON in the user's data directory
//! (`$XDG_DATA_HOME/el-silbon/profile.json`, else
//! `~/.local/share/el-silbon/profile.json`). A missing or unreadable file
//! simply means a fresh profile; saving is best effort and never stops play.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app::Settings;

/// A tally of the nights played on this machine.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    pub nights: u32,
    pub escapes: u32,
    pub banishments: u32,
    pub caught: u32,
    /// Fastest win, in seconds of the night.
    pub fastest: Option<f32>,
}

/// How a night ended, for the tally.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ending {
    Escaped { seconds: f32 },
    Banished { seconds: f32 },
    Caught,
}

impl Tally {
    pub fn record(&mut self, ending: Ending) {
        self.nights += 1;
        let won = match ending {
            Ending::Escaped { seconds } => {
                self.escapes += 1;
                Some(seconds)
            }
            Ending::Banished { seconds } => {
                self.banishments += 1;
                Some(seconds)
            }
            Ending::Caught => {
                self.caught += 1;
                None
            }
        };
        if let Some(s) = won {
            self.fastest = Some(self.fastest.map_or(s, |f| f.min(s)));
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub settings: Settings,
    /// Page ids of the tale read, over every night.
    pub pages: BTreeSet<u8>,
    pub tally: Tally,
    /// The last host address joined, offered again next time.
    pub join: String,
    /// Who the player likes to be with friends.
    pub survivor: crate::survivor::Survivor,
}

impl Profile {
    pub fn from_json(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Where the profile lives on this machine (`%APPDATA%\el-silbon` on
/// Windows).
pub fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("APPDATA").filter(|_| cfg!(windows)).map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))?;
    Some(base.join("el-silbon").join("profile.json"))
}

/// The saved profile, or a fresh one.
pub fn load() -> Profile {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| Profile::from_json(&t))
        .unwrap_or_default()
}

/// Save the profile (best effort: an error is returned, never raised).
pub fn save(profile: &Profile) -> Result<(), String> {
    let path = path().ok_or("no home directory")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    // Write beside it and move into place, so a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, profile.to_json()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_survives_the_round_trip_and_tolerates_old_or_broken_files() {
        let mut p = Profile::default();
        p.settings.volume = 0.4;
        p.settings.invert_y = true;
        p.pages.extend([0, 3, 19]);
        p.tally.record(Ending::Escaped { seconds: 500.0 });
        p.tally.record(Ending::Caught);
        p.tally.record(Ending::Banished { seconds: 420.0 });
        let back = Profile::from_json(&p.to_json()).expect("round trip");
        assert_eq!(back, p);
        assert_eq!(back.tally.nights, 3);
        assert_eq!(back.tally.fastest, Some(420.0));
        // A file from an older build (fewer fields) still loads, with defaults.
        let old = Profile::from_json(r#"{"pages":[1,2]}"#).expect("old file");
        assert_eq!(old.pages.len(), 2);
        assert_eq!(old.settings, Settings::default());
        // A broken one does not.
        assert!(Profile::from_json("{not json").is_none());
    }
}
