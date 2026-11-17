use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartPage {
    #[default]
    LastView,
    Home,
    Search,
    Library,
}

pub enum SleepTimer {
    Deadline(Instant),
    EndOfTrack {
        uri: String,
        last_position: u64,
        duration: u64,
    },
}

impl SleepTimer {
    pub fn minutes(minutes: u32) -> Self {
        Self::Deadline(Instant::now() + Duration::from_secs(u64::from(minutes) * 60))
    }

    pub fn expired(&mut self, now: Instant, uri: &str, position: u64) -> bool {
        match self {
            Self::Deadline(deadline) => now >= *deadline,
            Self::EndOfTrack {
                uri: target,
                last_position,
                duration,
            } => {
                let ended = !uri.is_empty()
                    && (uri != target
                        || (*duration > 1 && position >= *duration)
                        || (position.saturating_add(2500) < *last_position
                            && last_position.saturating_add(2500) >= *duration));
                *last_position = position;
                ended
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Deadline(deadline) => {
                let seconds = deadline.saturating_duration_since(Instant::now()).as_secs();
                format!("{}:{:02} remaining", seconds / 60, seconds % 60)
            }
            Self::EndOfTrack { .. } => "End of this song".into(),
        }
    }
    pub fn seeked(&mut self, position: u64) {
        if let Self::EndOfTrack { last_position, .. } = self {
            *last_position = position;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timers_expire_at_the_deadline_or_track_end_and_respect_seeking() {
        let now = Instant::now();
        let mut timer = SleepTimer::Deadline(now + Duration::from_secs(60));
        assert!(!timer.expired(now, "song", 1234));
        assert!(timer.expired(now + Duration::from_secs(60), "song", 1234));
        let mut timer = SleepTimer::EndOfTrack {
            uri: "song".into(),
            last_position: 178_000,
            duration: 180_000,
        };
        assert!(!timer.expired(now, "song", 179_000));
        timer.seeked(40_000);
        assert!(!timer.expired(now, "song", 40_000));
        assert!(timer.expired(now, "song", 180_000));
        assert!(timer.expired(now, "next-song", 0));
        let mut repeated = SleepTimer::EndOfTrack {
            uri: "song".into(),
            last_position: 179_000,
            duration: 180_000,
        };
        assert!(repeated.expired(now, "song", 0));
    }
}
