//! Theme colours, small widgets and text helpers.
use crate::StoreSort;
use eframe::egui::{self, vec2, Align, Color32, CornerRadius, Layout, Margin, RichText, Sense, Stroke, Ui};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set_english(on: bool) {
    ENGLISH.store(on, Ordering::Relaxed);
}

pub fn is_english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

/// Picks the German or English text for the current language.
pub fn t(de: &'static str, en: &'static str) -> &'static str {
    if is_english() { en } else { de }
}

/// `format!` with a German and an English pattern (positional `{}` only).
#[macro_export]
macro_rules! tf {
    ($de:literal, $en:literal $(, $a:expr)* $(,)?) => {
        if $crate::ui::is_english() { format!($en $(, $a)*) } else { format!($de $(, $a)*) }
    };
}

/// Labels the library stores in German, shown in the current language.
pub fn stored_label(s: &str) -> String {
    match s {
        "Lokal gefunden" => t("Lokal gefunden", "Found locally").into(),
        "Lokale Datei" => t("Lokale Datei", "Local file").into(),
        other => other.into(),
    }
}

/// One colour theme for the whole UI.
#[derive(Clone, Copy)]
pub struct Palette {
    pub id: &'static str,
    pub de: &'static str,
    pub en: &'static str,
    pub dark: bool,
    pub bg: Color32,
    pub card: Color32,
    pub fg: Color32,
    pub primary: Color32,
    pub primary_fg: Color32,
    pub secondary: Color32,
    pub secondary_fg: Color32,
    pub muted: Color32,
    pub muted_fg: Color32,
    pub border: Color32,
    pub sidebar_border: Color32,
    pub destructive: Color32,
    pub toast: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// "standard" and "midnight" are the original's dark and "Remlock" themes (CSS custom properties,
/// HSL converted); "light" is its `.light` theme; "magma" is our own orange one.
pub const THEMES: [Palette; 4] = [
    Palette { id: "standard", de: "Standard", en: "Default", dark: true, bg: rgb(17, 15, 14), card: rgb(25, 22, 21), fg: rgb(252, 249, 242), primary: rgb(239, 224, 190), primary_fg: rgb(17, 15, 14), secondary: rgb(45, 39, 37), secondary_fg: rgb(217, 183, 104), muted: rgb(38, 38, 38), muted_fg: rgb(163, 163, 163), border: rgb(38, 38, 38), sidebar_border: rgb(39, 39, 42), destructive: rgb(127, 29, 29), toast: rgb(37, 33, 31) },
    Palette { id: "midnight", de: "Mitternacht", en: "Midnight", dark: true, bg: rgb(24, 25, 42), card: rgb(30, 32, 52), fg: rgb(239, 242, 251), primary: rgb(154, 166, 254), primary_fg: rgb(30, 32, 52), secondary: rgb(48, 52, 79), secondary_fg: rgb(197, 240, 252), muted: rgb(45, 48, 72), muted_fg: rgb(179, 183, 209), border: rgb(58, 62, 94), sidebar_border: rgb(59, 63, 93), destructive: rgb(175, 48, 29), toast: rgb(36, 38, 62) },
    Palette { id: "magma", de: "Magma", en: "Magma", dark: true, bg: rgb(14, 12, 12), card: rgb(24, 20, 19), fg: rgb(245, 240, 235), primary: rgb(255, 145, 48), primary_fg: rgb(24, 14, 8), secondary: rgb(46, 32, 26), secondary_fg: rgb(255, 178, 102), muted: rgb(38, 33, 31), muted_fg: rgb(168, 158, 152), border: rgb(44, 38, 36), sidebar_border: rgb(44, 38, 36), destructive: rgb(150, 32, 32), toast: rgb(36, 28, 25) },
    Palette { id: "light", de: "Hell", en: "Light", dark: false, bg: rgb(250, 250, 249), card: rgb(255, 255, 255), fg: rgb(45, 39, 37), primary: rgb(239, 224, 190), primary_fg: rgb(17, 15, 14), secondary: rgb(245, 245, 245), secondary_fg: rgb(120, 90, 30), muted: rgb(241, 241, 240), muted_fg: rgb(115, 115, 115), border: rgb(229, 229, 229), sidebar_border: rgb(229, 229, 229), destructive: rgb(220, 58, 58), toast: rgb(255, 255, 255) },
];

static THEME: AtomicUsize = AtomicUsize::new(0);

pub fn set_theme(id: &str) {
    THEME.store(THEMES.iter().position(|p| p.id == id).unwrap_or(0), Ordering::Relaxed);
}

/// The current theme's colours.
pub fn pal() -> Palette {
    THEMES[THEME.load(Ordering::Relaxed).min(THEMES.len() - 1)]
}

pub const OK: Color32 = Color32::from_rgb(110, 190, 120);
pub const BAD: Color32 = Color32::from_rgb(230, 100, 90);
pub const RADIUS: u8 = 8; // --radius .5rem

/// Readable text colour on a filled background.
pub fn on(fill: Color32) -> Color32 {
    let l = 0.299 * fill.r() as f32 + 0.587 * fill.g() as f32 + 0.114 * fill.b() as f32;
    if l > 150.0 { rgb(20, 18, 17) } else { rgb(250, 248, 244) }
}

/// The heavier font for titles (Segoe UI Semibold when Windows has it).
pub fn semibold() -> egui::FontFamily {
    egui::FontFamily::Name("semibold".into())
}

pub fn setup_style(ctx: &egui::Context) {
    // Segoe UI first (as Windows apps look), then egui's own fonts, then CJK fonts for Nexus texts.
    let mut fonts = egui::FontDefinitions::default();
    let mut add = |path: &str, fams: &[egui::FontFamily], front: bool| {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(path.into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            for fam in fams {
                let list = fonts.families.entry(fam.clone()).or_default();
                if front { list.insert(0, path.into()) } else { list.push(path.into()) }
            }
        }
    };
    add(r"C:\Windows\Fonts\segoeui.ttf", &[egui::FontFamily::Proportional], true);
    for f in [r"C:\Windows\Fonts\msyh.ttc", r"C:\Windows\Fonts\YuGothM.ttc", r"C:\Windows\Fonts\malgun.ttf"] {
        add(f, &[egui::FontFamily::Proportional, egui::FontFamily::Monospace], false);
    }
    let mut bold = fonts.families[&egui::FontFamily::Proportional].clone();
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\seguisb.ttf") {
        fonts.font_data.insert("seguisb".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
        bold.insert(0, "seguisb".into());
    }
    fonts.families.insert(semibold(), bold);
    ctx.set_fonts(fonts);
    apply_theme(ctx);
}

/// Applies the current theme's colours, spacing and text sizes.
pub fn apply_theme(ctx: &egui::Context) {
    let p = pal();
    let mut v = if p.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.window_stroke = Stroke::new(1.0f32, p.border);
    v.window_corner_radius = CornerRadius::same(12);
    v.extreme_bg_color = if p.dark { lerp_color(p.bg, Color32::BLACK, 0.35) } else { p.card };
    v.faint_bg_color = p.card;
    v.selection.bg_fill = lerp_color(p.card, p.primary, 0.4);
    v.selection.stroke = Stroke::new(1.0f32, p.primary);
    v.hyperlink_color = p.primary;
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(if p.dark { 110 } else { 40 }) };
    v.window_shadow = v.popup_shadow;
    let hover_stroke = lerp_color(p.border, p.muted_fg, 0.35);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open, &mut v.widgets.noninteractive] {
        w.corner_radius = CornerRadius::same(8);
        w.fg_stroke = Stroke::new(1.0f32, p.fg);
        w.expansion = 0.0;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, p.border);
    v.widgets.inactive.weak_bg_fill = p.muted;
    v.widgets.inactive.bg_fill = p.muted;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0f32, p.border);
    for w in [&mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.weak_bg_fill = p.secondary;
        w.bg_fill = p.secondary;
        w.bg_stroke = Stroke::new(1.0f32, hover_stroke);
    }
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        use egui::{FontId, TextStyle};
        s.text_styles.insert(TextStyle::Small, FontId::proportional(11.5));
        s.text_styles.insert(TextStyle::Body, FontId::proportional(14.0));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(TextStyle::Heading, FontId::new(22.0, semibold()));
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.button_padding = vec2(12.0, 6.0);
        s.spacing.interact_size.y = 32.0;
        s.spacing.scroll = egui::style::ScrollStyle::floating();
        s.animation_time = 0.15;
    });
}

