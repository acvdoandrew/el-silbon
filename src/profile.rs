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
    /// The brightness calibration has been offered (once, on the first
    /// title screen; an older profile has not seen it).
    pub calibrated: bool,
    /// The Madrina's chapters heard (1..=`lore::CHAPTERS`), over every night.
    #[serde(default)]
    pub chapters: BTreeSet<u8>,
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
        p.settings.master = 0.4;
        p.settings.music = 0.25;
        p.settings.invert_y = true;
        p.settings.display_mode = Settings::default().display_mode.toggled();
        p.settings.brightness = 0.4;
        p.settings.contrast = 1.15;
        p.calibrated = true;
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
        // Nor has it been shown the calibration: it is offered once.
        assert!(!old.calibrated);
        // The old linear volume is not carried onto the new curve: that
        // player starts again from the default level, and keeps the rest.
        let linear = Profile::from_json(r#"{"settings":{"volume":1.0,"invert_y":true}}"#).expect("old volume");
        assert_eq!(linear.settings.master, Settings::default().master);
        assert!(linear.settings.invert_y);
        // Every earlier build saved "not fullscreen" without anyone choosing
        // it: that player gets the new default once, and keeps the rest.
        let windowed = Profile::from_json(r#"{"settings":{"fullscreen":false,"invert_y":true}}"#).expect("old window");
        assert_eq!(windowed.settings.display_mode, Settings::default().display_mode);
        assert!(windowed.settings.invert_y);
        // A broken one does not.
        assert!(Profile::from_json("{not json").is_none());
    }

    /// A whole profile as the 0.1.0-test.1 build saved it (every key it
    /// wrote, with a player's own choices in them).
    const TEST_1: &str = r#"{
      "settings": {"volume": 0.4, "sensitivity": 1.3, "captions": false, "invert_y": true,
                   "fov": 80.0, "brightness": 0.2, "head_bob": false, "fullscreen": false},
      "pages": [1, 4, 15],
      "tally": {"nights": 2, "escapes": 1, "banishments": 0, "caught": 1, "fastest": 610.5},
      "join": "100.64.0.7:5197",
      "survivor": "Coplera"
    }"#;

    #[test]
    fn a_testers_saved_profile_loads_into_this_build() {
        let p = Profile::from_json(TEST_1).expect("a test.1 profile loads");
        let fresh = Settings::default();
        // What the player chose and found is kept.
        assert_eq!(p.pages, BTreeSet::from([1, 4, 15]));
        assert_eq!((p.tally.nights, p.tally.escapes, p.tally.caught), (2, 1, 1));
        assert_eq!(p.tally.fastest, Some(610.5));
        assert_eq!(p.join, "100.64.0.7:5197");
        assert_eq!(p.survivor, crate::survivor::Survivor::Coplera);
        let s = &p.settings;
        assert_eq!((s.sensitivity, s.fov, s.brightness), (1.3, 80.0, 0.2));
        assert!(!s.captions && s.invert_y && !s.head_bob);
        // The level and the window it never chose start from this build's
        // defaults; the new settings are at theirs; calibration is offered.
        assert_eq!(s.master, fresh.master);
        assert_eq!(
            (s.music, s.ambience, s.effects),
            (fresh.music, fresh.ambience, fresh.effects)
        );
        assert_eq!(s.display_mode, fresh.display_mode);
        assert_eq!(s.contrast, fresh.contrast);
        assert!(!p.calibrated);
        // Saved again, it carries none of the old keys.
        let again = p.to_json();
        let saved: serde_json::Value = serde_json::from_str(&again).expect("saved as JSON");
        let keys = saved["settings"].as_object().expect("settings saved");
        assert!(!keys.contains_key("volume") && !keys.contains_key("fullscreen"));
        assert_eq!(Profile::from_json(&again), Some(p));
    }
}
