use ab_glyph::{FontRef, PxScale};
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;

use crate::Error;

static FONT_BYTES: &[u8] = include_bytes!("../assets/font.ttf");

pub struct LeaderboardRow {
    pub rank: u32,
    pub team_name: String,
    pub record: String,
    pub owner_display: String,
    pub avatar_bytes: Option<Vec<u8>>,
}
pub async fn fetch_avatar_bytes(url: &str) -> Option<Vec<u8>> {
    let response = reqwest::get(url).await.ok()?;
    response.bytes().await.ok().map(|b| b.to_vec())
}

/// Crops and resizes an avatar into a circular thumbnail of `size`x`size`.
fn make_circular_avatar(bytes: &[u8], size: u32) -> Option<RgbaImage> {
    let img = image::load_from_memory(bytes).ok()?;
    let resized = img.resize_to_fill(size, size, image::imageops::FilterType::Lanczos3);
    let mut rgba = resized.to_rgba8();

    let radius = size as f32 / 2.0;
    let center = (radius, radius);

    for (x, y, pixel) in rgba.enumerate_pixels_mut() {
        let dx = x as f32 - center.0;
        let dy = y as f32 - center.1;
        if (dx * dx + dy * dy).sqrt() > radius {
            pixel[3] = 0; // outside the circle: fully transparent
        }
    }

    Some(rgba)
}

pub fn generate_leaderboard_image(rows: &[LeaderboardRow]) -> Result<Vec<u8>, Error> {
    let scale_factor = 2u32;

    let width = 800u32 * scale_factor;
    let row_height = 90u32 * scale_factor;
    let height = row_height * rows.len() as u32; // no top or bottom padding

    let mut canvas: RgbaImage = ImageBuffer::from_pixel(width, height, Rgba([30, 33, 36, 255]));
    let font = FontRef::try_from_slice(FONT_BYTES)?;

    for (i, row) in rows.iter().enumerate() {
        let y = i as u32 * row_height; // rows start at 0, no top_padding offset

        let bg = if i % 2 == 0 {
            Rgba([40, 43, 47, 255])
        } else {
            Rgba([30, 33, 36, 255])
        };
        draw_filled_rect_mut(
            &mut canvas,
            Rect::at(0, y as i32).of_size(width, row_height),
            bg,
        );

        draw_text_mut(
            &mut canvas,
            Rgba([200, 200, 200, 255]),
            (24 * scale_factor) as i32,
            y as i32 + (24 * scale_factor) as i32,
            PxScale::from(32.0 * scale_factor as f32),
            &font,
            &format!("#{}", row.rank),
        );

        let avatar_size = 56 * scale_factor;
        let avatar_x = 100 * scale_factor;
        let avatar_y = y as i32 + (row_height as i32 - avatar_size as i32) / 2;

        if let Some(bytes) = &row.avatar_bytes {
            if let Some(avatar) = make_circular_avatar(bytes, avatar_size) {
                image::imageops::overlay(&mut canvas, &avatar, avatar_x as i64, avatar_y as i64);
            }
        } else {
            draw_filled_circle_mut(
                &mut canvas,
                (
                    avatar_x as i32 + avatar_size as i32 / 2,
                    avatar_y + avatar_size as i32 / 2,
                ),
                avatar_size as i32 / 2,
                Rgba([80, 80, 80, 255]),
            );
        }

        draw_text_mut(
            &mut canvas,
            Rgba([255, 255, 255, 255]),
            (180 * scale_factor) as i32,
            y as i32 + (16 * scale_factor) as i32,
            PxScale::from(28.0 * scale_factor as f32),
            &font,
            &row.team_name,
        );
        draw_text_mut(
            &mut canvas,
            Rgba([170, 170, 170, 255]),
            (180 * scale_factor) as i32,
            y as i32 + (50 * scale_factor) as i32,
            PxScale::from(22.0 * scale_factor as f32),
            &font,
            &row.owner_display,
        );

        draw_text_mut(
            &mut canvas,
            Rgba([87, 242, 135, 255]),
            width as i32 - (160 * scale_factor) as i32,
            y as i32 + (28 * scale_factor) as i32,
            PxScale::from(30.0 * scale_factor as f32),
            &font,
            &row.record,
        );
    }

    let final_width = width / scale_factor;
    let final_height = height / scale_factor;
    let resized = image::imageops::resize(
        &canvas,
        final_width,
        final_height,
        image::imageops::FilterType::Lanczos3,
    );

    let mut buffer = Vec::new();
    DynamicImage::ImageRgba8(resized).write_to(
        &mut std::io::Cursor::new(&mut buffer),
        image::ImageFormat::Png,
    )?;
    Ok(buffer)
}
