//! The front of the device, drawn once into a picture: the case (body, bevel, grip texture, keys
//! on the side walls) and the faceplate (cover glass in the case's opening, display, front
//! details), from `odap-vdev frame` (the case and faceplate layers). The geometry follows the
//! concept's 3D model: a squircle footprint (a superellipse of exponent `n` over 1.45 × the
//! corner radius), the keys' silhouettes as its lathe and extrusions give them seen from the
//! front. Millimetres in, pixels out; device coordinates have y up from the centre (T1-021).

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Case {
    pub width_mm: f32,
    pub height_mm: f32,
    pub radius_mm: f32,
    pub squircle_n: f32,
    pub bevel_mm: f32,
    pub lip_mm: f32,
    pub color: String,
    #[serde(default)]
    pub texture: String,
    pub keys: Vec<CaseKey>,
}

#[derive(Deserialize, Clone)]
pub struct CaseKey {
    pub role: String,
    pub side: String,
    pub y_mm: f32,
    pub length_mm: f32,
    /// The rounded bevel around the face, beyond its length.
    #[serde(default)]
    pub bevel_mm: f32,
    pub protrusion_mm: f32,
    pub shape: String,
    pub finish: String,
    /// How far the key goes into the case when pressed (sideways, towards the case).
    #[serde(default)]
    pub travel_mm: f32,
    #[serde(default)]
    pub ribs: u32,
    #[serde(default)]
    pub rib_pitch_mm: f32,
    #[serde(default)]
    pub rib_height_mm: f32,
}

/// One part of the faceplate's front: the display, a sensor, the status LED, a microphone, or
/// on other faceplates a control of their own (`button`, `dial`, with its role).
#[derive(Deserialize, Clone, Default)]
#[serde(default)]
pub struct Part {
    pub kind: String,
    pub name: String,
    pub role: String,
    pub x_mm: f32,
    pub y_mm: f32,
    pub d_mm: f32,
    pub w_mm: f32,
    pub h_mm: f32,
    pub color: String,
}

/// The faceplate's front: its surface in the case's opening, and its parts.
#[derive(Deserialize, Clone)]
pub struct Front {
    pub surface: String,
    pub color: String,
    pub parts: Vec<Part>,
}

/// The faceplate's display: its physical size and shape decide how the front draws it.
#[derive(Clone)]
pub struct Panel {
    pub width_mm: f32,
    pub height_mm: f32,
    pub round: bool,
}

/// A rectangle in the picture, in logical pixels.
#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Drawn {
    /// RGBA, `width × height` pixels, at `scale` pixels per logical pixel.
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// The picture, the display and each key (by role), in logical pixels.
    pub size: (f32, f32),
    pub screen: Rect,
    pub keys: Vec<(String, Rect)>,
    /// The case's keys, each drawn alone, to lay under the front: a pressed key slides into
    /// the case by its travel, and the body covers what goes in.
    pub sprites: Vec<Sprite>,
    /// The status LED's window: centre and diameter, in logical pixels (zero without one).
    pub led: (f32, f32, f32),
}

/// One key of the case, drawn alone (premultiplied RGBA at `scale`), where it sits in the front
/// (logical pixels) and how far it moves when pressed (logical pixels, signed: towards the case).
pub struct Sprite {
    pub role: String,
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub rect: Rect,
    pub travel: f32,
}

fn hex(s: &str) -> [f32; 3] {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0x808080);
    [
        ((v >> 16) & 255) as f32 / 255.0,
        ((v >> 8) & 255) as f32 / 255.0,
        (v & 255) as f32 / 255.0,
    ]
}

/// A squircle centred at the origin: an estimate of the distance from the edge in mm, positive
/// inside (exact on the straight edges, close on the corners).
fn squircle(x: f32, y: f32, w: f32, h: f32, r: f32, n: f32) -> f32 {
    let p = (r * 1.45).min(w / 2.0).min(h / 2.0);
    let (ax, ay) = (x.abs() - (w / 2.0 - p), y.abs() - (h / 2.0 - p));
    if ax <= 0.0 || ay <= 0.0 {
        (w / 2.0 - x.abs()).min(h / 2.0 - y.abs())
    } else {
        p * (1.0 - ((ax / p).powf(n) + (ay / p).powf(n)).powf(1.0 / n))
    }
}

