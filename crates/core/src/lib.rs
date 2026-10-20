use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

pub const GOAL: u64 = 100;
pub const PUSH_LIMIT: usize = 100;
pub const PAGE_LIMIT: usize = 500;
// D1's JavaScript bindings represent integer parameters as doubles.
pub const MAX_SEQUENCE: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exercise {
    PushUps,
    Crunches,
    Squats,
}

impl Exercise {
    pub const ALL: [Self; 3] = [Self::PushUps, Self::Crunches, Self::Squats];
    pub fn index(self) -> usize {
        match self {
            Self::PushUps => 0,
            Self::Crunches => 1,
            Self::Squats => 2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::PushUps => "Push-ups",
            Self::Crunches => "Crunches",
            Self::Squats => "Squats",
        }
    }
}

impl std::str::FromStr for Exercise {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "push-ups" | "pushups" | "push_ups" => Ok(Self::PushUps),
            "crunches" => Ok(Self::Crunches),
            "squats" => Ok(Self::Squats),
            _ => Err("Choose pushups, crunches or squats"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    Set { exercise: Exercise, reps: u16 },
    Cancel { target: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: String,
    pub at: String,
    pub change: Change,
}

impl Event {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !valid_id(&self.id) {
            return Err("Invalid record identity");
        }
        DateTime::parse_from_rfc3339(&self.at).map_err(|_| "Invalid record time")?;
        match &self.change {
            Change::Set { reps, .. } if !(1..=1000).contains(reps) => {
                Err("Enter between 1 and 1000 reps")
            }
            Change::Cancel { target } if !valid_id(target) || target == &self.id => {
                Err("Invalid cancellation target")
            }
            _ => Ok(()),
        }
    }
    pub fn day(&self) -> NaiveDate {
        // Only validated records enter a Store or a server response.
        DateTime::parse_from_rfc3339(&self.at)
            .expect("validated timestamp")
            .date_naive()
    }
}

fn valid_id(id: &str) -> bool {
    Uuid::parse_str(id).is_ok_and(|uuid| uuid.to_string() == id && !uuid.is_nil())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncRequest {
    pub after: u64,
    pub events: Vec<Event>,
}

impl SyncRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.after > MAX_SEQUENCE || self.events.len() > PUSH_LIMIT {
            return Err("Sync batch is too large");
        }
        let mut ids = HashSet::new();
        for event in &self.events {
            event.validate()?;
            if !ids.insert(&event.id) {
                return Err("Repeated identity in sync batch");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteEvent {
    pub seq: u64,
    pub event: Event,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncResponse {
    pub acknowledged: Vec<String>,
    pub records: Vec<RemoteEvent>,
    pub next: u64,
    pub more: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub version: u8,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub days: BTreeMap<NaiveDate, [u64; 3]>,
    pub current_streak: usize,
    pub best_streak: usize,
    pub complete_days: usize,
}

pub fn active_sets(events: &[Event]) -> Vec<&Event> {
    let cancelled: HashSet<&str> = events
        .iter()
        .filter_map(|event| match &event.change {
            Change::Cancel { target } => Some(target.as_str()),
            _ => None,
        })
        .collect();
    let mut sets: Vec<_> = events
        .iter()
        .filter(|event| {
            matches!(event.change, Change::Set { .. }) && !cancelled.contains(event.id.as_str())
        })
        .collect();
    sets.sort_by(|a, b| {
        let a_time = DateTime::parse_from_rfc3339(&a.at).unwrap();
        let b_time = DateTime::parse_from_rfc3339(&b.at).unwrap();
        a_time.cmp(&b_time).then_with(|| a.id.cmp(&b.id))
    });
    sets
}

pub fn progress(events: &[Event], today: NaiveDate) -> Progress {
    let mut progress = Progress::default();
    for event in active_sets(events) {
        if let Change::Set { exercise, reps } = event.change {
            progress.days.entry(event.day()).or_default()[exercise.index()] += u64::from(reps);
        }
    }
    let complete: HashSet<_> = progress
        .days
        .iter()
        .filter(|(_, totals)| totals.iter().all(|total| *total >= GOAL))
        .map(|(day, _)| *day)
        .collect();
    progress.complete_days = complete.len();
    let mut current = if complete.contains(&today) {
        Some(today)
    } else {
        today.pred_opt()
    };
    while let Some(day) = current.filter(|day| complete.contains(day)) {
        progress.current_streak += 1;
        current = day.pred_opt();
    }
    let mut run = 0;
    let mut previous: Option<NaiveDate> = None;
    for day in progress.days.keys().filter(|day| complete.contains(day)) {
        run = if previous.is_some_and(|previous| previous.succ_opt() == Some(*day)) {
            run + 1
        } else {
            1
        };
        progress.best_streak = progress.best_streak.max(run);
        previous = Some(*day);
    }
    progress
}

pub fn remaining(total: u64) -> u64 {
    GOAL.saturating_sub(total)
}

/// Static character pigment: an interpretation of progress, not a plotted measurement.
pub fn pigment(totals: [u64; 3], width: usize, height: usize) -> Vec<Vec<(char, Option<usize>)>> {
    let ramp = b" .:-=+*#%@";
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    let xf = x as f64 / width as f64;
                    let yf = y as f64 / height as f64;
                    let centres = [(0.28, 0.35), (0.66, 0.40), (0.48, 0.70)];
                    let noise = ((x * 73 + y * 151 + x * y * 19) % 97) as f64 / 97.;
                    let (index, density) = centres
                        .iter()
                        .enumerate()
                        .map(|(i, (cx, cy))| {
                            let dx = xf - cx + (yf * 12. + i as f64).sin() * 0.05;
                            let dy = yf - cy;
                            let density = (-((dx / 0.22).powi(2) + (dy / 0.26).powi(2))).exp()
                                * (totals[i].min(GOAL) as f64 / GOAL as f64).sqrt();
                            (i, density)
                        })
                        .max_by(|a, b| a.1.total_cmp(&b.1))
                        .unwrap();
                    let level = (density * 9. + noise * 1.4 - 0.65).max(0.) as usize;
                    if density < 0.015 || level == 0 {
                        (' ', None)
                    } else {
                        (ramp[level.min(9)] as char, Some(index))
                    }
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set(n: u128, at: &str, exercise: Exercise, reps: u16) -> Event {
        Event {
            id: Uuid::from_u128(n).to_string(),
            at: at.into(),
            change: Change::Set { exercise, reps },
        }
    }
    #[test]
    fn cancellations_commute_and_over_goal_counts_remain_exact() {
        let set = set(1, "2026-10-03T23:58:00-05:00", Exercise::Squats, 120);
        let cancel = Event {
            id: Uuid::from_u128(2).to_string(),
            at: "2026-10-04T01:00:00-05:00".into(),
            change: Change::Cancel {
                target: set.id.clone(),
            },
        };
        let today = set.day();
        assert_eq!(today.to_string(), "2026-10-03");
        assert_eq!(
            progress(std::slice::from_ref(&set), today).days[&today],
            [0, 0, 120]
        );
        assert_eq!(remaining(120), 0);
        assert!(
            progress(&[cancel.clone(), set.clone(), cancel.clone()], today)
                .days
                .is_empty()
        );
        assert!(progress(&[set, cancel], today).days.is_empty());
    }
    #[test]
    fn streak_requires_every_exercise_and_survives_an_unfinished_today() {
        let mut events = Vec::new();
        for (i, day) in ["2026-09-29", "2026-09-30", "2026-10-02", "2026-10-03"]
            .iter()
            .enumerate()
        {
            for exercise in Exercise::ALL {
                events.push(set(
                    (i * 3 + exercise.index() + 1) as u128,
                    &format!("{day}T20:00:00+02:00"),
                    exercise,
                    100,
                ));
            }
        }
        events.push(set(99, "2026-10-04T20:00:00+02:00", Exercise::Squats, 100));
        let p = progress(&events, NaiveDate::from_ymd_opt(2026, 10, 4).unwrap());
        assert_eq!(
            (p.current_streak, p.best_streak, p.complete_days),
            (2, 2, 4)
        );
        assert_eq!(
            progress(&events, NaiveDate::from_ymd_opt(2026, 10, 5).unwrap()).current_streak,
            0
        );
    }
    #[test]
    fn invalid_events_never_enter_a_sync_batch() {
        let mut event = set(1, "2026-10-03T10:00:00Z", Exercise::PushUps, 0);
        assert!(event.validate().is_err());
        event.change = Change::Set {
            exercise: Exercise::PushUps,
            reps: 1001,
        };
        assert!(event.validate().is_err());
        event.change = Change::Set {
            exercise: Exercise::PushUps,
            reps: 20,
        };
        event.at = "not a time".into();
        assert!(event.validate().is_err());
    }
}
