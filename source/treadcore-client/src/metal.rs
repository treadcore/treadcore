//! Brushed metal (Paul, 16 October: "the gray area needs rounded
//! outlines. make it shiny brushed metal ... with horizontal streaks
//! ... dynamically adjustable frames and frameboxes").
//!
//! The textures are drawn here, once, at startup, not loaded: a metal
//! plate, a button and the button pressed in. Each is a rounded
//! rectangle: a smooth chrome gradient top to bottom, fine horizontal
//! streaks (each row a little lighter or darker than its neighbours,
//! and wavering along its length, as a brush leaves it), a dark
//! outline, a bright rim along the top edge and a shade along the
//! bottom. A node wears one through `ImageNode` in sliced mode, nine
//! pieces: the corners are drawn at their own size whatever the node's,
//! the edges stretch one way and the middle both, so a frame of any
//! size, resized or not, keeps round corners and a one-pixel outline.
//! The streaks run with the stretch, and the plate is tall so a full
//! column stretches it little.
//!
//! The pressed button is the normal one turned over: the gradient dark
//! at the top, the rim light at the bottom, so it reads as pushed in.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::ui::widget::NodeImageMode;

/// The three textures, made at startup (`make`).
#[derive(Resource, Clone)]
pub struct Metal {
    pub plate: Handle<Image>,
    pub button: Handle<Image>,
    pub pressed: Handle<Image>,
}

/// A texture's recipe.
#[derive(Clone, Copy)]
pub struct Finish {
    pub w: u32,
    pub h: u32,
    /// Corner radius, pixels.
    pub radius: f32,
    /// The gradient top to bottom: (place 0..1, grey 0..255).
    pub stops: &'static [(f32, f32)],
    /// How far the streaks swing a row's grey, 0..1 of full.
    pub streak: f32,
    /// The rim: light along the top and shade along the bottom, or (for
    /// a button pushed in) the other way up.
    pub lit_from_above: bool,
    /// Varies the streaks between textures.
    pub seed: u32,
}

/// The plate: light chrome, its sheen near the top, darkening slowly.
pub const PLATE: Finish = Finish { w: 128, h: 512, radius: 9.0, stops: &[(0.0, 196.0), (0.03, 236.0), (0.10, 214.0), (0.45, 186.0), (0.85, 160.0), (1.0, 138.0)], streak: 0.05, lit_from_above: true, seed: 7 };
/// A button: the old brushed-metal window's capsule, light above and
/// dark below (#7a, #cf at 5%, #a3 at 10%, #66 at 90%, #44).
pub const BUTTON: Finish = Finish { w: 64, h: 64, radius: 14.0, stops: &[(0.0, 122.0), (0.05, 207.0), (0.10, 190.0), (0.90, 112.0), (1.0, 84.0)], streak: 0.04, lit_from_above: true, seed: 11 };
/// The button pressed: the gradient turned over.
pub const PRESSED: Finish = Finish { w: 64, h: 64, radius: 14.0, stops: &[(0.0, 84.0), (0.10, 112.0), (0.90, 176.0), (0.95, 196.0), (1.0, 140.0)], streak: 0.04, lit_from_above: false, seed: 11 };

/// The slice border for a finish: the corner and the rim inside it.
pub fn border(f: &Finish) -> f32 {
    f.radius + 3.0
}

/// A node's metal: the texture sliced so it fits any size.
pub fn node(image: Handle<Image>, f: &Finish) -> ImageNode {
    ImageNode {
        image,
        image_mode: NodeImageMode::Sliced(TextureSlicer { border: BorderRect::all(border(f)), center_scale_mode: SliceScaleMode::Stretch, sides_scale_mode: SliceScaleMode::Stretch, max_corner_scale: 1.0 }),
        ..default()
    }
}

/// The three textures into the assets.
pub fn make(images: &mut Assets<Image>) -> Metal {
    let mut add = |f: &Finish| {
        let data = pixels(f);
        images.add(Image::new(Extent3d { width: f.w, height: f.h, depth_or_array_layers: 1 }, TextureDimension::D2, data, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD))
    };
    Metal { plate: add(&PLATE), button: add(&BUTTON), pressed: add(&PRESSED) }
}

/// A fixed hash to 0..1, so the streaks are the same every run.
fn hash(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x & 0xffff) as f32 / 65535.0
}

fn gradient(stops: &[(f32, f32)], t: f32) -> f32 {
    let mut prev = stops[0];
    for &s in stops {
        if t <= s.0 {
            let span = (s.0 - prev.0).max(1e-6);
            let k = ((t - prev.0) / span).clamp(0.0, 1.0);
            // Smooth between stops: a sheen, not facets.
            let k = k * k * (3.0 - 2.0 * k);
            return prev.1 + (s.1 - prev.1) * k;
        }
        prev = s;
    }
    prev.1
}

