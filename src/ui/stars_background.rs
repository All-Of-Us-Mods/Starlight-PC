//! Decorative drifting starfield, modeled on upstream's StarBackground:
//! stars distributed by Poisson-disc-style sampling (minimum spacing, so
//! they never overlap or clump), duplicated 2×2 into a 200%-sized field that
//! slowly translates by exactly one tile and loops seamlessly. Star color is
//! fixed amber (upstream's `#fbbf24`) regardless of theme; toggled by the
//! "Floating stars background" setting.

use std::sync::OnceLock;
use std::time::Duration;

use gpui::*;
use gpui_component::Icon;

use crate::ui::icon::AppIcon;

/// Upstream's star color (Tailwind amber-400).
const STAR_COLOR: u32 = 0xfbbf24;
/// Minimum spacing between star centers, as a fraction of the window.
const MIN_DIST: f32 = 0.075;
const MAX_STARS: usize = 120;
const DRIFT_SECS: u64 = 90;

struct Star {
    x: f32,
    y: f32,
    size: f32,
    opacity: f32,
}

/// Star placements within one tile. Deterministic (fixed-seed LCG) so the
/// field looks the same every launch; sampled once and cached.
///
/// Rejection sampling with a *toroidal* min-distance check — the tile wraps,
/// so spacing also holds across tile seams when the field repeats.
fn tile_stars() -> &'static [Star] {
    static TILE: OnceLock<Vec<Star>> = OnceLock::new();
    TILE.get_or_init(|| {
        let mut seed: u32 = 0x5EED_5AFE;
        let mut next = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / 16_777_216.0
        };

        let mut stars: Vec<Star> = Vec::new();
        for _ in 0..4000 {
            if stars.len() >= MAX_STARS {
                break;
            }
            let x = next();
            let y = next();
            let too_close = stars.iter().any(|s| {
                let dx = (s.x - x).abs();
                let dx = dx.min(1.0 - dx);
                let dy = (s.y - y).abs();
                let dy = dy.min(1.0 - dy);
                dx * dx + dy * dy < MIN_DIST * MIN_DIST
            });
            if too_close {
                continue;
            }
            stars.push(Star {
                x,
                y,
                size: 6.0 + next() * 8.0,
                opacity: 0.06 + next() * 0.14,
            });
        }
        stars
    })
}

pub fn stars_background() -> AnyElement {
    let amber = rgb(STAR_COLOR);

    let mut elements: Vec<AnyElement> = Vec::with_capacity(tile_stars().len() * 4);
    for star in tile_stars() {
        // Repeat the tile 2×2 so a one-tile translation wraps seamlessly.
        for (tile_x, tile_y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
            let color = Rgba {
                a: star.opacity,
                ..amber
            };
            elements.push(
                div()
                    // The field is two tiles wide/high, so a tile-local
                    // fraction is half a field fraction.
                    .absolute()
                    .left(relative((star.x + tile_x) / 2.0))
                    .top(relative((star.y + tile_y) / 2.0))
                    .child(
                        Icon::new(AppIcon::Starlight)
                            .size(px(star.size))
                            .text_color(color),
                    )
                    .into_any_element(),
            );
        }
    }

    let field = div()
        .absolute()
        .w(relative(2.0))
        .h(relative(2.0))
        .children(elements);

    div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .child(field.with_animation(
            "stars-drift",
            Animation::new(Duration::from_secs(DRIFT_SECS)).repeat(),
            // Drift diagonally by one full tile (100% of the container),
            // after which the repeated tile lines up exactly with the start.
            |field, delta| field.left(relative(-delta)).top(relative(-delta)),
        ))
        .into_any_element()
}
