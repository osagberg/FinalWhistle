//! `render_contact_sheet` — static PNG contact-sheet of a full match.
//!
//! Reads a `dump_frames` JSON file (or stdin), samples every Nth tick, and
//! renders a grid of mini-pitch thumbnails — one per sampled tick — showing:
//! - Pitch outline (green rectangle)
//! - Home players (white dots)
//! - Away players (yellow dots)
//! - Ball (red dot)
//! - Score + tick label
//!
//! The result is a single PNG that lets you scan a whole match's motion at a
//! glance. Saved to `target/contact-sheet-<seed>.png` (gitignored).
//!
//! ## Usage
//!
//! ```sh
//! # Render every 90th tick (1-minute intervals):
//! cargo run -p fw-match-sim --bin render_contact_sheet -- /tmp/dx2-frames.json
//!
//! # Custom sample interval (every 30 ticks = 0.5 min):
//! cargo run -p fw-match-sim --bin render_contact_sheet -- /tmp/dx2-frames.json --step 30
//!
//! # Custom output path:
//! cargo run -p fw-match-sim --bin render_contact_sheet -- /tmp/dx2-frames.json \
//!   --output /tmp/contact.png
//! ```
//!
//! ## Note on float arithmetic
//!
//! This binary does pixel-coordinate math using f64. It is a viewer/renderer
//! and never feeds values back into canonical state. The crate-wide
//! `float_arithmetic = "deny"` lint applies to the library, not to bins;
//! Cargo attributes the bin separately. We use a module-level allow here
//! to keep clippy happy without polluting the lib.
#![allow(clippy::float_arithmetic)]

use std::io::{self, Read};
use std::path::PathBuf;

use clap::Parser;
use fw_match_sim::MatchFrameDto;
use image::{ImageBuffer, Rgb, RgbImage};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "render_contact_sheet",
    about = "Render a grid-of-mini-pitches PNG for a dump_frames JSON file."
)]
struct Cli {
    /// Path to the dump_frames JSON file. If omitted, reads from stdin.
    input: Option<PathBuf>,

    /// Sample every N ticks (default 90 = 1 match-minute per cell).
    #[arg(long, default_value_t = 90)]
    step: usize,

    /// Number of columns in the grid (default 10).
    #[arg(long, default_value_t = 10)]
    cols: usize,

    /// Width of each mini-pitch cell in pixels (default 120).
    #[arg(long, default_value_t = 120)]
    cell_w: u32,

    /// Height of each mini-pitch cell in pixels (default 80).
    #[arg(long, default_value_t = 80)]
    cell_h: u32,