/// Signed distance from a point to a rounded rectangle centred in the
/// texture: negative inside.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let qx = (x - w / 2.0).abs() - (w / 2.0 - r);
    let qy = (y - h / 2.0).abs() - (h / 2.0 - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

/// The RGBA pixels of a finish, row by row from the top.
pub fn pixels(f: &Finish) -> Vec<u8> {
    let (w, h) = (f.w as f32, f.h as f32);
    let mut out = Vec::with_capacity((f.w * f.h * 4) as usize);
    for y in 0..f.h {
        // A row's own shade, and how it wavers along the row: a long
        // slow swing and a faster one, as a brush's grain.
        let row = hash(y.wrapping_mul(2654435761) ^ f.seed) * 2.0 - 1.0;
        let row2 = hash(y.wrapping_mul(40503) ^ f.seed.wrapping_mul(97)) * 2.0 - 1.0;
        let (f1, p1) = (0.02 + 0.05 * hash(y ^ 0x51ed ^ f.seed), std::f32::consts::TAU * hash(y ^ 0xa11 ^ f.seed));
        for x in 0..f.w {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = rounded(px, py, w, h, f.radius);
            let alpha = (0.5 - d).clamp(0.0, 1.0);
            let t = py / h;
            let mut g = gradient(f.stops, t) / 255.0;
            let waver = 0.65 + 0.35 * (px * f1 + p1).sin();
            let fine = hash(x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663) ^ f.seed) * 2.0 - 1.0;
            g += f.streak * (0.7 * row * waver + 0.3 * row2) + f.streak * 0.25 * fine;
            // The rim, inside the outline: light on the lit edge, shade
            // on the other, fading in over three pixels.
            let rim = ((d + 4.5) / 3.0).clamp(0.0, 1.0) * if d < -1.2 { 1.0 } else { 0.0 };
            let upper = if f.lit_from_above { t < 0.5 } else { t >= 0.5 };
            if upper {
                g += (1.0 - g) * 0.55 * rim;
            } else {
                g *= 1.0 - 0.3 * rim;
            }
            // The outline: dark charcoal, about a pixel and a half.
            let line = ((d + 1.6) / 0.6).clamp(0.0, 1.0);
            g = g * (1.0 - line) + 0.17 * line;
            let g = g.clamp(0.0, 1.0);
            // A breath of blue in the steel.
            let (r, gg, b) = (g * 0.985, g * 0.995, (g * 1.02).min(1.0));
            out.extend_from_slice(&[(r * 255.0) as u8, (gg * 255.0) as u8, (b * 255.0) as u8, (alpha * 255.0) as u8]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(f: &Finish, p: &[u8], x: u32, y: u32) -> [u8; 4] {
        let i = ((y * f.w + x) * 4) as usize;
        [p[i], p[i + 1], p[i + 2], p[i + 3]]
    }

    /// The corners are cut round (clear), the outline dark, the middle
    /// opaque metal, and the rows differ: streaks.
    #[test]
    fn a_plate_is_rounded_outlined_and_streaked() {
        let p = pixels(&PLATE);
        assert_eq!(p.len(), (PLATE.w * PLATE.h * 4) as usize);
        assert_eq!(at(&PLATE, &p, 0, 0)[3], 0, "the corner is not cut");
        let edge = at(&PLATE, &p, PLATE.w / 2, 0);
        assert!(edge[3] > 200 && edge[0] < 80, "no dark outline on top: {:?}", edge);
        let mid = at(&PLATE, &p, PLATE.w / 2, PLATE.h / 2);
        assert!(mid[3] == 255 && mid[0] > 120, "the middle is not metal: {:?}", mid);
        let rows: Vec<u8> = (200..240).map(|y| at(&PLATE, &p, PLATE.w / 2, y)[1]).collect();
        let distinct = rows.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(distinct > 20, "no streaks: {:?}", rows);
        // Along a row the grey hardly changes: the streaks are horizontal.
        let along: Vec<i32> = (20..100).map(|x| at(&PLATE, &p, x, 220)[1] as i32).collect();
        let swing = along.iter().max().unwrap() - along.iter().min().unwrap();
        let across = rows.iter().map(|v| *v as i32).max().unwrap() - rows.iter().map(|v| *v as i32).min().unwrap();
        assert!(swing <= across, "streaks run across: along {} against across {}", swing, across);
    }

    /// Pressed in, the button is dark at the top where it was light.
    #[test]
    fn a_pressed_button_is_the_other_way_up() {
        let (n, p) = (pixels(&BUTTON), pixels(&PRESSED));
        let top = |f: &Finish, v: &[u8]| at(f, v, f.w / 2, 8)[1];
        let bottom = |f: &Finish, v: &[u8]| at(f, v, f.w / 2, f.h - 8)[1];
        assert!(top(&BUTTON, &n) > bottom(&BUTTON, &n));
        assert!(top(&PRESSED, &p) < bottom(&PRESSED, &p));
    }
}