/// Lays out `n` fixed-size cells in as many columns as fit.
pub fn grid(ui: &mut Ui, n: usize, w: f32, h: f32, mut cell: impl FnMut(&mut Ui, usize)) {
    let gap = 14.0;
    let cols = (((ui.available_width() + gap) / (w + gap)).floor() as usize).max(1);
    for row in 0..n.div_ceil(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for c in 0..cols {
                let i = row * cols + c;
                if i >= n {
                    break;
                }
                ui.allocate_ui_with_layout(vec2(w, h), Layout::top_down(Align::Min), |ui| {
                    ui.set_min_size(vec2(w, h));
                    ui.set_max_size(vec2(w, h));
                    cell(ui, i);
                });
            }
        });
        ui.add_space(gap - 6.0);
    }
}

/// A mod card's background: soft shadow and a border that warms up on hover.
pub fn card_frame(ui: &Ui, rect: egui::Rect, resp: &egui::Response) {
    let p = pal();
    let h = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered(), 0.15);
    let alpha = if p.dark { 60.0 + 70.0 * h } else { 14.0 + 26.0 * h };
    let shadow = egui::epaint::Shadow { offset: [0, (2.0 + 5.0 * h) as i8], blur: (10.0 + 16.0 * h) as u8, spread: 0, color: Color32::from_black_alpha(alpha as u8) };
    ui.painter().add(shadow.as_shape(rect, CornerRadius::same(RADIUS)));
    let stroke = lerp_color(p.border, lerp_color(p.border, p.primary, 0.45), h);
    ui.painter().rect(rect, CornerRadius::same(RADIUS), p.card, Stroke::new(1.0f32, stroke), egui::StrokeKind::Inside);
}

