//! Clips (LR 102): a recorded run (`scenario::Recording`) drawn frame by
//! frame into pixels, for short animated pictures of the learned tank at
//! work. Plain Rust, no image library: each frame is an RGB buffer written
//! as a PPM, and `ffmpeg` (or anything that reads PPM) makes the GIF.
//!
//! The look is the site's field (assets/site.css, night): dark grass,
//! deep water, the learner's tank in sky blue with its armour bar, the
//! others in the foe's red, shells in orange with their landing marked,
//! drones as small diamonds, a hit as a white ring, and one line of text
//! along the top in a 3 by 5 pixel font.

use crate::scenario::{Frame, Recording};
use treadcore_sim::map::Terrain;
use treadcore_sim::*;

/// Pixels a tile.
pub const SCALE: i32 = 8;
/// The text strip's height, pixels.
pub const STRIP: i32 = 14;

/// An RGB image.
pub struct Image {
    pub w: i32,
    pub h: i32,
    pub px: Vec<[u8; 3]>,
}

/// A colour from its hex code, `0xRRGGBB`.
fn hex(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

impl Image {
    /// A `w` by `h` image filled with `bg`.
    pub fn new(w: i32, h: i32, bg: [u8; 3]) -> Image {
        Image { w, h, px: vec![bg; (w * h) as usize] }
    }

    /// One pixel; off the image, nothing.
    fn put(&mut self, x: i32, y: i32, c: [u8; 3]) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    /// A filled rectangle, its top-left at (x, y).
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 3]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, c);
            }
        }
    }

    /// A filled circle of radius r round (cx, cy).
    fn disc(&mut self, cx: i32, cy: i32, r: i32, c: [u8; 3]) {
        for yy in -r..=r {
            for xx in -r..=r {
                if xx * xx + yy * yy <= r * r {
                    self.put(cx + xx, cy + yy, c);
                }
            }
        }
    }

    /// A circle's outline of radius r round (cx, cy).
    fn ring(&mut self, cx: i32, cy: i32, r: i32, c: [u8; 3]) {
        for a in 0..(r * 8).max(16) {
            let t = a as f32 / (r * 8).max(16) as f32 * std::f32::consts::TAU;
            self.put(cx + (r as f32 * t.cos()).round() as i32, cy + (r as f32 * t.sin()).round() as i32, c);
        }
    }

    /// A straight line from (x0, y0) to (x1, y1).
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: [u8; 3]) {
        let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for i in 0..=n {
            self.put(x0 + (x1 - x0) * i / n, y0 + (y1 - y0) * i / n, c);
        }
    }

    /// One line of text, upper case, 3 by 5 glyphs scaled by `k`.
    pub fn text(&mut self, x: i32, y: i32, s: &str, k: i32, c: [u8; 3]) {
        let mut cx = x;
        for ch in s.chars() {
            let g = glyph(ch.to_ascii_uppercase());
            for (row, bits) in g.iter().enumerate() {
                for col in 0..3 {
                    if bits & (4 >> col) != 0 {
                        self.rect(cx + col * k, y + row as i32 * k, k, k, c);
                    }
                }
            }
            cx += 4 * k;
        }
    }

    /// As a binary PPM.
    pub fn ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.w, self.h).into_bytes();
        for p in &self.px {
            out.extend_from_slice(p);
        }
        out
    }
}

/// A 3 by 5 glyph, each row's three bits left to right.
fn glyph(c: char) -> [u8; 5] {
    match c {
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [6, 5, 5, 5, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 6, 3],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [6, 1, 2, 4, 7],
        '3' => [6, 1, 2, 1, 6],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 6, 1, 6],
        '6' => [3, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 6],
        '.' => [0, 0, 0, 0, 2],
        ':' => [0, 2, 0, 2, 0],
        '%' => [5, 1, 2, 4, 5],
        '-' => [0, 0, 7, 0, 0],
        '/' => [1, 1, 2, 4, 4],
        '+' => [0, 2, 7, 2, 0],
        '(' => [1, 2, 2, 2, 1],
        ')' => [4, 2, 2, 2, 4],
        _ => [0, 0, 0, 0, 0],
    }
}

/// A tile's colour, the site's night field.
fn terrain(t: Terrain) -> [u8; 3] {
    hex(match t {
        Terrain::Grass => 0x0f3a17,
        Terrain::Forest => 0x0b5a12,
        Terrain::Road => 0x3a3a3a,
        Terrain::Water => 0x03454f,
        Terrain::Bridge => 0x5a4a32,
        Terrain::Rubble => 0x4c3826,
        Terrain::Depot => 0x2b2f2b,
        Terrain::Wall => 0x8a8a8a,
        Terrain::Pad => 0x2f3a30,
        Terrain::Concrete => 0x555a55,
    })
}

