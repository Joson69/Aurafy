use std::time::{Duration, Instant};

/// Keep the thumb local while a gesture is in progress and send only the newest
/// value. One outstanding command prevents a slow connection building a backlog.
#[derive(Default)]
pub struct Control {
    pub interacting: bool,
    pending: Option<u8>,
    in_flight: bool,
    force: bool,
    last_sent: Option<(u8, Instant)>,
}

impl Control {
    pub fn queue(&mut self, value: u8, finished: bool, now: Instant) {
        let value = value.min(100);
        self.pending = if self.last_sent.is_some_and(|(sent, at)| {
            sent == value && now.duration_since(at) < Duration::from_secs(2)
        }) {
            None
        } else {
            Some(value)
        };
        self.force = self.pending.is_some() && (self.force || finished);
    }

    pub fn next(&mut self, now: Instant, interval: Duration) -> Option<u8> {
        if self.in_flight || self.pending.is_none() {
            return None;
        }
        if !self.force
            && self
                .last_sent
                .is_some_and(|(_, at)| now.duration_since(at) < interval)
        {
            return None;
        }
        let value = self.pending.take()?;
        self.in_flight = true;
        self.force = false;
        self.last_sent = Some((value, now));
        Some(value)
    }

    pub fn complete(&mut self, succeeded: bool) {
        self.in_flight = false;
        if !succeeded {
            self.last_sent = None;
        }
    }

    pub fn waiting(&self) -> bool {
        self.pending.is_some() && !self.in_flight
    }

    pub fn accepts_snapshot(&self, value: u8, now: Instant) -> bool {
        if self.interacting || self.pending.is_some() || self.in_flight {
            return false;
        }
        // State notifications can arrive after the command acknowledgement.
        // Protect the final thumb position briefly, then allow external controls.
        self.last_sent.is_none_or(|(sent, at)| {
            value == sent || now.duration_since(at) >= Duration::from_secs(2)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_sends_live_coalesces_moves_and_flushes_final_value() {
        let now = Instant::now();
        let interval = Duration::from_millis(50);
        let mut control = Control::default();
        control.interacting = true;
        control.queue(20, false, now);
        assert_eq!(control.next(now, interval), Some(20));
        for value in [30, 40, 55, 70] {
            control.queue(value, false, now);
            assert_eq!(control.next(now, interval), None);
        }
        control.complete(true);
        assert_eq!(control.next(now + interval / 2, interval), None);
        assert_eq!(control.next(now + interval, interval), Some(70));
        control.queue(83, true, now + interval);
        control.interacting = false;
        assert_eq!(control.next(now + interval, interval), None);
        control.complete(true);
        assert_eq!(control.next(now + interval, interval), Some(83));
        control.complete(true);
        assert_eq!(control.next(now + interval * 2, interval), None);
    }

    #[test]
    fn stale_echoes_do_not_move_thumb_but_external_changes_eventually_apply() {
        let now = Instant::now();
        let mut control = Control::default();
        control.interacting = true;
        control.queue(73, false, now);
        assert!(!control.accepts_snapshot(20, now));
        control.next(now, Duration::ZERO);
        control.complete(true);
        assert!(!control.accepts_snapshot(73, now));
        control.interacting = false;
        assert!(control.accepts_snapshot(73, now));
        assert!(!control.accepts_snapshot(20, now + Duration::from_millis(500)));
        assert!(control.accepts_snapshot(20, now + Duration::from_secs(2)));
    }

    #[test]
    fn click_is_immediate_and_failed_commands_release_the_queue() {
        let now = Instant::now();
        let mut control = Control::default();
        control.queue(35, true, now);
        assert_eq!(control.next(now, Duration::from_millis(150)), Some(35));
        control.queue(60, true, now);
        control.complete(false);
        assert_eq!(control.next(now, Duration::from_millis(150)), Some(60));
        control.complete(false);
        assert!(control.accepts_snapshot(10, now));
    }
}
