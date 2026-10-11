use ai_usage_core::{
    card_model::Tone,
    panel_layout::{BoxKind, Ink, Layout, Rect},
};
use windows::{
    core::w,
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct2D::{Common::*, *},
            DirectWrite::*,
        },
    },
};

fn color(rgb: [u8; 3], alpha: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: rgb[0] as f32 / 255.0,
        g: rgb[1] as f32 / 255.0,
        b: rgb[2] as f32 / 255.0,
        a: alpha,
    }
}
fn rect(r: Rect, scroll: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left: r.x,
        top: r.y - scroll,
        right: r.x + r.w,
        bottom: r.y + r.h - scroll,
    }
}
pub struct Painter {
    factory: ID2D1Factory,
    write: IDWriteFactory,
    formats: Vec<(f32, bool, IDWriteTextFormat)>,
    pub target: Option<ID2D1HwndRenderTarget>,
}
impl Painter {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let mut formats = Vec::new();
            for (size, bold) in [
                (11.0, false),
                (12.0, false),
                (12.0, true),
                (13.0, false),
                (13.0, true),
                (16.0, false),
                (18.0, false),
            ] {
                let format = write.CreateTextFormat(
                    w!("Segoe UI"),
                    None,
                    if bold {
                        DWRITE_FONT_WEIGHT_SEMI_BOLD
                    } else {
                        DWRITE_FONT_WEIGHT_NORMAL
                    },
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    size,
                    w!("en-US"),
                )?;
                format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
                formats.push((size, bold, format));
            }
            Ok(Self {
                factory,
                write,
                formats,
                target: None,
            })
        }
    }
    fn font(&self, size: f32, bold: bool) -> &IDWriteTextFormat {
        &self
            .formats
            .iter()
            .find(|f| f.0 == size && f.1 == bold)
            .unwrap_or(&self.formats[1])
            .2
    }
    pub fn measure(&self, text: &str, size: f32) -> f32 {
        unsafe {
            let text: Vec<u16> = text.encode_utf16().collect();
            let Ok(layout) =
                self.write
                    .CreateTextLayout(&text, self.font(size, false), 10000.0, 100.0)
            else {
                return text.len() as f32 * size * 0.6;
            };
            let mut metrics = DWRITE_TEXT_METRICS::default();
            if layout.GetMetrics(&mut metrics).is_err() {
                return text.len() as f32 * size * 0.6;
            }
            metrics.widthIncludingTrailingWhitespace
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(target) = &self.target {
            unsafe {
                if target.Resize(&D2D_SIZE_U { width, height }).is_err() {
                    self.target = None;
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn paint(
        &mut self,
        hwnd: HWND,
        pixels: (u32, u32),
        scale: f32,
        layout: &Layout,
        scroll: f32,
        hover: Option<&str>,
        focused: Option<usize>,
        hovered: Option<usize>,
        dark: bool,
    ) -> windows::core::Result<()> {
        unsafe {
            if self.target.is_none() {
                self.target = Some(self.factory.CreateHwndRenderTarget(
                    &D2D1_RENDER_TARGET_PROPERTIES {
                        dpiX: 96.0 * scale,
                        dpiY: 96.0 * scale,
                        ..Default::default()
                    },
                    &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                        hwnd,
                        pixelSize: D2D_SIZE_U {
                            width: pixels.0,
                            height: pixels.1,
                        },
                        presentOptions: D2D1_PRESENT_OPTIONS_NONE,
                    },
                )?);
            }
            let target = self.target.as_ref().unwrap();
            let brush = target.CreateSolidColorBrush(&color([128, 128, 128], 1.0), None)?;
            let fg = if dark { [240, 240, 240] } else { [32, 32, 32] };
            let muted = if dark {
                [180, 180, 180]
            } else {
                [103, 103, 103]
            };
            let border = if dark { [66, 66, 68] } else { [222, 222, 224] };
            let panel = if dark { [36, 36, 36] } else { [250, 250, 250] };
            let card = if dark { [42, 42, 42] } else { [255, 255, 255] };
            let accent = if dark { [117, 186, 255] } else { [0, 103, 192] };
            let viewport = pixels.1 as f32 / scale;
            target.BeginDraw();
            target.Clear(Some(&color(panel, 1.0)));
            for kind in [0, 1] {
                for shape in &layout.boxes {
                    if !shape.visibility.shown(hover)
                        || shape.rect.y + shape.rect.h < scroll
                        || shape.rect.y > scroll + viewport
                    {
                        continue;
                    }
                    let is_card = matches!(shape.kind, BoxKind::Card);
                    if (kind == 0) != is_card {
                        continue;
                    }
                    let rr = D2D1_ROUNDED_RECT {
                        rect: rect(shape.rect, scroll),
                        radiusX: shape.radius,
                        radiusY: shape.radius,
                    };
                    match shape.kind {
                        BoxKind::Card => {
                            brush.SetColor(&color(card, 1.0));
                            target.FillRoundedRectangle(&rr, &brush);
                            brush.SetColor(&color(border, 1.0));
                            target.DrawRoundedRectangle(&rr, &brush, 1.0, None);
                        }
                        BoxKind::Button(selected) => {
                            brush.SetColor(&color(if selected { accent } else { border }, 1.0));
                            target.DrawRoundedRectangle(&rr, &brush, 1.0, None);
                        }
                        BoxKind::Separator => {
                            brush.SetColor(&color(border, 1.0));
                            target.FillRectangle(&rr.rect, &brush);
                        }
                    }
                }
            }
            if let Some(hit) = hovered.and_then(|i| layout.hits.get(i)) {
                if hit.visibility.shown(hover) {
                    brush.SetColor(&color(accent, if dark { 0.20 } else { 0.12 }));
                    target.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: rect(hit.rect, scroll),
                            radiusX: 5.0,
                            radiusY: 5.0,
                        },
                        &brush,
                    );
                }
            }
            for chevron in &layout.chevrons {
                let cx = chevron.rect.x + chevron.rect.w / 2.0;
                let cy = chevron.rect.y + chevron.rect.h / 2.0 - scroll;
                let offsets = if chevron.collapsed {
                    [(-2.0, -4.0), (2.0, 0.0), (-2.0, 4.0)]
                } else {
                    [(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]
                };
                let points = offsets.map(|(x, y)| {
                    let mut p = D2D1_ELLIPSE::default().point;
                    p.X = cx + x;
                    p.Y = cy + y;
                    p
                });
                brush.SetColor(&color(muted, 1.0));
                target.DrawLine(points[0], points[1], &brush, 1.5, None);
                target.DrawLine(points[1], points[2], &brush, 1.5, None);
            }
            for ring in &layout.rings {
                if ring.rect.y + ring.rect.h < scroll || ring.rect.y > scroll + viewport {
                    continue;
                }
                let cx = ring.rect.x + ring.rect.w / 2.0;
                let cy = ring.rect.y + ring.rect.h / 2.0 - scroll;
                let r = (ring.rect.w - 5.0) / 2.0;
                let mut ellipse = D2D1_ELLIPSE {
                    radiusX: r,
                    radiusY: r,
                    ..Default::default()
                };
                ellipse.point.X = cx;
                ellipse.point.Y = cy;
                brush.SetColor(&color(
                    if dark { [65, 65, 67] } else { [229, 229, 232] },
                    1.0,
                ));
                target.DrawEllipse(&ellipse, &brush, 5.0, None);
                let Some(p) = ring.percent.filter(|p| *p > 0.0) else {
                    continue;
                };
                let rgb = ring.color.map(|v| {
                    ((v as f32) * (1.0 - ring.shade) + 255.0 * ring.shade).min(255.0) as u8
                });
                brush.SetColor(&color(rgb, 1.0));
                if p >= 100.0 {
                    target.DrawEllipse(&ellipse, &brush, 5.0, None);
                    continue;
                }
                let angle = p as f32 / 100.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                let mut end = ellipse.point;
                end.X = cx + r * angle.cos();
                end.Y = cy + r * angle.sin();
                let mut start = ellipse.point;
                start.Y = cy - r;
                let path = self.factory.CreatePathGeometry()?;
                let sink = path.Open()?;
                sink.BeginFigure(start, D2D1_FIGURE_BEGIN_HOLLOW);
                sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: end,
                    size: D2D_SIZE_F {
                        width: r,
                        height: r,
                    },
                    rotationAngle: 0.0,
                    sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                    arcSize: if p > 50.0 {
                        D2D1_ARC_SIZE_LARGE
                    } else {
                        D2D1_ARC_SIZE_SMALL
                    },
                });
                sink.EndFigure(D2D1_FIGURE_END_OPEN);
                sink.Close()?;
                target.DrawGeometry(&path, &brush, 5.0, None);
                for point in [start, end] {
                    target.FillEllipse(
                        &D2D1_ELLIPSE {
                            point,
                            radiusX: 2.5,
                            radiusY: 2.5,
                        },
                        &brush,
                    );
                }
            }
            for pill in &layout.pills {
                if !pill.visibility.shown(hover)
                    || pill.rect.y + pill.rect.h < scroll
                    || pill.rect.y > scroll + viewport
                {
                    continue;
                }
                let rgb = match pill.tone {
                    Tone::Green => {
                        if dark {
                            [74, 222, 128]
                        } else {
                            [21, 128, 54]
                        }
                    }
                    Tone::Orange => {
                        if dark {
                            [255, 193, 102]
                        } else {
                            [176, 99, 0]
                        }
                    }
                    Tone::Red => {
                        if dark {
                            [255, 134, 144]
                        } else {
                            [203, 38, 54]
                        }
                    }
                    Tone::Secondary => muted,
                };
                brush.SetColor(&color(rgb, 0.15));
                let rr = D2D1_ROUNDED_RECT {
                    rect: rect(pill.rect, scroll),
                    radiusX: 10.0,
                    radiusY: 10.0,
                };
                target.FillRoundedRectangle(&rr, &brush);
                brush.SetColor(&color(rgb, 1.0));
                let text: Vec<u16> = pill.text.encode_utf16().collect();
                target.DrawText(
                    &text,
                    self.font(12.0, true),
                    &rect(
                        Rect {
                            x: pill.rect.x + 7.0,
                            y: pill.rect.y + 1.0,
                            w: pill.rect.w - 14.0,
                            h: pill.rect.h,
                        },
                        scroll,
                    ),
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_CLIP,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            for t in &layout.texts {
                if !t.visibility.shown(hover)
                    || t.rect.y + t.rect.h < scroll
                    || t.rect.y > scroll + viewport
                {
                    continue;
                }
                brush.SetColor(&color(
                    match t.ink {
                        Ink::Normal => fg,
                        Ink::Muted => muted,
                        Ink::Accent => accent,
                        Ink::Error => {
                            if dark {
                                [255, 134, 144]
                            } else {
                                [203, 38, 54]
                            }
                        }
                    },
                    1.0,
                ));
                let text: Vec<u16> = t.text.encode_utf16().collect();
                if t.centered {
                    let centered = self.write.CreateTextLayout(
                        &text,
                        self.font(t.size, t.bold),
                        t.rect.w,
                        t.rect.h,
                    )?;
                    centered.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
                    centered.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
                    let mut origin = D2D1_ELLIPSE::default().point;
                    origin.X = t.rect.x;
                    origin.Y = t.rect.y - scroll;
                    target.DrawTextLayout(origin, &centered, &brush, D2D1_DRAW_TEXT_OPTIONS_CLIP);
                    continue;
                }
                target.DrawText(
                    &text,
                    self.font(t.size, t.bold),
                    &rect(t.rect, scroll),
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_CLIP,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            if let Some(hit) = focused.and_then(|i| layout.hits.get(i)) {
                brush.SetColor(&color(accent, 1.0));
                target.DrawRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: rect(hit.rect, scroll),
                        radiusX: 3.0,
                        radiusY: 3.0,
                    },
                    &brush,
                    1.0,
                    None,
                );
            }
            let result = target.EndDraw(None, None);
            if result.is_err() {
                self.target = None;
            }
            result
        }
    }
}
