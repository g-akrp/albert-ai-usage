use crate::{
    config::ProviderConfig,
    report::{Report, Window},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    Secondary,
    Green,
    Orange,
    Red,
}
impl Tone {
    pub fn of(percent: Option<f64>) -> Self {
        match percent {
            Some(n) if n >= 90.0 => Self::Red,
            Some(n) if n >= 70.0 => Self::Orange,
            Some(_) => Self::Green,
            None => Self::Secondary,
        }
    }
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Green => [52, 199, 89],
            Self::Orange => [255, 149, 0],
            Self::Red => [255, 59, 48],
            Self::Secondary => [140, 140, 140],
        }
    }
}
#[derive(Clone, Debug)]
pub struct Run {
    pub id: String,
    pub config_id: String,
    pub name: String,
    pub label: String,
    pub color: [u8; 3],
    pub account: Option<String>,
    pub result: Option<Result<Report, String>>,
}
impl Run {
    pub fn loading(config: &ProviderConfig) -> Self {
        Self {
            id: config.id.clone(),
            config_id: config.id.clone(),
            name: config.name.clone(),
            label: config.label.clone(),
            color: config.color,
            account: None,
            result: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Chart {
    pub title: Option<String>,
    pub pin: Option<String>,
    pub center: Option<f64>,
    pub windows: Vec<Window>,
    pub row_pins: Vec<Option<String>>,
}
#[derive(Clone, Debug)]
pub struct Card {
    pub id: String,
    pub title: String,
    pub account: Option<String>,
    pub color: [u8; 3],
    pub title_pin: String,
    pub message: Option<String>,
    pub error: bool,
    pub notices: Vec<String>,
    pub charts: Vec<Chart>,
}
impl Card {
    pub fn from_run(run: &Run) -> Self {
        let mut card = Self {
            id: run.id.clone(),
            title: run.name.clone(),
            account: run.account.clone(),
            color: run.color,
            title_pin: run.id.clone(),
            message: None,
            error: false,
            notices: Vec::new(),
            charts: Vec::new(),
        };
        let report = match &run.result {
            None => {
                card.message = Some("Loading…".into());
                return card;
            }
            Some(Err(e)) => {
                card.message = Some(e.clone());
                card.error = true;
                return card;
            }
            Some(Ok(r)) => r,
        };
        if let Some(plan) = &report.plan {
            card.title.push_str(&format!(" ({plan})"));
        }
        if report.available == Some(false) && report.meters.is_empty() {
            card.message = Some("Usage unavailable".into());
            return card;
        }
        if report.available == Some(false) {
            card.notices
                .push("Plan limits don't apply to this account".into());
        }
        if report.access == Some(false) {
            card.notices.push("Usage limit reached".into());
        }
        if report.meters.is_empty() {
            card.notices.push("No usage data".into());
        }
        if report.grouped() {
            if let Some(first) = report.meters.first() {
                card.title_pin = format!("{}|{}", run.id, first.id);
            }
            for meter in &report.meters {
                let mut windows = meter.windows.clone();
                windows.sort_by_key(Window::priority);
                let only = Report {
                    plan: None,
                    available: None,
                    access: None,
                    meters: vec![meter.clone()],
                };
                card.charts.push(Chart {
                    title: Some(meter.label.clone()),
                    pin: Some(format!("{}|{}", run.id, meter.id)),
                    center: only.headline(),
                    row_pins: vec![None; windows.len()],
                    windows,
                });
            }
        } else if !report.meters.is_empty() {
            let mut pairs: Vec<_> = report
                .meters
                .iter()
                .flat_map(|m| {
                    m.windows.iter().map(move |w| {
                        let mut w = w.clone();
                        if w.label.is_none() {
                            w.label = Some(m.label.clone());
                        }
                        let pin = (report.meters.len() > 1 || run.config_id == "copilot")
                            .then(|| format!("{}|{}", run.id, m.id));
                        (w, pin, m.id.contains("premium"))
                    })
                })
                .collect();
            pairs.sort_by_key(|(w, _, premium)| if *premium { 0 } else { w.priority() + 1 });
            let (windows, row_pins): (Vec<_>, Vec<_>) =
                pairs.into_iter().map(|(w, p, _)| (w, p)).unzip();
            card.charts.push(Chart {
                title: None,
                pin: None,
                center: report.headline(),
                windows,
                row_pins,
            });
        }
        card
    }
    pub fn pills(&self) -> Vec<(String, Tone)> {
        if self.charts.len() == 1 {
            self.charts[0]
                .windows
                .iter()
                .take(4)
                .map(|w| (w.percent_text(false), Tone::of(w.used)))
                .collect()
        } else {
            self.charts
                .iter()
                .take(4)
                .map(|c| {
                    (
                        c.center
                            .map(|n| format!("{n:.0}%"))
                            .unwrap_or_else(|| "--".into()),
                        c.windows
                            .iter()
                            .map(|w| Tone::of(w.used))
                            .max()
                            .unwrap_or(Tone::Secondary),
                    )
                })
                .collect()
        }
    }
}
pub fn ensure_pin(settings: &mut crate::settings_model::Settings, runs: &[Run]) {
    if let Some(pin) = settings.pin.clone() {
        let id = pin.split('|').next().unwrap_or("");
        let run = runs
            .iter()
            .find(|r| r.id == id)
            .or_else(|| runs.iter().find(|r| r.config_id == id));
        if let Some(run) = run {
            if !pin.contains('|') {
                settings.pin = Some(Card::from_run(run).title_pin);
            }
        }
    }
    let valid = settings.pin.as_ref().is_some_and(|pin| {
        let mut parts = pin.splitn(2, '|');
        let id = parts.next().unwrap_or("");
        let meter = parts.next();
        runs.iter().any(|r| {
            r.id == id
                && meter.is_none_or(|m| {
                    r.result.as_ref().is_none_or(|v| {
                        v.as_ref().is_err()
                            || v.as_ref()
                                .is_ok_and(|report| report.meters.iter().any(|mtr| mtr.id == m))
                    })
                })
        })
    });
    if !valid {
        settings.pin = runs.first().map(|r| Card::from_run(r).title_pin);
    }
}