    /// Output PNG path. Defaults to `target/contact-sheet-<seed>.png`.
    #[arg(long)]
    output: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Pitch constants: imported from canonical sources.
// Q32 → f64: raw_bits / 2^32 (same formula as dto.rs::q32_to_f64).
// ---------------------------------------------------------------------------

const Q32_SCALE: f64 = 4_294_967_296.0; // 2^32

/// Pitch half-length = GOAL_LINE_X. IMPORTED: fw_core::GOAL_LINE_X.
const PITCH_HALF_LEN: f64 = fw_core::GOAL_LINE_X.to_bits() as f64 / Q32_SCALE;

/// Pitch half-width = SIDELINE_Y. IMPORTED: fw_core::SIDELINE_Y.
const PITCH_HALF_WIDTH: f64 = fw_core::SIDELINE_Y.to_bits() as f64 / Q32_SCALE;

/// Goal half-width. IMPORTED: fw_content::event::GOAL_HALF_WIDTH_M.
const GOAL_HALF_WIDTH: f64 = fw_content::event::GOAL_HALF_WIDTH_M.to_bits() as f64 / Q32_SCALE;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

const BG_COLOR: Rgb<u8> = Rgb([30u8, 30u8, 30u8]); // Dark background between cells
const PITCH_COLOR: Rgb<u8> = Rgb([34u8, 100u8, 34u8]); // Green pitch
const LINE_COLOR: Rgb<u8> = Rgb([200u8, 200u8, 200u8]); // Pitch lines
const HOME_COLOR: Rgb<u8> = Rgb([255u8, 255u8, 255u8]); // White for home
const AWAY_COLOR: Rgb<u8> = Rgb([255u8, 220u8, 50u8]); // Yellow for away
const BALL_COLOR: Rgb<u8> = Rgb([255u8, 60u8, 60u8]); // Red for ball
const LABEL_COLOR: Rgb<u8> = Rgb([220u8, 220u8, 220u8]); // Light grey for labels

// ---------------------------------------------------------------------------
// Coordinate mapping
// ---------------------------------------------------------------------------

/// Map a world-coordinate point (pitch-centered, metres) to pixel coordinates
/// within a cell. Returns (px, py) in [0, cell_w) × [0, cell_h).
fn world_to_pixel(wx: f64, wy: f64, cell_w: u32, cell_h: u32, margin: u32) -> (u32, u32) {
    let inner_w = cell_w.saturating_sub(2 * margin) as f64;
    let inner_h = cell_h.saturating_sub(2 * margin) as f64;
    // wx in [-52.5, 52.5] → [0, inner_w], wy in [-34, 34] → [0, inner_h].
    // Note: pitch is horizontal (long axis = X), cell is landscape.
    let norm_x = (wx + PITCH_HALF_LEN) / (2.0 * PITCH_HALF_LEN);
    let norm_y = (wy + PITCH_HALF_WIDTH) / (2.0 * PITCH_HALF_WIDTH);
    let px = (margin as f64 + norm_x * inner_w).round() as u32;
    let py = (margin as f64 + norm_y * inner_h).round() as u32;
    (px.min(cell_w - 1), py.min(cell_h - 1))
}

// ---------------------------------------------------------------------------
// Drawing primitives (integer-only after coordinate mapping)
// ---------------------------------------------------------------------------

fn fill_rect(img: &mut RgbImage, x0: u32, y0: u32, x1: u32, y1: u32, color: Rgb<u8>) {
    for y in y0..=y1.min(img.height() - 1) {
        for x in x0..=x1.min(img.width() - 1) {
            img.put_pixel(x, y, color);
        }
    }
}

fn draw_hline(img: &mut RgbImage, x0: u32, x1: u32, y: u32, color: Rgb<u8>) {
    if y >= img.height() {
        return;
    }
    for x in x0..=x1.min(img.width() - 1) {
        img.put_pixel(x, y, color);
    }
}

fn draw_vline(img: &mut RgbImage, x: u32, y0: u32, y1: u32, color: Rgb<u8>) {
    if x >= img.width() {
        return;
    }
    for y in y0..=y1.min(img.height() - 1) {
        img.put_pixel(x, y, color);
    }
}

/// Draw a filled circle (dot) with the given radius.
fn draw_dot(img: &mut RgbImage, cx: u32, cy: u32, r: u32, color: Rgb<u8>) {
    let r2 = (r * r) as i64;
    let cx_i = cx as i64;
    let cy_i = cy as i64;
    let x0 = cx.saturating_sub(r);
    let x1 = (cx + r).min(img.width() - 1);
    let y0 = cy.saturating_sub(r);
    let y1 = (cy + r).min(img.height() - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as i64 - cx_i;
            let dy = y as i64 - cy_i;
            if dx * dx + dy * dy <= r2 {
                img.put_pixel(x, y, color);
            }
        }
    }
}