/// A rounded rectangle from `(x0, y0)` to `(x1, y1)`: distance from its edge, positive inside.
fn rounded(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (hw, hh) = ((x1 - x0) / 2.0 - r, (y1 - y0) / 2.0 - r);
    let (qx, qy) = ((x - cx).abs() - hw, (y - cy).abs() - hh);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    -(outside + qx.max(qy).min(0.0) - r)
}

/// Deterministic noise in [-1, 1] per pixel: the grip texture's grain.
fn grain(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 32_767.5 - 1.0
}

/// How far a key stands out of the wall at `along` mm from its centre, or `None` past its ends:
/// the silhouette its shape gives it seen from the front.
fn key_extent(k: &CaseKey, along: f32) -> Option<f32> {
    let half = k.length_mm / 2.0 + if k.shape == "disc" { 0.0 } else { k.bevel_mm };
    let a = along.abs();
    if a > half {
        return None;
    }
    let e = k.protrusion_mm;
    Some(match k.shape.as_str() {
        // The concept's lathe profile, scaled to the key: the silhouette is the highest point at
        // or beyond each radius (the concave face does not show from the front).
        "disc" => {
            let s = a / half * 6.5;
            let z = if s <= 5.74 {
                2.06
            } else if s <= 6.27 {
                2.06 + (1.92 - 2.06) * (s - 5.74) / (6.27 - 5.74)
            } else {
                1.92 + (1.55 - 1.92) * (s - 6.27) / (6.5 - 6.27)
            };
            z / 2.06 * e
        }
        // Main's pill, with its crests across it.
        "pill" => {
            let base = bevelled(e, k.bevel_mm, a - k.length_mm / 2.0);
            let mut z = base;
            for i in 0..k.ribs {
                let c = (i as f32 - (k.ribs as f32 - 1.0) / 2.0) * k.rib_pitch_mm;
                let d = (along - c).abs();
                if d < 0.275 {
                    z = z.max(e + k.rib_height_mm * (1.0 - (d / 0.275).powi(2)).sqrt());
                }
            }
            z
        }
        // A half capsule: the face, then its bevel down to the extrusion's side.
        _ => bevelled(e, k.bevel_mm, a - k.length_mm / 2.0),
    })
}

/// A face `e` out of the wall whose rounded bevel of `bevel` mm starts `past` mm beyond its end
/// (negative: on the face).
fn bevelled(e: f32, bevel: f32, past: f32) -> f32 {
    if past <= 0.0 || bevel <= 0.0 {
        e
    } else {
        e - bevel + (bevel * bevel - past.min(bevel).powi(2)).max(0.0).sqrt()
    }
}

/// A key's colour and coverage at `(mx, my)` device mm: its measured profile, straight on and
/// projected to the plane (rectangles with their outer corners rounded by the bevel), satin metal
/// with a soft band of light across it and the key light from above.
fn key_pixel(case: &Case, key: &CaseKey, mx: f32, my: f32, k: f32) -> Option<([f32; 3], f32)> {
    let aa = |d: f32| (d * k + 0.5).clamp(0.0, 1.0);
    let sign = if key.side == "left" { -1.0 } else { 1.0 };
    let out = sign * mx - case.width_mm / 2.0; // mm outside the wall
    if out < -0.6 - key.travel_mm {
        return None;
    }
    let along = my - key.y_mm;
    let e = key_extent(key, along)?;
    // The key continues into its slot as far as it travels, so a pressed key shows no gap.
    let cover = aa(e - out) * aa(out + 0.6 + key.travel_mm);
    if cover <= 0.0 {
        return None;
    }
    let base = hex(if key.finish == "anodised-orange" {
        "#ff6a2a"
    } else {
        "#b4b9bf"
    });
    let t = (out / e.max(0.1)).clamp(0.0, 1.0);
    let rel = (along / (key.length_mm / 2.0)).clamp(-1.0, 1.0);
    let shade = 0.72 + 0.42 * (std::f32::consts::PI * (0.15 + 0.7 * t)).sin() + 0.10 * rel;
    Some(([base[0] * shade, base[1] * shade, base[2] * shade], cover))
}

