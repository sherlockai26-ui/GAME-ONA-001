use crate::font8x8::FONT;
use pixels::Pixels;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;

pub fn draw_char(pixels: &mut Pixels, x: usize, y: usize, ch: char, color: [u8; 3]) {
    let idx = ch as usize;
    if !(32..=126).contains(&idx) {
        return;
    }
    let font_idx = idx - 32;
    let bytes = &FONT[font_idx];
    let frame = pixels.frame_mut();
    for (row, byte) in bytes.iter().enumerate().take(8) {
        let mut bits = *byte;
        for col in 0..8 {
            if bits & 0x80 != 0 {
                let px = x + col;
                let py = y + row;
                if px < WIDTH && py < HEIGHT {
                    let pixel_idx = (py * WIDTH + px) * 4;
                    if pixel_idx + 3 < frame.len() {
                        frame[pixel_idx] = color[0];
                        frame[pixel_idx + 1] = color[1];
                        frame[pixel_idx + 2] = color[2];
                        frame[pixel_idx + 3] = 255;
                    }
                }
            }
            bits <<= 1;
        }
    }
}

pub fn draw_string(pixels: &mut Pixels, x: usize, y: usize, text: &str, color: [u8; 3]) {
    let mut cx = x;
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        draw_char(pixels, cx, y, ch, color);
        cx += 9;
    }
}

pub fn draw_rect(pixels: &mut Pixels, x: usize, y: usize, w: usize, h: usize, color: [u8; 3]) {
    let frame = pixels.frame_mut();
    for py in y..(y + h) {
        for px in x..(x + w) {
            if px < WIDTH && py < HEIGHT {
                let idx = (py * WIDTH + px) * 4;
                if idx + 3 < frame.len() {
                    frame[idx] = color[0];
                    frame[idx + 1] = color[1];
                    frame[idx + 2] = color[2];
                    frame[idx + 3] = 255;
                }
            }
        }
    }
}