/// One frame drawn: the recording's tiles, then what moves, seen from
/// `learner`'s side, with `caption` along the top.
pub fn render(rec: &Recording, f: &Frame, learner: PlayerId, caption: &str) -> Image {
    let (w, h) = (rec.size.0 * SCALE, rec.size.1 * SCALE + STRIP);
    let mut img = Image::new(w, h, hex(0x050605));
    for y in 0..rec.size.1 {
        for x in 0..rec.size.0 {
            let t = rec.tiles[(y * rec.size.0 + x) as usize];
            img.rect(x * SCALE, STRIP + y * SCALE, SCALE, SCALE, terrain(t));
        }
    }
    let px = |p: V2| (((p.x - rec.origin.0 as f32) * SCALE as f32) as i32, STRIP + ((p.y - rec.origin.1 as f32) * SCALE as f32) as i32);
    let side = |o: Option<PlayerId>| match o {
        None => hex(0x9fb0a2),
        Some(p) if p == learner => hex(0x73c7ff),
        Some(_) => hex(0xf2554a),
    };
    for (p, o) in &f.depots {
        let (x, y) = px(*p);
        img.ring(x, y, SCALE + 2, side(*o));
        img.ring(x, y, SCALE + 1, side(*o));
    }
    for (p, o, alive) in &f.pillboxes {
        let (x, y) = px(*p);
        let c = if *alive { side(*o) } else { hex(0x3c3e40) };
        img.rect(x - 5, y - 5, 10, 10, c);
        img.rect(x - 2, y - 2, 4, 4, hex(0x050605));
        if *alive && o.map(|o| o != learner).unwrap_or(false) {
            // A hostile pillbox's ring, 8 tiles, faint.
            img.ring(x, y, 8 * SCALE, hex(0x6a2a24));
        }
    }
    for (_, impact, direct) in &f.shells {
        if !direct {
            let (x, y) = px(*impact);
            img.line(x - 4, y - 4, x + 4, y + 4, hex(0xff8c33));
            img.line(x - 4, y + 4, x + 4, y - 4, hex(0xff8c33));
            img.ring(x, y, (2.5 * SCALE as f32) as i32, hex(0x804218));
        }
    }
    for (owner, pos, heading, armor, alive) in &f.tanks {
        let (x, y) = px(*pos);
        let c = if *alive { side(Some(*owner)) } else { hex(0x3c3e40) };
        img.disc(x, y, 5, c);
        let d = V2::from_heading(*heading).scale(9.0);
        img.line(x, y, x + d.x as i32, y + d.y as i32, c);
        if *alive {
            img.rect(x - 6, y - 10, 12, 2, hex(0x222222));
            img.rect(x - 6, y - 10, (12.0 * armor.clamp(0.0, 1.0)) as i32, 2, if *armor > 0.5 { hex(0x33f24d) } else if *armor > 0.25 { hex(0xffbf4d) } else { hex(0xf23326) });
        }
    }
    for (pos, _, direct) in &f.shells {
        let (x, y) = px(*pos);
        img.disc(x, y, if *direct { 1 } else { 2 }, if *direct { hex(0xffe08a) } else { hex(0xff8c33) });
    }
    for (owner, pos, kind) in &f.drones {
        let (x, y) = px(*pos);
        let c = match (kind, *owner == learner) {
            (DroneKind::Interceptor, true) => hex(0x73f0ff),
            (DroneKind::Recon, _) => hex(0xc8c8c8),
            (_, true) => hex(0x73c7ff),
            (_, false) => hex(0xff4a3a),
        };
        for k in 0..=3 {
            img.line(x - 3 + k, y, x, y - 3 + k, c);
            img.line(x + 3 - k, y, x, y + 3 - k, c);
        }
    }
    for p in &f.hits {
        let (x, y) = px(*p);
        img.ring(x, y, 9, hex(0xffffff));
        img.ring(x, y, 10, hex(0xffffff));
    }
    img.rect(0, 0, w, STRIP, hex(0x050605));
    img.text(4, 2, caption, 2, hex(0x33f24d));
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame draws: the learner's tank in its colour where it stands,
    /// a hostile in red, and text in the strip; the PPM has its header.
    #[test]
    fn a_frame_draws_where_things_are() {
        let rec = Recording { every: 1, origin: (0, 0), size: (10, 10), tiles: vec![Terrain::Grass; 100], frames: Vec::new() };
        let f = Frame { t: 0.0, tanks: vec![(0, V2::new(2.5, 2.5), 0.0, 1.0, true), (1, V2::new(7.5, 7.5), 0.0, 1.0, true)], ..Default::default() };
        let img = render(&rec, &f, 0, "T 0.0");
        let at = |x: i32, y: i32| img.px[(y * img.w + x) as usize];
        assert_eq!(at(20, STRIP + 20), hex(0x73c7ff));
        assert_eq!(at(60, STRIP + 60), hex(0xf2554a));
        assert!(img.px[..(STRIP * img.w) as usize].iter().any(|p| *p == hex(0x33f24d)), "text drawn");
        assert!(img.ppm().starts_with(b"P6\n80 94\n255\n"));
    }
}