fn soft_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow { offset: [0, 2], blur: 10, spread: 0, color: Color32::from_black_alpha(if pal().dark { 60 } else { 12 }) }
}

pub fn card<R>(ui: &mut Ui, margin: i8, body: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new().fill(pal().card).stroke(Stroke::new(1.0f32, pal().border)).shadow(soft_shadow()).corner_radius(CornerRadius::same(RADIUS)).inner_margin(Margin::same(margin)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        body(ui)
    })
    .inner
}

pub fn draw_cover(ui: &Ui, r: egui::Rect, tex: Option<&egui::TextureHandle>, name: &str) {
    draw_cover_rounded(ui, r, tex, name, CornerRadius { nw: RADIUS, ne: RADIUS, sw: 0, se: 0 });
}

/// Fills `r` with the picture, cropped to cover it, or a placeholder with the mod's initials.
pub fn draw_cover_rounded(ui: &Ui, r: egui::Rect, tex: Option<&egui::TextureHandle>, name: &str, cr: CornerRadius) {
    match tex {
        Some(t) => {
            let [tw, th] = t.size();
            let (ta, ra) = (tw as f32 / th as f32, r.width() / r.height());
            let uv = if ta > ra {
                let w = ra / ta;
                egui::Rect::from_min_max(egui::pos2((1.0 - w) / 2.0, 0.0), egui::pos2((1.0 + w) / 2.0, 1.0))
            } else {
                let h = ta / ra;
                egui::Rect::from_min_max(egui::pos2(0.0, (1.0 - h) / 2.0), egui::pos2(1.0, (1.0 + h) / 2.0))
            };
            egui::Image::new(egui::load::SizedTexture::from_handle(t)).uv(uv).corner_radius(cr).paint_at(ui, r);
        }
        None => {
            ui.painter().rect_filled(r, cr, pal().secondary);
            let initials: String = name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_uppercase();
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, initials, egui::FontId::proportional(28.0), pal().secondary_fg);
        }
    }
}

pub const FRESH: Color32 = Color32::from_rgb(36, 120, 70);
pub const WARN: Color32 = Color32::from_rgb(176, 110, 20);
pub const NEW: Color32 = Color32::from_rgb(40, 90, 170);

pub fn overlay_badge(ui: &Ui, img: egui::Rect, text: &str, fill: Color32) {
    overlay_badge_at(ui, img, text, fill, false);
}

