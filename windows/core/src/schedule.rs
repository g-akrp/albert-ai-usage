use std::collections::HashMap;
#[derive(Default)]
struct Entry {
    due: i64,
    running: bool,
    failures: u32,
    refresh_requested: bool,
}
#[derive(Default)]
pub struct Schedule {
    entries: HashMap<String, Entry>,
}
impl Schedule {
    pub fn reserve(&mut self, id: &str, now: i64) -> bool {
        if self.entries.values().filter(|e| e.running).count() >= 2 {
            return false;
        }
        let e = self.entries.entry(id.into()).or_default();
        if e.running || e.due > now {
            return false;
        }
        e.running = true;
        true
    }
    pub fn finish(&mut self, id: &str, now: i64, interval: u64, ok: bool) {
        let e = self.entries.entry(id.into()).or_default();
        e.running = false;
        if ok {
            e.failures = 0;
            e.due = now + interval as i64;
        } else {
            e.failures = (e.failures + 1).min(6);
            e.due = now + (60 * (1i64 << (e.failures - 1))).min(1800);
        }
        if e.refresh_requested {
            e.due = 0;
            e.refresh_requested = false;
        }
    }
    pub fn refresh(&mut self) {
        for e in self.entries.values_mut() {
            e.due = 0;
            if e.running {
                e.refresh_requested = true;
            }
        }
    }
}
