use crate::keypad::{BUTTON_NAMES, KEY_COUNT};
use std::collections::VecDeque;
use std::time::Instant;

const LOG_LINES: usize = 12;

pub struct Diagnostics {
    start: Instant,
    keys: [bool; KEY_COUNT],
    log: VecDeque<String>,
    digital_events: u64,
    mark_count: u64,
}

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics {
            start: Instant::now(),
            keys: [false; KEY_COUNT],
            log: VecDeque::with_capacity(LOG_LINES + 1),
            digital_events: 0,
            mark_count: 0,
        }
    }

    pub fn keys(&self) -> [bool; KEY_COUNT] {
        self.keys
    }

    pub fn digital_events(&self) -> u64 {
        self.digital_events
    }

    pub fn log_lines(&self) -> impl Iterator<Item = &String> {
        self.log.iter()
    }

    pub fn on_digital(&mut self, name: &str, down: bool) {
        self.digital_events += 1;
        if let Some(i) = button_index(name) {
            self.keys[i] = down;
        }
        let line = format!(
            "{}  {name:<14} {}",
            crate::stamp(self.start.elapsed()),
            if down { "DOWN" } else { "up" }
        );
        log::info!("{line}");
        self.push(line);
    }

    pub fn mark_stall(&mut self, detail: &str) -> String {
        self.mark_count += 1;
        let n = self.mark_count;
        let stamp = self.stamp();
        let banner = format!("{stamp}  ★★★ STALL MARK #{n} ★★★");
        log::error!("========== STALL MARK #{n} @ {stamp} ==========");
        log::error!("{detail}");
        log::error!("========== END STALL MARK #{n} ==========");
        self.push(banner.clone());
        banner
    }

    pub fn stamp(&self) -> String {
        crate::stamp(self.start.elapsed())
    }

    fn push(&mut self, line: String) {
        if self.log.len() == LOG_LINES {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }
}

fn button_index(name: &str) -> Option<usize> {
    BUTTON_NAMES.iter().position(|n| *n == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypad::{POP_L, POP_R};

    #[test]
    fn button_names_map_to_lamps_and_auxiliaries_do_not() {
        assert_eq!(button_index("Button 1"), Some(0));
        assert_eq!(button_index("Button 9"), Some(8));
        assert_eq!(button_index("Red Pop-Kun"), Some(POP_L));
        assert_eq!(button_index("Blue Pop-Kun"), Some(POP_R));
        assert_eq!(button_index("Button 10"), None);
        assert_eq!(button_index("Button 0"), None);
        assert_eq!(button_index("Service"), None);
    }

    #[test]
    fn digital_events_track_held_keys() {
        let mut st = Diagnostics::new();
        st.on_digital("Button 3", true);
        st.on_digital("Service", true);
        assert!(st.keys()[2]);
        assert_eq!(
            st.keys().iter().filter(|k| **k).count(),
            1,
            "Service is not a lamp"
        );
        st.on_digital("Button 3", false);
        assert!(!st.keys()[2]);
        assert_eq!(st.digital_events(), 3);
    }

    #[test]
    fn the_pop_kuns_light_their_own_lamps() {
        let mut st = Diagnostics::new();
        st.on_digital("Blue Pop-Kun", true);
        assert!(st.keys()[POP_R]);
        assert!(!st.keys()[POP_L]);
    }

    #[test]
    fn the_log_is_bounded() {
        let mut st = Diagnostics::new();
        for _ in 0..(LOG_LINES * 3) {
            st.on_digital("Button 1", true);
        }
        assert_eq!(st.log_lines().count(), LOG_LINES);
    }

    #[test]
    fn mark_stall_shows_up_in_the_on_screen_log() {
        let mut st = Diagnostics::new();
        let banner = st.mark_stall("spiceapi snapshot test");
        assert!(banner.contains("STALL MARK #1"));
        assert_eq!(st.log_lines().count(), 1);
        assert!(st.log_lines().next().unwrap().contains("STALL MARK #1"));
    }
}