/// A small label on a card's picture, top right (or top left).
pub fn overlay_badge_at(ui: &Ui, img: egui::Rect, text: &str, fill: Color32, left: bool) {
    let g = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::new(11.0, semibold()), on(fill));
    let x = if left { img.left() + 8.0 } else { img.right() - g.size().x - 20.0 };
    let r = egui::Rect::from_min_size(egui::pos2(x, img.top() + 8.0), g.size() + vec2(12.0, 6.0));
    ui.painter().rect_filled(r, CornerRadius::same(6), fill.gamma_multiply(0.94));
    ui.painter().galley(r.min + vec2(6.0, 3.0), g, on(fill));
}

pub fn pill(ui: &mut Ui, text: &str, fill: Color32, color: Color32) {
    egui::Frame::new().fill(fill).corner_radius(CornerRadius::same(6)).inner_margin(Margin::symmetric(8, 2)).show(ui, |ui| {
        ui.label(RichText::new(text).small().color(color));
    });
}

pub fn nav_group(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).small().color(pal().muted_fg));
    ui.add_space(2.0);
}

#[derive(Clone, Copy, PartialEq)]
enum Btn {
    Primary,
    Outline,
    Ghost,
    Danger,
}

/// A button that eases its colours on hover and press.
fn styled_button(ui: &mut Ui, text: &str, kind: Btn, active: bool) -> egui::Response {
    let p = pal();
    let family = if kind == Btn::Ghost && !active { egui::FontFamily::Proportional } else { semibold() };
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::new(14.0, family), Color32::PLACEHOLDER);
    let size = vec2(galley.size().x + 28.0, 32.0f32.max(galley.size().y + 12.0));
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
        let lift = |c: Color32| if p.dark { lerp_color(c, Color32::WHITE, 0.12 * h) } else { lerp_color(c, Color32::BLACK, 0.06 * h) };
        let (mut fill, stroke, text_col) = match kind {
            Btn::Primary => (lift(p.primary), None, p.primary_fg),
            Btn::Danger => (lift(p.destructive), None, on(p.destructive)),
            Btn::Outline => (lerp_color(p.bg, p.secondary, h), Some(lerp_color(p.border, p.muted_fg, 0.4 * h)), p.fg),
            Btn::Ghost if active => (p.secondary, None, p.fg),
            Btn::Ghost => (p.secondary.gamma_multiply(h), None, lerp_color(p.muted_fg, p.fg, h)),
        };
        if resp.is_pointer_button_down_on() {
            fill = lerp_color(fill, Color32::BLACK, 0.1);
        }
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(8), fill);
        if let Some(s) = stroke {
            painter.rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0f32, s), egui::StrokeKind::Inside);
        }
        painter.galley(rect.center() - galley.size() / 2.0, galley, text_col);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    styled_button(ui, text, Btn::Primary, false)
}

pub fn outline_button(ui: &mut Ui, text: &str) -> egui::Response {
    styled_button(ui, text, Btn::Outline, false)
}

pub fn danger_button(ui: &mut Ui, text: &str) -> egui::Response {
    styled_button(ui, text, Btn::Danger, false)
}

pub fn tab_button(ui: &mut Ui, text: &str, active: bool) -> egui::Response {
    styled_button(ui, text, Btn::Ghost, active)
}

/// Small round icon button (delete, open on Nexus) on cards.
pub fn icon_button(ui: &mut Ui, icon: &str, tip: &str) -> egui::Response {
    let p = pal();
    let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
    ui.painter().circle_filled(rect.center(), 14.0, p.secondary.gamma_multiply(h));
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(13.0), lerp_color(p.muted_fg, p.fg, h));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip)
}

