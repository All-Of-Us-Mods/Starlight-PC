//! Decorative drifting starfield, modeled on upstream's StarBackground:
//! stars distributed by Poisson-disc-style sampling (minimum spacing, so
//! they never overlap or clump), tiled 2×2 and translated by exactly one
//! tile per cycle so the drift loops seamlessly. Star color is fixed amber
//! (upstream's `#fbbf24`) regardless of theme; toggled by the "Floating
//! stars background" setting.
//!
//! Perf notes: the whole field is ONE `canvas` element that paints every
//! star directly via `paint_svg` — no per-star elements, so the per-frame
//! layout cost is a single node and the paint cost is a few hundred quads
//! reusing cached SVG rasters (sizes are quantized to a few buckets so the
//! rasterizer cache is shared). The dominant per-frame cost turns out to be
//! the window redraw itself, so motion is stepped by a 30fps timer instead
//! of per-vsync animation — at ~11px/s drift that's sub-pixel per step
//! (indistinguishable from 60fps) for half the redraws. The offset derives
//! from wall-clock elapsed time, so tick jitter never accumulates drift
//! error.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui::*;

use crate::ui::icon::AppIcon;
use gpui_component::IconNamed;

/// Upstream's star color (Tailwind amber-400).
const STAR_COLOR: u32 = 0xfbbf24;
/// Minimum spacing between star centers, as a fraction of the window.
const MIN_DIST: f32 = 0.09;
const MAX_STARS: usize = 60;
/// Seconds for the field to drift one full tile.
const DRIFT_SECS: f32 = 90.0;
/// Drift update interval (~30fps). The drift is slow enough that steps are
/// sub-pixel at this rate; going faster only doubles redraw cost.
const TICK: Duration = Duration::from_millis(33);

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
            // Quantized sizes: a handful of buckets, not a continuum.
            let size = 6.0 + (next() * 5.0).floor() * 2.0;
            stars.push(Star {
                x,
                y,
                size,
                opacity: 0.06 + next() * 0.14,
            });
        }
        stars
    })
}

pub struct StarsBackground {
    start: Instant,
}

impl StarsBackground {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            start: Instant::now(),
        }
    }
}

impl Render for StarsBackground {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Drift offset in tile fractions; one full tile per cycle, after
        // which the repeated tiles line up exactly with the start.
        let offset = (self.start.elapsed().as_secs_f32() / DRIFT_SECS).fract();

        let amber: Hsla = rgb(STAR_COLOR).into();

        canvas(
            |_, _, _| {},
            move |bounds, _, window, cx| {
                let path = AppIcon::Starlight.path();
                for star in tile_stars() {
                    for (tile_x, tile_y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                        let star_size = px(star.size);
                        let x = bounds.size.width * (star.x + tile_x - offset);
                        let y = bounds.size.height * (star.y + tile_y - offset);
                        // Cull stars outside the visible tile window.
                        if x < -star_size
                            || x > bounds.size.width
                            || y < -star_size
                            || y > bounds.size.height
                        {
                            continue;
                        }
                        let star_bounds =
                            Bounds::new(bounds.origin + point(x, y), size(star_size, star_size));
                        let color = Hsla {
                            a: star.opacity,
                            ..amber
                        };
                        let _ = window.paint_svg(
                            star_bounds,
                            path.clone(),
                            None,
                            TransformationMatrix::default(),
                            color,
                            cx,
                        );
                    }
                }
            },
        )
        .absolute()
        .size_full()
    }
}