/// Write a small rasterised label (seed-hex style digits) using a fixed 5×7
/// pixel font baked into a const. Only supports 0-9, '-', 'm', 'i', 'n', ':',
/// space, 'G', '/', '@'. Enough to render "min NN G:H-A".
fn draw_label(img: &mut RgbImage, ox: u32, oy: u32, text: &str, color: Rgb<u8>) {
    // Minimal 5×7 bitmap font for digits 0-9 and a few extra glyphs.
    // Each glyph is 5 columns wide, 7 rows tall, stored as 7 u8 bitmasks (MSB=col0).
    let glyph = |ch: char| -> [u8; 7] {
        match ch {
            '0' => [
                0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
            ],
            '1' => [
                0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ],
            '2' => [
                0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
            ],
            '3' => [
                0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
            '4' => [
                0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
            ],
            '5' => [
                0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
            ],
            '6' => [
                0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
            ],
            '7' => [
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
            ],
            '8' => [
                0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
            ],
            '9' => [
                0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
            ],
            '-' => [
                0b00000, 0b00000, 0b00000, 0b01110, 0b00000, 0b00000, 0b00000,
            ],
            ':' => [
                0b00000, 0b00100, 0b00000, 0b00000, 0b00100, 0b00000, 0b00000,
            ],
            ' ' => [0b00000; 7],
            _ => [
                0b10001, 0b01010, 0b00100, 0b00100, 0b01010, 0b10001, 0b00000,
            ], // X (fallback)
        }
    };

    let mut x_cursor = ox;
    for ch in text.chars() {
        let bits = glyph(ch);
        for (row, &mask) in bits.iter().enumerate() {
            let py = oy + row as u32;
            if py >= img.height() {
                break;
            }
            for col in 0..5u32 {
                let px = x_cursor + col;
                if px >= img.width() {
                    break;
                }
                if mask & (1 << (4 - col)) != 0 {
                    img.put_pixel(px, py, color);
                }
            }
        }
        x_cursor += 6; // 5px glyph + 1px gap
        if x_cursor >= img.width() {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Cell rendering
// ---------------------------------------------------------------------------

fn render_cell(
    img: &mut RgbImage,
    frame: &MatchFrameDto,
    cell_x: u32,
    cell_y: u32,
    cell_w: u32,
    cell_h: u32,
) {
    let margin = 4u32;
    let ox = cell_x;
    let oy = cell_y;

    // Pitch background.
    fill_rect(img, ox, oy, ox + cell_w - 1, oy + cell_h - 1, PITCH_COLOR);

    // Pitch outline.
    let (pl, pt) = (ox + margin, oy + margin);
    let (pr, pb) = (ox + cell_w - 1 - margin, oy + cell_h - 1 - margin - 8); // leave 8px for label
    draw_hline(img, pl, pr, pt, LINE_COLOR);
    draw_hline(img, pl, pr, pb, LINE_COLOR);
    draw_vline(img, pl, pt, pb, LINE_COLOR);
    draw_vline(img, pr, pt, pb, LINE_COLOR);

    // Halfway line.
    let mid_x = (pl + pr) / 2;
    draw_vline(img, mid_x, pt, pb, LINE_COLOR);

    // Goal boxes (simplified: just the goal-line extent).
    // Left goal (home GK's end, x = -52.5).
    let (gl_px, _) = world_to_pixel(
        -PITCH_HALF_LEN,
        -GOAL_HALF_WIDTH,
        cell_w,
        cell_h - 8,
        margin,
    );
    let (_, gl_py0) = world_to_pixel(0.0, -GOAL_HALF_WIDTH, cell_w, cell_h - 8, margin);
    let (_, gl_py1) = world_to_pixel(0.0, GOAL_HALF_WIDTH, cell_w, cell_h - 8, margin);
    let gl_px = gl_px + ox;
    let gl_py0 = gl_py0 + oy;
    let gl_py1 = gl_py1 + oy;
    draw_vline(img, gl_px, gl_py0, gl_py1, Rgb([255, 255, 255]));

    // Right goal (away GK's end, x = +52.5).
    let (gr_px, _) = world_to_pixel(PITCH_HALF_LEN, -GOAL_HALF_WIDTH, cell_w, cell_h - 8, margin);
    let gr_px = gr_px + ox;
    draw_vline(img, gr_px, gl_py0, gl_py1, Rgb([255, 255, 255]));

    // Players: slots 0-10 = home, 11-21 = away.
    for player in &frame.players {
        let is_home = (player.slot as usize) < 11;
        let color = if is_home { HOME_COLOR } else { AWAY_COLOR };
        let (px, py) = world_to_pixel(player.pos_x, player.pos_y, cell_w, cell_h - 8, margin);
        draw_dot(img, ox + px, oy + py, 2, color);
    }

    // Ball.
    let (bx, by) = world_to_pixel(
        frame.ball.pos_x,
        frame.ball.pos_y,
        cell_w,
        cell_h - 8,
        margin,
    );
    draw_dot(img, ox + bx, oy + by, 2, BALL_COLOR);

    // Label: "{minute:02} {home}-{away}" at bottom of cell.
    // Digit-only format: bitmap font only has reliable glyphs for 0-9 and '-'.
    let minute = frame.tick / 60;
    let label = format!("{minute:02} {}-{}", frame.home_score, frame.away_score);
    let label_y = oy + cell_h - 8;
    draw_label(img, ox + margin, label_y, &label, LABEL_COLOR);
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(path) => {
            eprintln!("contact-sheet written: {path}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("render_contact_sheet: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<String, String> {
    let json_str = if let Some(path) = &cli.input {
        std::fs::read_to_string(path).map_err(|e| format!("failed to read {:?}: {e}", path))?
    } else {
        let mut s = String::new();
        io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        s
    };

    let frames: Vec<MatchFrameDto> =
        serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {e}"))?;

    if frames.is_empty() {
        return Err("empty frame list".to_string());
    }

    let seed_hex = &frames[0].seed_hex;
    let step = cli.step.max(1);
    let cols = cli.cols.max(1);
    let cell_w = cli.cell_w.max(60);
    let cell_h = cli.cell_h.max(48);

    // Sample frames.
    let sampled: Vec<&MatchFrameDto> = frames.iter().step_by(step).collect();
    let n_cells = sampled.len();
    let rows = n_cells.div_ceil(cols);

    let img_w = cols as u32 * cell_w;
    let img_h = rows as u32 * cell_h;

    let mut img: RgbImage = ImageBuffer::from_pixel(img_w, img_h, BG_COLOR);

    for (idx, frame) in sampled.iter().enumerate() {
        let col = (idx % cols) as u32;
        let row = (idx / cols) as u32;
        render_cell(&mut img, frame, col * cell_w, row * cell_h, cell_w, cell_h);
    }

    // Output path.
    let out_path = if let Some(p) = &cli.output {
        p.clone()
    } else {
        // Default to target/contact-sheet-<seed>.png.
        // Walk up to workspace root (the bin is run from workspace root normally).
        let target_dir = PathBuf::from("target");
        let _ = std::fs::create_dir_all(&target_dir);
        // Sanitise seed_hex for use as filename.
        let safe_seed = seed_hex.replace("0x", "").replace(['/', '\\', ':'], "_");
        target_dir.join(format!("contact-sheet-{safe_seed}.png"))
    };

    img.save(&out_path)
        .map_err(|e| format!("failed to write PNG {:?}: {e}", out_path))?;

    Ok(out_path.to_string_lossy().into_owned())
}
