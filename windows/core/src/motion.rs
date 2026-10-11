//! Flyout motion: easing and the animated values the painter reads. Times are milliseconds.
use std::collections::HashMap;

pub const OPEN_MS: f64 = 180.0;
pub const RING_MS: f64 = 300.0;
pub const UPDATED_MS: f64 = 1000.0;
/// How far the flyout slides in from the taskbar side, in DIPs.
pub const SLIDE_DIP: f32 = 10.0;

pub fn ease_out_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[derive(Clone, Copy, Debug)]
struct Tween {
    from: f64,
    to: f64,
    start: f64,
    duration: f64,
}
impl Tween {
    fn value(&self, now: f64) -> f64 {
        if self.duration <= 0.0 {
            return self.to;
        }
        let t = ease_out_cubic((now - self.start) / self.duration);
        self.from + (self.to - self.from) * t
    }
    fn done(&self, now: f64) -> bool {
        now >= self.start + self.duration
    }
}

/// With `reduced` set (Windows "Animation effects" off) nothing animates.
#[derive(Debug, Default)]
pub struct Motion {
    reduced: bool,
    open: Option<Tween>,
    rings: HashMap<String, Tween>,
    updated: Option<Tween>,
}
impl Motion {
    pub fn new(reduced: bool) -> Self {
        Self {
            reduced,
            ..Default::default()
        }
    }
    pub fn set_reduced(&mut self, reduced: bool) {
        self.reduced = reduced;
    }
    pub fn open(&mut self, now: f64) {
        self.open = (!self.reduced).then_some(Tween {
            from: 0.0,
            to: 1.0,
            start: now,
            duration: OPEN_MS,
        });
    }
    /// 0 when the flyout starts opening, 1 when it is in place.
    pub fn open_progress(&self, now: f64) -> f64 {
        self.open.map_or(1.0, |t| t.value(now))
    }
    /// Starts a sweep for each ring whose target changed since it was last shown. A ring seen
    /// for the first time sweeps from 0; a ring that is mid-sweep continues from where it is.
    pub fn rings(&mut self, targets: &[(String, Option<f64>)], now: f64) {
        for (key, target) in targets {
            let Some(to) = *target else {
                self.rings.remove(key);
                continue;
            };
            let from = self.rings.get(key).map_or(0.0, |t| t.value(now));
            if (from - to).abs() < 0.05 && self.rings.contains_key(key) {
                continue;
            }
            let duration = if self.reduced { 0.0 } else { RING_MS };
            self.rings.insert(
                key.clone(),
                Tween {
                    from,
                    to,
                    start: now,
                    duration,
                },
            );
        }
    }
    /// The percent to draw for a ring: its sweep value, else the layout's own value.
    pub fn ring_value(&self, key: &str, now: f64, target: Option<f64>) -> Option<f64> {
        target?;
        match self.rings.get(key) {
            Some(t) if t.duration > 0.0 => Some(t.value(now)),
            _ => target,
        }
    }
    pub fn flash_updated(&mut self, now: f64) {
        self.updated = (!self.reduced).then_some(Tween {
            from: 1.0,
            to: 0.0,
            start: now,
            duration: UPDATED_MS,
        });
    }
    /// 1 right after new results arrive, fading to 0.
    pub fn updated_glow(&self, now: f64) -> f64 {
        self.updated
            .filter(|t| !t.done(now))
            .map_or(0.0, |t| t.value(now))
    }
    /// True while any value is still changing, so the animation timer must keep running.
    pub fn active(&self, now: f64) -> bool {
        self.open.is_some_and(|t| !t.done(now))
            || self.updated.is_some_and(|t| !t.done(now))
            || self
                .rings
                .values()
                .any(|t| t.duration > 0.0 && !t.done(now))
    }
}