/// Draws one key of the case alone.
fn draw_key(case: &Case, key: &CaseKey, px_mm: f32, scale: f32, ox: f32, oy: f32) -> Sprite {
    let k = px_mm * scale;
    let sign = if key.side == "left" { -1.0 } else { 1.0 };
    let wall = sign * case.width_mm / 2.0;
    let reach = key.protrusion_mm + key.rib_height_mm + 0.2;
    let inner = 0.6 + key.travel_mm;
    let (x0, x1) = if sign < 0.0 {
        (wall - reach, wall + inner)
    } else {
        (wall - inner, wall + reach)
    };
    let half = key.length_mm / 2.0 + key.bevel_mm + 0.2;
    let (y0, y1) = (key.y_mm - half, key.y_mm + half);
    let (width, height) = (((x1 - x0) * k).ceil() as u32, ((y1 - y0) * k).ceil() as u32);
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for py in 0..height {
        for px in 0..width {
            let (mx, my) = (x0 + (px as f32 + 0.5) / k, y1 - (py as f32 + 0.5) / k);
            if let Some((c, a)) = key_pixel(case, key, mx, my, k) {
                let i = ((py * width + px) * 4) as usize;
                for j in 0..3 {
                    rgba[i + j] = (c[j].clamp(0.0, 1.0) * a * 255.0).round() as u8;
                }
                rgba[i + 3] = (a * 255.0).round() as u8;
            }
        }
    }
    Sprite {
        role: key.role.clone(),
        rgba,
        width,
        height,
        rect: Rect {
            x: (x0 + ox) * px_mm,
            y: (oy - y1) * px_mm,
            w: (x1 - x0) * px_mm,
            h: (y1 - y0) * px_mm,
        },
        travel: -sign * key.travel_mm * px_mm,
    }
}

