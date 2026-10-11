use crate::{
    card_model::{Card, Tone},
    relative_time,
    settings_model::Settings,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}
pub fn position(anchor: Rect, work: Rect, width: f32, height: f32) -> Rect {
    let w = width.min(work.w).max(1.0);
    let h = height.min(work.h).max(1.0);
    let x = (anchor.x + anchor.w - w).clamp(work.x, work.x + work.w - w);
    let preferred = if anchor.y >= work.y + work.h * 0.5 {
        anchor.y - h - 8.0
    } else {
        anchor.y + anchor.h + 8.0
    };
    Rect {
        x,
        y: preferred.clamp(work.y, work.y + work.h - h),
        w,
        h,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Pin(String),
    Hide(String),
    Collapse(String),
    Move(String, isize),
    Refresh,
    Providers,
    Hidden,
    Login,
    Folder,
    Quit,
    DismissHint,
}
#[derive(Clone, Debug, Default)]
pub enum Visibility {
    #[default]
    Always,
    Hover(String),
    NotHover(String),
}
impl Visibility {
    pub fn shown(&self, hover: Option<&str>) -> bool {
        match self {
            Self::Always => true,
            Self::Hover(id) => hover == Some(id),
            Self::NotHover(id) => hover != Some(id),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Hit {
    pub rect: Rect,
    pub action: Action,
    pub visibility: Visibility,
    pub group: Option<String>,
}
#[derive(Clone, Copy, Debug)]
pub enum Ink {
    Normal,
    Muted,
    Accent,
    Error,
}
#[derive(Clone, Debug)]
pub struct Text {
    pub rect: Rect,
    pub text: String,
    pub size: f32,
    pub bold: bool,
    pub ink: Ink,
    pub visibility: Visibility,
    pub centered: bool,
}
#[derive(Clone, Debug)]
pub struct Chevron {
    pub rect: Rect,
    pub collapsed: bool,
}
#[derive(Clone, Debug)]
pub struct BoxShape {
    pub rect: Rect,
    pub radius: f32,
    pub kind: BoxKind,
    pub visibility: Visibility,
}
#[derive(Clone, Copy, Debug)]
pub enum BoxKind {
    Card,
    Button(bool),
    Separator,
}
#[derive(Clone, Debug)]
pub struct Pill {
    pub rect: Rect,
    pub text: String,
    pub tone: Tone,
    pub visibility: Visibility,
}
#[derive(Clone, Debug)]
pub struct Ring {
    pub rect: Rect,
    pub percent: Option<f64>,
    pub color: [u8; 3],
    pub shade: f32,
}
#[derive(Default, Clone, Debug)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    pub texts: Vec<Text>,
    pub boxes: Vec<BoxShape>,
    pub pills: Vec<Pill>,
    pub rings: Vec<Ring>,
    pub chevrons: Vec<Chevron>,
    pub hits: Vec<Hit>,
    pub cards: Vec<(String, Rect)>,
}
impl Layout {
    fn text(
        &mut self,
        rect: Rect,
        text: impl Into<String>,
        size: f32,
        bold: bool,
        ink: Ink,
        visibility: Visibility,
    ) {
        self.texts.push(Text {
            rect,
            text: text.into(),
            size,
            bold,
            ink,
            visibility,
            centered: false,
        });
    }
    fn button(
        &mut self,
        rect: Rect,
        text: &str,
        action: Action,
        selected: bool,
        visibility: Visibility,
        group: Option<&str>,
    ) {
        self.boxes.push(BoxShape {
            rect,
            radius: 4.0,
            kind: BoxKind::Button(selected),
            visibility: visibility.clone(),
        });
        self.text(
            Rect {
                x: rect.x + 5.0,
                y: rect.y + 3.0,
                w: rect.w - 10.0,
                h: rect.h - 4.0,
            },
            text,
            11.0,
            false,
            if selected { Ink::Accent } else { Ink::Normal },
            visibility.clone(),
        );
        self.hits.push(Hit {
            rect,
            action,
            visibility,
            group: group.map(str::to_string),
        });
    }
}
#[allow(clippy::too_many_arguments)]
pub fn layout(
    cards: &[Card],
    settings: &Settings,
    errors: &[String],
    updated: &str,
    version: &str,
    login: bool,
    hint: bool,
    max_width: f32,
    measure: impl Fn(&str, f32) -> f32,
    format_reset: impl Fn(i64) -> String,
) -> Layout {
    let mut width = 320.0f32;
    for card in cards {
        width = width.max(measure(&card.title, 13.0) + 238.0);
        for chart in &card.charts {
            for row in &chart.windows {
                width = width.max(
                    180.0
                        + measure(row.label.as_deref().unwrap_or("Usage"), 12.0)
                        + measure(&row.percent_text(true), 12.0)
                        + if chart.row_pins.iter().any(Option::is_some) {
                            62.0
                        } else {
                            10.0
                        },
                );
                if let Some(reset) = row.resets_at {
                    width = width.max(
                        140.0
                            + measure(
                                &format!(
                                    "{} · {}",
                                    format_reset(reset),
                                    relative_time::time_left(reset, relative_time::now())
                                ),
                                11.0,
                            ),
                    );
                }
            }
        }
    }
    width = width.min(max_width.max(300.0));
    let mut out = Layout {
        width,
        ..Default::default()
    };
    let mut y = 12.0;
    for (index, card) in cards.iter().enumerate() {
        let collapsed = settings.collapsed.contains(&card.id);
        let top = y;
        let pad = if collapsed { 10.0 } else { 16.0 };
        let mut cy = y + pad;
        let visible = Visibility::Hover(card.id.clone());
        let hidden = Visibility::NotHover(card.id.clone());
        let group = Some(card.id.as_str());
        let collapse_rect = Rect {
            x: 24.0,
            y: cy,
            w: 22.0,
            h: 24.0,
        };
        out.chevrons.push(Chevron {
            rect: collapse_rect,
            collapsed,
        });
        out.hits.push(Hit {
            rect: collapse_rect,
            action: Action::Collapse(card.id.clone()),
            visibility: Visibility::Always,
            group: Some(card.id.clone()),
        });
        let title_rect = Rect {
            x: 48.0,
            y: cy + 3.0,
            w: (width - 228.0).max(60.0),
            h: 22.0,
        };
        let pinned = settings
            .pin
            .as_ref()
            .is_some_and(|p| p.split('|').next() == Some(card.id.as_str()));
        out.text(
            title_rect,
            &card.title,
            13.0,
            pinned,
            if card.error { Ink::Error } else { Ink::Normal },
            Visibility::Always,
        );
        out.hits.push(Hit {
            rect: title_rect,
            action: Action::Pin(card.title_pin.clone()),
            visibility: Visibility::Always,
            group: Some(card.id.clone()),
        });
        let selected = settings.pin.as_deref() == Some(card.title_pin.as_str());
        let pin = Rect {
            x: width - 80.0,
            y: cy,
            w: 52.0,
            h: 24.0,
        };
        out.button(
            pin,
            if selected { "Pinned" } else { "Pin" },
            Action::Pin(card.title_pin.clone()),
            selected,
            if selected {
                Visibility::Always
            } else {
                visible.clone()
            },
            group,
        );
        out.button(
            Rect {
                x: width - 127.0,
                y: cy,
                w: 40.0,
                h: 24.0,
            },
            "Hide",
            Action::Hide(card.id.clone()),
            false,
            visible.clone(),
            group,
        );
        let mut ax = width - 157.0;
        if index + 1 < cards.len() {
            out.button(
                Rect {
                    x: ax,
                    y: cy,
                    w: 24.0,
                    h: 24.0,
                },
                "↓",
                Action::Move(card.id.clone(), 1),
                false,
                visible.clone(),
                group,
            );
            ax -= 30.0;
        }
        if index > 0 {
            out.button(
                Rect {
                    x: ax,
                    y: cy,
                    w: 24.0,
                    h: 24.0,
                },
                "↑",
                Action::Move(card.id.clone(), -1),
                false,
                visible.clone(),
                group,
            );
        }
        if collapsed {
            let mut x = if selected { width - 86.0 } else { width - 28.0 };
            for (text, tone) in card.pills().into_iter().rev() {
                let w = measure(&text, 12.0) + 14.0;
                x -= w;
                out.pills.push(Pill {
                    rect: Rect {
                        x,
                        y: cy + 2.0,
                        w,
                        h: 20.0,
                    },
                    text,
                    tone,
                    visibility: hidden.clone(),
                });
                x -= 6.0;
            }
        }
        cy += 26.0;
        if let Some(account) = &card.account {
            out.text(
                Rect {
                    x: 48.0,
                    y: cy,
                    w: width - 76.0,
                    h: 16.0,
                },
                account,
                11.0,
                false,
                Ink::Muted,
                Visibility::Always,
            );
            cy += 19.0;
        }
        if let Some(message) = &card.message {
            out.text(
                Rect {
                    x: 48.0,
                    y: cy + 5.0,
                    w: width - 76.0,
                    h: 32.0,
                },
                message,
                12.0,
                false,
                if card.error { Ink::Error } else { Ink::Muted },
                Visibility::Always,
            );
            cy += 37.0;
        }
        for notice in &card.notices {
            out.text(
                Rect {
                    x: 48.0,
                    y: cy + 4.0,
                    w: width - 76.0,
                    h: 20.0,
                },
                notice,
                11.0,
                false,
                if notice == "Usage limit reached" {
                    Ink::Error
                } else {
                    Ink::Muted
                },
                Visibility::Always,
            );
            cy += 24.0;
        }
        if !collapsed {
            for chart in &card.charts {
                cy += 10.0;
                let rows_height: f32 = chart
                    .windows
                    .iter()
                    .map(|w| if w.resets_at.is_some() { 35.0 } else { 22.0 })
                    .sum();
                let h = (rows_height + if chart.title.is_some() { 24.0 } else { 0.0 }).max(76.0);
                let ring_top = cy + (h - 76.0) / 2.0;
                for (i, w) in chart.windows.iter().take(4).enumerate() {
                    let inset = i as f32 * 8.0;
                    out.rings.push(Ring {
                        rect: Rect {
                            x: 32.0 + inset,
                            y: ring_top + inset,
                            w: 76.0 - 2.0 * inset,
                            h: 76.0 - 2.0 * inset,
                        },
                        percent: w.used,
                        color: card.color,
                        shade: i as f32 * 0.3,
                    });
                }
                out.text(
                    Rect {
                        x: 32.0,
                        y: ring_top,
                        w: 76.0,
                        h: 76.0,
                    },
                    chart
                        .center
                        .map(|n| format!("{n:.0}%"))
                        .unwrap_or_else(|| "--".into()),
                    12.0,
                    true,
                    Ink::Normal,
                    Visibility::Always,
                );
                out.texts.last_mut().unwrap().centered = true;
                let mut ry = cy;
                if let Some(title) = &chart.title {
                    out.text(
                        Rect {
                            x: 124.0,
                            y: ry + 3.0,
                            w: width - 210.0,
                            h: 20.0,
                        },
                        title,
                        12.0,
                        true,
                        Ink::Normal,
                        Visibility::Always,
                    );
                    if let Some(pin) = &chart.pin {
                        let selected = settings.pin.as_deref() == Some(pin.as_str());
                        out.button(
                            Rect {
                                x: width - 80.0,
                                y: ry,
                                w: 52.0,
                                h: 24.0,
                            },
                            if selected { "Pinned" } else { "Pin" },
                            Action::Pin(pin.clone()),
                            selected,
                            if selected {
                                Visibility::Always
                            } else {
                                visible.clone()
                            },
                            group,
                        );
                    }
                    ry += 26.0;
                }
                for (i, row) in chart.windows.iter().enumerate() {
                    let text = row.percent_text(true);
                    let pill_w = measure(&text, 12.0) + 14.0;
                    let pin = chart.row_pins.get(i).and_then(Option::as_ref);
                    let right = if pin.is_some() {
                        width - 86.0
                    } else {
                        width - 28.0
                    };
                    let pill = Rect {
                        x: right - pill_w,
                        y: ry,
                        w: pill_w,
                        h: 20.0,
                    };
                    out.text(
                        Rect {
                            x: 124.0,
                            y: ry + 2.0,
                            w: (pill.x - 132.0).max(30.0),
                            h: 20.0,
                        },
                        row.label.as_deref().unwrap_or("Usage"),
                        12.0,
                        false,
                        Ink::Normal,
                        Visibility::Always,
                    );
                    out.pills.push(Pill {
                        rect: pill,
                        text,
                        tone: Tone::of(row.used),
                        visibility: Visibility::Always,
                    });
                    if let Some(pin) = pin {
                        let selected = settings.pin.as_deref() == Some(pin.as_str());
                        out.button(
                            Rect {
                                x: width - 80.0,
                                y: ry,
                                w: 52.0,
                                h: 24.0,
                            },
                            if selected { "Pinned" } else { "Pin" },
                            Action::Pin(pin.clone()),
                            selected,
                            if selected {
                                Visibility::Always
                            } else {
                                visible.clone()
                            },
                            group,
                        );
                    }
                    if let Some(reset) = row.resets_at {
                        out.text(
                            Rect {
                                x: 124.0,
                                y: ry + 21.0,
                                w: width - 152.0,
                                h: 16.0,
                            },
                            format!(
                                "{} · {}",
                                format_reset(reset),
                                relative_time::time_left(reset, relative_time::now())
                            ),
                            11.0,
                            false,
                            Ink::Muted,
                            Visibility::Always,
                        );
                        ry += 35.0;
                    } else {
                        ry += 22.0;
                    }
                }
                cy += h;
            }
        }
        y = cy + pad;
        let rect = Rect {
            x: 12.0,
            y: top,
            w: width - 24.0,
            h: y - top,
        };
        out.boxes.push(BoxShape {
            rect,
            radius: 8.0,
            kind: BoxKind::Card,
            visibility: Visibility::Always,
        });
        out.cards.push((card.id.clone(), rect));
        y += 8.0;
    }
    if cards.is_empty() {
        out.text(
            Rect {
                x: 24.0,
                y,
                w: width - 48.0,
                h: 26.0,
            },
            if settings.disabled.is_empty() {
                "All cards are hidden"
            } else {
                "All providers are off"
            },
            12.0,
            false,
            Ink::Muted,
            Visibility::Always,
        );
        y += 35.0;
    }
    for error in errors {
        out.text(
            Rect {
                x: 24.0,
                y,
                w: width - 48.0,
                h: 32.0,
            },
            error,
            11.0,
            false,
            Ink::Error,
            Visibility::Always,
        );
        y += 35.0;
    }
    out.boxes.push(BoxShape {
        rect: Rect {
            x: 12.0,
            y,
            w: width - 24.0,
            h: 1.0,
        },
        radius: 0.0,
        kind: BoxKind::Separator,
        visibility: Visibility::Always,
    });
    y += 8.0;
    let menu = |text: &str, action: Action, arrow: bool, out: &mut Layout, y: &mut f32| {
        let r = Rect {
            x: 12.0,
            y: *y,
            w: width - 24.0,
            h: 29.0,
        };
        out.text(
            Rect {
                x: 28.0,
                y: *y + 5.0,
                w: width - 56.0,
                h: 22.0,
            },
            text,
            12.0,
            false,
            Ink::Normal,
            Visibility::Always,
        );
        if arrow {
            out.text(
                Rect {
                    x: width - 40.0,
                    y: *y + 4.0,
                    w: 20.0,
                    h: 22.0,
                },
                "›",
                16.0,
                false,
                Ink::Muted,
                Visibility::Always,
            );
        }
        out.hits.push(Hit {
            rect: r,
            action,
            visibility: Visibility::Always,
            group: None,
        });
        *y += 29.0;
    };
    menu("Refresh Now", Action::Refresh, false, &mut out, &mut y);
    out.text(
        Rect {
            x: width - 77.0,
            y: y - 24.0,
            w: 52.0,
            h: 20.0,
        },
        "Ctrl+R",
        11.0,
        false,
        Ink::Muted,
        Visibility::Always,
    );
    out.text(
        Rect {
            x: 28.0,
            y,
            w: width - 56.0,
            h: 18.0,
        },
        format!("Updated {updated}"),
        11.0,
        false,
        Ink::Muted,
        Visibility::Always,
    );
    y += 20.0;
    menu("Providers", Action::Providers, true, &mut out, &mut y);
    if !settings.hidden.is_empty() {
        menu("Hidden Cards", Action::Hidden, true, &mut out, &mut y);
    }
    menu(
        if login {
            "✓ Launch at Login"
        } else {
            "Launch at Login"
        },
        Action::Login,
        false,
        &mut out,
        &mut y,
    );
    menu(
        "Open Providers Folder…",
        Action::Folder,
        false,
        &mut out,
        &mut y,
    );
    for text in [
        format!("AI Usage {version}"),
        "Built with ♥ by g.akrp".into(),
    ] {
        out.text(
            Rect {
                x: 28.0,
                y,
                w: width - 56.0,
                h: 18.0,
            },
            text,
            11.0,
            false,
            Ink::Muted,
            Visibility::Always,
        );
        y += 20.0;
    }
    if hint {
        menu(
            "Keep icon visible in Taskbar settings · Dismiss",
            Action::DismissHint,
            false,
            &mut out,
            &mut y,
        );
    }
    menu("Quit AI Usage", Action::Quit, false, &mut out, &mut y);
    out.height = y + 8.0;
    out
}