/// A clickable preview of a theme: its background, a card and an accent bar.
pub fn theme_tile(ui: &mut Ui, th: &Palette, selected: bool) -> egui::Response {
    let cur = pal();
    let (rect, resp) = ui.allocate_exact_size(vec2(150.0, 104.0), Sense::click());
    let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
    let p = ui.painter();
    let preview = rect.with_max_y(rect.top() + 72.0);
    p.rect_filled(preview, CornerRadius::same(RADIUS), th.bg);
    let side = egui::Rect::from_min_size(preview.min + vec2(8.0, 8.0), vec2(30.0, preview.height() - 16.0));
    p.rect_filled(side, CornerRadius::same(4), th.card);
    p.rect_filled(egui::Rect::from_min_size(side.min + vec2(4.0, 6.0), vec2(22.0, 6.0)), CornerRadius::same(3), th.primary);
    let body = egui::Rect::from_min_max(egui::pos2(side.right() + 8.0, side.top()), preview.max - vec2(8.0, 8.0));
    p.rect_filled(body, CornerRadius::same(4), th.card);
    p.rect_filled(egui::Rect::from_min_size(body.min + vec2(8.0, 8.0), vec2(body.width() * 0.6, 6.0)), CornerRadius::same(3), th.fg.gamma_multiply(0.8));
    p.rect_filled(egui::Rect::from_min_size(body.min + vec2(8.0, 20.0), vec2(body.width() * 0.4, 5.0)), CornerRadius::same(3), th.muted_fg.gamma_multiply(0.7));
    p.rect_filled(egui::Rect::from_min_size(egui::pos2(body.right() - 30.0, body.bottom() - 14.0), vec2(22.0, 8.0)), CornerRadius::same(4), th.primary);
    let ring = if selected { cur.primary } else { lerp_color(cur.border, cur.muted_fg, 0.5 * h) };
    p.rect_stroke(preview, CornerRadius::same(RADIUS), Stroke::new(if selected { 2.0f32 } else { 1.0 }, ring), egui::StrokeKind::Outside);
    let name = if is_english() { th.en } else { th.de };
    let label = if selected { format!("✔ {name}") } else { name.to_string() };
    p.text(egui::pos2(rect.left() + 2.0, preview.bottom() + 16.0), egui::Align2::LEFT_CENTER, label, egui::FontId::new(13.0, semibold()), if selected { cur.fg } else { cur.muted_fg });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn search_box(ui: &mut Ui, text: &mut String, hint: &str, w: f32) {
    ui.add(egui::TextEdit::singleline(text).hint_text(format!("🔍 {hint}")).desired_width(w).margin(Margin::symmetric(8, 6)));
}

/// On/off switch, like the original's.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(38.0, 20.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let c = pal();
    let t = ui.ctx().animate_bool_with_time(resp.id, *on, 0.18);
    let h = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.12);
    let off = lerp_color(c.card, c.muted_fg, 0.35 + 0.1 * h);
    let bg = lerp_color(off, c.primary, t);
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(10), bg);
    let x = egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), t);
    let knob = lerp_color(if c.dark { c.fg } else { c.card }, c.primary_fg, t);
    p.circle_filled(egui::pos2(x, rect.center().y + 0.5), 7.5, Color32::from_black_alpha(50));
    p.circle_filled(egui::pos2(x, rect.center().y), 7.5, knob);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub fn dashed_rect(p: &egui::Painter, r: egui::Rect, c: Color32) {
    let s = Stroke::new(1.0f32, c);
    for (a, b) in [(r.left_top(), r.right_top()), (r.right_top(), r.right_bottom()), (r.right_bottom(), r.left_bottom()), (r.left_bottom(), r.left_top())] {
        p.extend(egui::Shape::dashed_line(&[a, b], s, 6.0, 4.0));
    }
}

pub fn notice(ui: &mut Ui, text: &str, fill: Color32) {
    egui::Frame::new().fill(fill).corner_radius(CornerRadius::same(RADIUS)).inner_margin(Margin::same(12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(text).color(on(fill)));
    });
}

pub fn empty_state(ui: &mut Ui, title: &str, text: &str) {
    ui.add_space(60.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(title).size(18.0).family(semibold()).color(pal().fg));
        ui.label(RichText::new(text).color(pal().muted_fg));
    });
}

pub fn section_title(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(17.0).family(semibold()).color(pal().fg));
    ui.add_space(4.0);
}

pub fn settings_card(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    card(ui, 16, |ui| {
        ui.label(RichText::new(title).size(16.0).family(semibold()).color(pal().fg));
        ui.add_space(4.0);
        body(ui);
    });
    ui.add_space(10.0);
}