/// Draws the front at `px_mm` logical pixels per mm, `scale` pixels per logical pixel.
pub fn draw(case: &Case, front: &Front, panel: &Panel, px_mm: f32, scale: f32) -> Drawn {
    let s = Scene::new(case, front, panel, px_mm, scale);
    let mut rgba = vec![0u8; (s.width * s.height * 4) as usize];
    for py in 0..s.height {
        for px in 0..s.width {
            let (c, alpha) = s.pixel(px, py);
            let o = ((py * s.width + px) * 4) as usize;
            for i in 0..3 {
                rgba[o + i] = (c[i].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            rgba[o + 3] = (alpha * 255.0).round() as u8;
        }
    }
    premultiply(&mut rgba);
    let to_logical =
        |x_mm: f32, y_mm: f32| ((x_mm + s.centre.0) * px_mm, (s.centre.1 - y_mm) * px_mm);
    let (sx, sy) = to_logical(s.screen.x0, s.screen.y0 + s.screen.h);
    let led = front.parts.iter().find(|p| p.kind == "led");
    let (lx, ly, ld) = led
        .map(|p| {
            let (x, y) = to_logical(p.x_mm, p.y_mm);
            (x, y, p.d_mm * px_mm)
        })
        .unwrap_or_default();
    let sprites = case
        .keys
        .iter()
        .map(|key| draw_key(case, key, px_mm, scale, s.centre.0, s.centre.1))
        .collect();
    Drawn {
        rgba,
        width: s.width,
        height: s.height,
        size: (s.size.0 * px_mm, s.size.1 * px_mm),
        screen: Rect {
            x: sx,
            y: sy,
            w: s.screen.w * px_mm,
            h: s.screen.h * px_mm,
        },
        keys: key_rects(case, front, &to_logical, px_mm),
        sprites,
        led: (lx, ly, ld),
    }
}

/// Premultiplied for display.
fn premultiply(rgba: &mut [u8]) {
    for p in rgba.as_chunks_mut::<4>().0 {
        let a = p[3] as u32;
        for c in p.iter_mut().take(3) {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
}

/// Where each control is pressed, in logical pixels: the case's keys on its sides, and the
/// faceplate's own controls where they are.
fn key_rects(
    case: &Case,
    front: &Front,
    to_logical: &dyn Fn(f32, f32) -> (f32, f32),
    px_mm: f32,
) -> Vec<(String, Rect)> {
    let mut keys: Vec<(String, Rect)> = case
        .keys
        .iter()
        .map(|key| {
            let sign = if key.side == "left" { -1.0 } else { 1.0 };
            let wall = sign * case.width_mm / 2.0;
            let tip = wall + sign * (key.protrusion_mm + key.rib_height_mm + 0.4);
            let (x0, x1) = if sign < 0.0 {
                (tip, wall + 0.6)
            } else {
                (wall - 0.6, tip)
            };
            let len = key.length_mm + 2.0 * key.bevel_mm;
            let (lx, ly) = to_logical(x0, key.y_mm + len / 2.0);
            let rect = Rect {
                x: lx,
                y: ly,
                w: (x1 - x0) * px_mm,
                h: len * px_mm,
            };
            (key.role.clone(), rect)
        })
        .collect();
    for part in front.parts.iter().filter(|p| !p.role.is_empty()) {
        let (w, h) = if part.d_mm > 0.0 {
            (part.d_mm, part.d_mm)
        } else {
            (part.w_mm, part.h_mm)
        };
        let (lx, ly) = to_logical(part.x_mm - w / 2.0, part.y_mm + h / 2.0);
        let rect = Rect {
            x: lx,
            y: ly,
            w: w * px_mm,
            h: h * px_mm,
        };
        keys.push((part.role.clone(), rect));
    }
    keys
}

/// What every pixel of the front needs, worked out once.
struct Scene<'a> {
    case: &'a Case,
    front: &'a Front,
    round: bool,
    scale: f32,
    /// Pixels per millimetre.
    k: f32,
    width: u32,
    height: u32,
    /// The picture in mm, and the device centre in mm from its top left.
    size: (f32, f32),
    centre: (f32, f32),
    body: [f32; 3],
    surface: [f32; 3],
    glossy: bool,
    /// The case's opening: width, height and corner radius.
    glass: (f32, f32, f32),
    /// The display's active area, in mm.
    screen: Area,
}

/// A rectangle in mm: its size and lower-left corner.
struct Area {
    w: f32,
    h: f32,
    x0: f32,
    y0: f32,
}

impl<'a> Scene<'a> {
    fn new(case: &'a Case, front: &'a Front, panel: &Panel, px_mm: f32, scale: f32) -> Self {
        let margin = case
            .keys
            .iter()
            .map(|k| k.protrusion_mm + k.rib_height_mm)
            .fold(0.0, f32::max)
            + 0.6;
        let (cw, ch) = (case.width_mm + 2.0 * margin, case.height_mm);
        let k = px_mm * scale;
        let display = front
            .parts
            .iter()
            .find(|p| p.kind == "display")
            .cloned()
            .unwrap_or_default();
        let (dw, dh) = (panel.width_mm, panel.height_mm);
        Scene {
            case,
            front,
            round: panel.round,
            scale,
            k,
            width: (cw * k).ceil() as u32,
            height: (ch * k).ceil() as u32,
            size: (cw, ch),
            centre: (cw / 2.0, ch / 2.0),
            body: hex(&case.color),
            surface: hex(&front.color),
            glossy: front.surface == "glass",
            glass: (
                case.width_mm - 2.0 * case.lip_mm,
                case.height_mm - 2.0 * case.lip_mm,
                case.radius_mm - case.lip_mm,
            ),
            screen: Area {
                w: dw,
                h: dh,
                x0: display.x_mm - dw / 2.0,
                y0: display.y_mm - dh / 2.0,
            },
        }
    }

    fn aa(&self, d: f32) -> f32 {
        (d * self.k + 0.5).clamp(0.0, 1.0)
    }

    fn body_distance(&self, x: f32, y: f32) -> f32 {
        let c = self.case;
        squircle(x, y, c.width_mm, c.height_mm, c.radius_mm, c.squircle_n)
    }

    /// One pixel of the front: its colour and coverage.
    fn pixel(&self, px: u32, py: u32) -> ([f32; 3], f32) {
        let (mx, my) = (
            (px as f32 + 0.5) / self.k - self.centre.0,
            self.centre.1 - (py as f32 + 0.5) / self.k,
        ); // device mm, y up
        let mut c = [0.0f32; 3];
        let mut alpha = 0.0f32;
        // The body: one piece, its bevel lit from the upper left, the grip texture over it.
        let d = self.body_distance(mx, my);
        let cover = self.aa(d);
        if cover > 0.0 {
            let shade = self.body_shade(mx, my, d, px, py);
            let mut col = [
                self.body[0] * shade,
                self.body[1] * shade,
                self.body[2] * shade,
            ];
            // The faceplate in the case's opening: its surface, then its parts.
            let (gw, gh, gr) = self.glass;
            let g = squircle(mx, my, gw, gh, gr, self.case.squircle_n);
            let gc = self.aa(g);
            if gc > 0.0 {
                let gcol = self.faceplate(mx, my, g);
                for i in 0..3 {
                    col[i] = col[i] * (1.0 - gc) + gcol[i] * gc;
                }
            }
            for i in 0..3 {
                c[i] = c[i] * (1.0 - cover) + col[i] * cover;
            }
            alpha = alpha.max(cover);
        }
        (c, alpha)
    }

    /// The body's light: a gentle vertical falloff, the bevel lit from the upper left (as in
    /// the concept's studio), and the grip texture.
    fn body_shade(&self, mx: f32, my: f32, d: f32, px: u32, py: u32) -> f32 {
        let case = self.case;
        let light = (-0.45f32, -0.89f32);
        let vertical = 1.10 - 0.20 * ((self.centre.1 - my) / case.height_mm);
        let mut shade = vertical;
        if d < case.bevel_mm {
            let e = 0.02;
            let gx = self.body_distance(mx + e, my) - d;
            let gy = self.body_distance(mx, my + e) - d;
            let len = (gx * gx + gy * gy).sqrt().max(1e-6);
            // Outward normal in picture coordinates (y down).
            let (nx, ny) = (-gx / len, gy / len);
            let lit = -(nx * light.0 + ny * light.1);
            shade *= 1.0 + 0.55 * (-lit) * (1.0 - d / case.bevel_mm);
        }
        if case.texture == "grip" {
            let step = (self.scale.max(1.0) as u32).max(1);
            shade *= 1.0 + 0.045 * grain(px / step, py / step);
        }
        shade
    }

    /// The faceplate's surface at a point `g` mm inside the opening (cover glass: glossy, with a
    /// faint reflection and a fine edge), with its parts over it.
    fn faceplate(&self, mx: f32, my: f32, g: f32) -> [f32; 3] {
        let mut gcol = self.surface;
        if self.glossy {
            let (gw, gh, _) = self.glass;
            let diag = (((mx / gw) - (my / gh)) * 0.5 + 0.5).clamp(0.0, 1.0);
            let refl = 0.035 * (1.0 - diag);
            let edge = if g < 0.18 {
                0.10 * (1.0 - g / 0.18) * if my > 0.0 { 1.0 } else { 0.35 }
            } else {
                0.0
            };
            gcol = [
                gcol[0] + refl + edge,
                gcol[1] + refl + edge,
                gcol[2] + refl + edge,
            ];
        }
        for part in &self.front.parts {
            let (cover, colour) = self.part(part, mx, my);
            if cover > 0.0 {
                for i in 0..3 {
                    gcol[i] = gcol[i] * (1.0 - cover) + colour[i] * cover;
                }
            }
        }
        gcol
    }

    /// A part of the faceplate at a point: how much it covers, and its colour.
    fn part(&self, part: &Part, mx: f32, my: f32) -> (f32, [f32; 3]) {
        let colour = |default: &str| {
            hex(if part.color.is_empty() {
                default
            } else {
                &part.color
            })
        };
        let r = ((mx - part.x_mm).powi(2) + (my - part.y_mm).powi(2)).sqrt();
        match part.kind.as_str() {
            // The display's active area: the live picture covers it.
            "display" => {
                let d = if self.round {
                    self.screen.w.min(self.screen.h) / 2.0 - r
                } else {
                    let (x0, y0) = (self.screen.x0, self.screen.y0);
                    rounded(mx, my, x0, y0, x0 + self.screen.w, y0 + self.screen.h, 0.2)
                };
                (self.aa(d), [0.0; 3])
            }
            // A face button: round, or a rounded rectangle, lit from above.
            "button" => {
                let d = if part.d_mm > 0.0 {
                    part.d_mm / 2.0 - r
                } else {
                    let (hw, hh) = (part.w_mm / 2.0, part.h_mm / 2.0);
                    let (x, y) = (part.x_mm, part.y_mm);
                    rounded(mx, my, x - hw, y - hh, x + hw, y + hh, hw.min(hh) * 0.4)
                };
                let c = colour("#b4b9bf");
                let lit = 1.0 + 0.12 * ((my - part.y_mm) / part.d_mm.max(part.h_mm).max(1.0));
                (self.aa(d), [c[0] * lit, c[1] * lit, c[2] * lit])
            }
            // A dial or wheel: a ring around a centre.
            "dial" => {
                let d = (part.d_mm / 2.0 - r).min(r - part.d_mm * 0.18);
                (self.aa(d), colour("#b4b9bf"))
            }
            // Sensors, the status LED's window, microphones: small round openings.
            _ => {
                let ds = part.d_mm / 2.0 - r;
                let c = colour("#050505");
                let rim = if ds < 0.12 { 0.06 } else { 0.0 };
                (self.aa(ds), [c[0] + rim, c[1] + rim, c[2] + rim])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squircle_edges_and_corner() {
        // 10 mm inside the left edge, 0 at the edge, negative outside.
        assert!((squircle(-19.75, 0.0, 59.5, 94.3, 9.0, 3.6) - 10.0).abs() < 1e-4);
        assert!(squircle(-29.75, 0.0, 59.5, 94.3, 9.0, 3.6).abs() < 1e-4);
        assert!(squircle(-30.5, 0.0, 59.5, 94.3, 9.0, 3.6) < 0.0);
        // The very corner of the bounding box is outside the squircle.
        assert!(squircle(29.7, 47.1, 59.5, 94.3, 9.0, 3.6) < 0.0);
    }

    #[test]
    fn keys_stand_out_by_their_shape() {
        let disc = CaseKey {
            role: "action".into(),
            side: "left".into(),
            y_mm: 0.0,
            length_mm: 13.0,
            protrusion_mm: 2.06,
            shape: "disc".into(),
            finish: String::new(),
            travel_mm: 0.0,
            bevel_mm: 0.0,
            ribs: 0,
            rib_pitch_mm: 0.0,
            rib_height_mm: 0.0,
        };
        assert!((key_extent(&disc, 0.0).unwrap() - 2.06).abs() < 1e-4);
        assert!(key_extent(&disc, 6.4).unwrap() < 2.06);
        assert!(key_extent(&disc, 7.0).is_none());
        let pill = CaseKey {
            shape: "pill".into(),
            length_mm: 15.0,
            bevel_mm: 0.4,
            protrusion_mm: 0.85,
            ribs: 5,
            rib_pitch_mm: 1.8,
            rib_height_mm: 0.29,
            ..disc
        };
        assert!(key_extent(&pill, 0.0).unwrap() > 0.85);
        assert!((key_extent(&pill, 0.9).unwrap() - 0.85).abs() < 1e-4);
    }

    #[test]
    fn a_volume_half_is_a_rectangle_with_rounded_outer_corners() {
        let up = CaseKey {
            role: "volume-up".into(),
            side: "left".into(),
            y_mm: 0.0,
            length_mm: 11.375,
            bevel_mm: 0.45,
            protrusion_mm: 1.0,
            shape: "half-up".into(),
            finish: String::new(),
            travel_mm: 0.0,
            ribs: 0,
            rib_pitch_mm: 0.0,
            rib_height_mm: 0.0,
        };
        // The measured profile: the face, then the bevel down to 0.55 mm at the very end.
        assert!((key_extent(&up, 0.0).unwrap() - 1.0).abs() < 1e-4);
        assert!((key_extent(&up, 11.375 / 2.0 + 0.45).unwrap() - 0.55).abs() < 1e-3);
        assert!(key_extent(&up, 11.375 / 2.0 + 0.5).is_none());
    }
}
