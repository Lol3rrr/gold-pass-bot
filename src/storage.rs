use std::{
    collections::{BTreeMap, HashMap, HashSet},
    future::Future,
    pin::Pin,
};

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use crate::{ClanTag, ClanWarLeagueSeason, PlayerTag, Time};

mod files;
pub use files::FileStorage;

mod s3;
pub use s3::S3Storage;

mod replicated;
pub use replicated::Replicated;

pub trait StorageBackend: Send {
    fn write(
        &mut self,
        content: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'static>>;
    fn load(&mut self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ()>> + Send + 'static>>;
}

impl<S> StorageBackend for Box<S>
where
    S: StorageBackend,
{
    fn write(
        &mut self,
        content: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'static>> {
        S::write(self.as_mut(), content)
    }

    fn load(&mut self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ()>> + Send + 'static>> {
        S::load(self.as_mut())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Storage {
    clans: HashMap<ClanTag, HashMap<Season, ClanStorage>>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Season {
    pub year: usize,
    pub month: usize,
}

impl<'de> Deserialize<'de> for Season {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;

        let (raw_year, raw_month) = raw.split_once('-').ok_or(serde::de::Error::custom(""))?;

        let year = raw_year.parse().map_err(|e| serde::de::Error::custom(e))?;
        let month = raw_month.parse().map_err(|e| serde::de::Error::custom(e))?;

        Ok(Self { year, month })
    }
}
impl Serialize for Season {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = format!("{:04}-{:02}", self.year, self.month);
        raw.serialize(serializer)
    }
}

impl From<Time> for Season {
    fn from(value: crate::Time) -> Self {
        Self {
            year: value.year,
            month: value.month,
        }
    }
}
impl From<ClanWarLeagueSeason> for Season {
    fn from(value: ClanWarLeagueSeason) -> Self {
        Self {
            year: value.year,
            month: value.month,
        }
    }
}

impl Season {
    pub fn current() -> Self {
        let now = chrono::Utc::now();
        Self {
            year: now.year() as usize,
            month: now.month() as usize,
        }
    }

    pub fn previous(&self) -> Self {
        let mut year = self.year;
        let mut month = self.month - 1;
        if month < 1 {
            year -= 1;
            month = 12;
        }
        Self { year, month }
    }
}

/// All the Stats for a single Clan
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ClanStorage {
    /// All the CWL related Stats
    pub cwl: CwlStats,
    /// All the War related Stats
    pub wars: BTreeMap<Time, WarStats>,
    pub games: HashMap<PlayerTag, PlayerGamesStats>,
    pub raid_weekend: BTreeMap<Time, RaidWeekendStats>,
    pub player_names: HashMap<PlayerTag, String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct PlayerGamesStats {
    pub start_score: Option<usize>,
    pub end_score: usize,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CwlStats {
    pub wars: Vec<CwlWarStats>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CwlWarStats {
    pub members: HashMap<PlayerTag, MemberWarStats>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WarStats {
    pub start_time: Time,
    pub members: HashMap<PlayerTag, MemberWarStats>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemberWarStats {
    pub attacks: Vec<WarAttack>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WarAttack {
    pub destruction: usize,
    pub stars: usize,
    pub duration: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RaidWeekendStats {
    pub start_time: Time,
    pub members: HashMap<PlayerTag, RaidMember>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RaidMember {
    pub looted: usize,
}

impl Storage {
    pub fn empty() -> Self {
        Self {
            clans: HashMap::new(),
        }
    }

    pub fn register_clan(&mut self, tag: ClanTag) {
        if self.clans.contains_key(&tag) {
            return;
        }

        self.clans.insert(tag, HashMap::new());
    }

    pub fn get_mut(&mut self, tag: &ClanTag, season: &Season) -> Option<&mut ClanStorage> {
        self.clans.get_mut(tag).map(|seasons| {
            if !seasons.contains_key(season) {
                seasons.insert(season.clone(), ClanStorage::default());
            }

            seasons.get_mut(season).unwrap()
        })
    }

    pub fn get(&self, tag: &ClanTag, season: &Season) -> Option<&ClanStorage> {
        self.clans.get(tag).and_then(|s| s.get(season))
    }

    pub async fn load(store: &mut dyn StorageBackend) -> Result<Self, ()> {
        let content = store.load().await.map_err(|e| ())?;
        serde_json::from_slice(&content).map_err(|e| ())
    }

    pub async fn save(&self, store: &mut dyn StorageBackend) -> Result<(), ()> {
        let content = serde_json::to_vec(&self).map_err(|e| {
            tracing::error!("Serializing {:?}", e);
            ()
        })?;

        store.write(content).await.map_err(|e| {
            tracing::error!("Storing {:?}", e);
            ()
        })
    }
}

#[derive(Debug)]
pub struct PlayerSummary {
    pub cwl_stars: usize,
    pub war_stars: usize,
    pub raid_loot: usize,
    pub games_score: usize,
}

impl ClanStorage {
    pub fn players_summary(&self) -> impl Iterator<Item = (PlayerTag, PlayerSummary)> + '_ {
        // TODO
        // Get all the players we have some data for
        let players: HashSet<PlayerTag> = self.player_names.keys().cloned().collect();

        players.into_iter().map(|ptag| {
            let cwl_stars: usize = self
                .cwl
                .wars
                .iter()
                .map(|war| {
                    war.members
                        .get(&ptag)
                        .map(|mstats| mstats.attacks.iter().map(|a| a.stars).sum::<usize>())
                        .unwrap_or(0)
                })
                .sum();

            let war_stars: usize = self
                .wars
                .values()
                .map(|war| {
                    war.members
                        .get(&ptag)
                        .map(|mstats| mstats.attacks.iter().map(|att| att.stars).sum::<usize>())
                        .unwrap_or(0)
                })
                .sum();

            let raid_loot: usize = self
                .raid_weekend
                .values()
                .map(|raid| {
                    raid.members
                        .get(&ptag)
                        .map(|rstats| rstats.looted)
                        .unwrap_or(0)
                })
                .sum();

            let games_score = self
                .games
                .get(&ptag)
                .map(|s| s.end_score - s.start_score.unwrap_or(s.end_score))
                .unwrap_or(0);

            (
                ptag,
                PlayerSummary {
                    cwl_stars,
                    war_stars,
                    raid_loot,
                    games_score,
                },
            )
        })
    }
}