pub fn store_sort_label(s: StoreSort) -> &'static str {
    match s {
        StoreSort::Endorsements => t("Beliebteste", "Most endorsed"),
        StoreSort::Downloads => t("Meiste Downloads", "Most downloaded"),
        StoreSort::Updated => t("Zuletzt aktualisiert", "Last updated"),
        StoreSort::Newest => t("Neueste", "Newest"),
        StoreSort::Name => "Name",
    }
}

pub fn file_cat(c: &str) -> String {
    match c {
        "MAIN" => t("Hauptdatei", "Main file"),
        "OPTIONAL" => "Optional",
        "UPDATE" => "Update",
        "MISCELLANEOUS" => t("Sonstiges", "Miscellaneous"),
        "OLD_VERSION" => t("Alte Version", "Old version"),
        other => other,
    }
    .to_string()
}

pub fn shorten(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() <= n {
        s
    } else {
        format!("{}…", s.chars().take(n).collect::<String>().trim_end())
    }
}

/// "Workbench Full Control V2.3" and "Workbench Full Control" -> "workbenchfullcontrol".
pub fn name_key(name: &str) -> String {
    name.split_whitespace()
        .filter(|w| {
            let w = w.trim_start_matches(['v', 'V']);
            !(w.chars().next().map_or(false, |c| c.is_ascii_digit()) && w.chars().all(|c| c.is_ascii_digit() || c == '.'))
        })
        .flat_map(|w| w.chars())
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

pub fn short_num(n: u64) -> String {
    match n {
        0..=999 => n.to_string(),
        1000..=999_999 => format!("{:.1}k", n as f64 / 1000.0),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

pub fn fmt_size(b: u64) -> String {
    match b {
        0..=1023 => format!("{b} B"),
        1024..=1_048_575 => format!("{:.1} KB", b as f64 / 1024.0),
        _ => format!("{:.1} MB", b as f64 / 1_048_576.0),
    }
}

/// "2026-05-21T21:20:42Z" -> "21.05.2026"
pub fn fmt_date(s: &str) -> String {
    if s.len() >= 10 && is_english() {
        s[..10].to_string()
    } else if s.len() >= 10 {
        format!("{}.{}.{}", &s[8..10], &s[5..7], &s[0..4])
    } else {
        s.to_string()
    }
}

/// Created within the last 7 days.
pub fn is_new(created: &str) -> bool {
    let c = parse_time(created);
    c > 0 && crate::library::now().saturating_sub(c) < 7 * 86400
}

/// Unix seconds to "YYYY-MM-DDT00:00:00Z" (date part only), the inverse of `parse_time`'s date.
pub fn unix_to_iso(secs: u64) -> String {
    // Civil from days (Howard Hinnant).
    let z = (secs / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T00:00:00Z")
}

/// RFC 3339 UTC timestamp to Unix seconds (0 when unreadable).
pub fn parse_time(s: &str) -> u64 {
    let n = |a: usize, b: usize| s.get(a..b).and_then(|x| x.parse::<i64>().ok());
    let (Some(y), Some(m), Some(d)) = (n(0, 4), n(5, 7), n(8, 10)) else { return 0 };
    let (hh, mm, ss) = (n(11, 13).unwrap_or(0), n(14, 16).unwrap_or(0), n(17, 19).unwrap_or(0));
    // Days from civil (Howard Hinnant).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    (days * 86400 + hh * 3600 + mm * 60 + ss).max(0) as u64
}

#[cfg(test)]
mod tests {
    #[test]
    fn time_and_formats() {
        assert_eq!(super::parse_time("1970-01-02T00:00:00Z"), 86400);
        assert_eq!(super::unix_to_iso(1795098976), "2026-11-19T00:00:00Z");
        assert_eq!(super::parse_time(&super::unix_to_iso(1779398442)), 1779321600);
        assert_eq!(super::parse_time("2026-05-21T21:20:42Z"), 1779398442);
        assert_eq!(super::fmt_date("2026-05-21T21:20:42Z"), "21.05.2026");
        assert_eq!(super::short_num(5079), "5.1k");
        assert_eq!(super::name_key("Workbench Full Control V2.3"), super::name_key("Workbench Full Control"));
        assert_eq!(super::name_key("Rogue Core Mod Manager"), super::name_key("RogueCore Mod Manager"));
        assert_ne!(super::name_key("Heat Crosshair 1.0.0"), super::name_key("Heat"));
    }
}
