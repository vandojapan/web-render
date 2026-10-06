//! Checked image boundaries shared by the IPC client and the CEF capture process.
pub const SURFACE_WIDTH: usize = 4096;
pub const SURFACE_HEIGHT: usize = 2304;
pub const METADATA_ROWS: usize = 64;

#[derive(Debug, thiserror::Error)]
#[error("Invalid RGBA image: {0}")]
pub struct ImageError(pub &'static str);

pub fn rgba_len(width: usize, height: usize) -> Result<usize, ImageError> {
    if width == 0 || height == 0 || width > SURFACE_WIDTH || height > SURFACE_HEIGHT {
        return Err(ImageError("dimensions are outside the capture surface"));
    }
    width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .ok_or(ImageError("byte length overflow"))
}

pub fn validate_rgba(width: usize, height: usize, length: usize) -> Result<(), ImageError> {
    if length != rgba_len(width, height)? {
        return Err(ImageError("buffer length does not match dimensions"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum PaintFormat {
    Rgba,
    PremultipliedBgra,
}

/// Borrowed CEF/GPU paint. It must not outlive the native paint callback.
/// Decode the small metadata and convert only the requested rectangles.
#[derive(Debug, Clone, Copy)]
pub struct PaintFrame<'a> {
    buffer: &'a [u8],
    width: usize,
    height: usize,
    stride: usize,
    format: PaintFormat,
}

fn straight_component(value: u8, alpha: u8) -> u8 {
    match alpha {
        0 => 0,
        255 => value,
        _ => ((u32::from(value) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255) as u8,
    }
}

impl<'a> PaintFrame<'a> {
    pub fn new(
        buffer: &'a [u8],
        width: usize,
        height: usize,
        stride: usize,
        format: PaintFormat,
    ) -> Result<Self, ImageError> {
        rgba_len(width, height)?;
        let row_bytes = width * 4;
        let required = stride
            .checked_mul(height - 1)
            .and_then(|n| n.checked_add(row_bytes))
            .ok_or(ImageError("stride overflow"))?;
        if stride < row_bytes || buffer.len() < required {
            return Err(ImageError("invalid stride or truncated paint buffer"));
        }
        Ok(Self {
            buffer,
            width,
            height,
            stride,
            format,
        })
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn rgb_byte(&self, index: usize) -> u8 {
        let pixel = index / 3;
        let offset = pixel / self.width * self.stride + pixel % self.width * 4;
        let channel = index % 3;
        match self.format {
            PaintFormat::Rgba => self.buffer[offset + channel],
            PaintFormat::PremultipliedBgra => {
                straight_component(self.buffer[offset + 2 - channel], self.buffer[offset + 3])
            }
        }
    }

    pub fn metadata_nonce(&self) -> Result<u32, ImageError> {
        if self.width * self.height.min(METADATA_ROWS) < 4
            || self.buffer[3] != 255
            || [self.rgb_byte(0), self.rgb_byte(1), self.rgb_byte(2)] != [255, 192, 128]
        {
            return Err(ImageError("missing metadata header"));
        }
        Ok(u32::from_le_bytes([
            self.rgb_byte(3),
            self.rgb_byte(4),
            self.rgb_byte(5),
            self.rgb_byte(6),
        ]))
    }

    pub fn read_metadata(&self) -> Result<Vec<u8>, ImageError> {
        self.metadata_nonce()?;
        let length = u32::from_le_bytes([
            self.rgb_byte(7),
            self.rgb_byte(8),
            self.rgb_byte(9),
            self.rgb_byte(10),
        ]) as usize;
        let capacity = self.width * self.height.min(METADATA_ROWS) * 3;
        if length > capacity.saturating_sub(11) {
            return Err(ImageError("metadata exceeds reserved rows"));
        }
        Ok((11..11 + length).map(|i| self.rgb_byte(i)).collect())
    }

    pub fn crop_rgba(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<Vec<u8>, ImageError> {
        if x < 0 || y < 0 || width <= 0 || height <= 0 {
            return Err(ImageError(
                "negative coordinates or non-positive dimensions",
            ));
        }
        let (x, y, width, height) = (x as usize, y as usize, width as usize, height as usize);
        let len = rgba_len(width, height)?;
        if x.checked_add(width).is_none_or(|n| n > self.width)
            || y.checked_add(height).is_none_or(|n| n > self.height)
        {
            return Err(ImageError("crop rectangle is outside the captured image"));
        }
        let mut result = Vec::with_capacity(len);
        for row in y..y + height {
            let start = row * self.stride + x * 4;
            let source = &self.buffer[start..start + width * 4];
            match self.format {
                PaintFormat::Rgba => result.extend_from_slice(source),
                PaintFormat::PremultipliedBgra => {
                    for pixel in source.chunks_exact(4) {
                        let alpha = pixel[3];
                        result.extend_from_slice(&[
                            straight_component(pixel[2], alpha),
                            straight_component(pixel[1], alpha),
                            straight_component(pixel[0], alpha),
                            alpha,
                        ]);
                    }
                }
            }
        }
        Ok(result)
    }
}

pub fn packed_rows(
    buffer: &[u8],
    width: usize,
    height: usize,
    stride: usize,
) -> Result<Vec<u8>, ImageError> {
    let len = rgba_len(width, height)?;
    let row_bytes = width
        .checked_mul(4)
        .ok_or(ImageError("row length overflow"))?;
    let required = stride
        .checked_mul(height - 1)
        .and_then(|n| n.checked_add(row_bytes))
        .ok_or(ImageError("stride overflow"))?;
    if stride < row_bytes || buffer.len() < required {
        return Err(ImageError("invalid stride or truncated paint buffer"));
    }
    let mut result = Vec::with_capacity(len);
    for row in 0..height {
        result.extend_from_slice(&buffer[row * stride..row * stride + row_bytes]);
    }
    Ok(result)
}

pub fn crop_rgba(
    buffer: &[u8],
    surface_width: usize,
    surface_height: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, ImageError> {
    validate_rgba(surface_width, surface_height, buffer.len())?;
    if x < 0 || y < 0 || width <= 0 || height <= 0 {
        return Err(ImageError(
            "negative coordinates or non-positive dimensions",
        ));
    }
    let (x, y, width, height) = (x as usize, y as usize, width as usize, height as usize);
    let len = rgba_len(width, height)?;
    if x.checked_add(width).is_none_or(|n| n > surface_width)
        || y.checked_add(height).is_none_or(|n| n > surface_height)
    {
        return Err(ImageError("crop rectangle is outside the captured image"));
    }
    let mut result = Vec::with_capacity(len);
    for row in y..y + height {
        let start = (row * surface_width + x) * 4;
        result.extend_from_slice(&buffer[start..start + width * 4]);
    }
    Ok(result)
}

pub fn bgra_to_rgba(buffer: &[u8], width: usize, height: usize) -> Result<Vec<u8>, ImageError> {
    validate_rgba(width, height, buffer.len())?;
    let mut result = Vec::with_capacity(buffer.len());
    for pixel in buffer.chunks_exact(4) {
        let alpha = u32::from(pixel[3]);
        let straight = |v: u8| {
            (u32::from(v) * 255 + alpha / 2)
                .checked_div(alpha)
                .unwrap_or(0)
                .min(255) as u8
        };
        result.extend_from_slice(&[
            straight(pixel[2]),
            straight(pixel[1]),
            straight(pixel[0]),
            pixel[3],
        ]);
    }
    Ok(result)
}

pub fn read_metadata(buffer: &[u8], width: usize, height: usize) -> Result<Vec<u8>, ImageError> {
    validate_rgba(width, height, buffer.len())?;
    let metadata = &buffer[..width * height.min(METADATA_ROWS) * 4];
    if metadata.len() < 16 || metadata[..4] != [255, 192, 128, 255] {
        return Err(ImageError("missing metadata header"));
    }
    let length =
        u32::from_le_bytes([metadata[9], metadata[10], metadata[12], metadata[13]]) as usize;
    if length > (metadata.len() / 4 * 3).saturating_sub(11) {
        return Err(ImageError("metadata exceeds reserved rows"));
    }
    Ok((0..length)
        .map(|i| {
            let rgb_index = 11 + i;
            metadata[4 * (rgb_index / 3) + rgb_index % 3]
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_dimensions_and_buffers() {
        for (w, h, n) in [
            (0, 1, 0),
            (1, 0, 0),
            (usize::MAX, 2, 0),
            (4097, 1, 0),
            (1, 1, 3),
            (1, 1, 5),
        ] {
            assert!(validate_rgba(w, h, n).is_err());
        }
        assert!(validate_rgba(3840, 2160, 3840 * 2160 * 4).is_ok());
    }
    #[test]
    fn crop_checks_every_edge_and_copies_the_requested_rows() {
        let buffer: Vec<u8> = (0..48).collect();
        assert_eq!(
            crop_rgba(&buffer, 4, 3, 1, 1, 2, 2).unwrap(),
            [&buffer[20..28], &buffer[36..44]].concat()
        );
        for (x, y, w, h) in [
            (-1, 0, 1, 1),
            (0, -1, 1, 1),
            (0, 0, 0, 1),
            (3, 2, 2, 1),
            (0, 2, 1, 2),
            (i32::MAX, 0, i32::MAX, 1),
        ] {
            assert!(crop_rgba(&buffer, 4, 3, x, y, w, h).is_err());
        }
        assert!(crop_rgba(&buffer[..47], 4, 3, 0, 0, 1, 1).is_err());
    }
    #[test]
    fn stride_padding_and_truncation() {
        let buffer: Vec<u8> = (0..20).collect();
        assert_eq!(
            packed_rows(&buffer, 2, 2, 12).unwrap(),
            [&buffer[..8], &buffer[12..20]].concat()
        );
        assert!(packed_rows(&buffer[..19], 2, 2, 12).is_err());
        assert!(packed_rows(&buffer, 2, 2, 7).is_err());
        assert!(packed_rows(&buffer, 2, 2, usize::MAX).is_err());
    }
    #[test]
    fn software_alpha_and_rgb_order() {
        assert_eq!(
            bgra_to_rgba(&[50, 25, 100, 128, 99, 99, 99, 0, 0, 0, 255, 255], 3, 1).unwrap(),
            [199, 50, 100, 128, 0, 0, 0, 0, 255, 0, 0, 255]
        );
    }
    #[test]
    fn metadata_cannot_read_image_pixels() {
        let mut buffer = vec![0; 4 * 70 * 4];
        buffer[..4].copy_from_slice(&[255, 192, 128, 255]);
        let oversized = (4 * 64 * 3 - 10) as u32;
        let [a, b, c, d] = oversized.to_le_bytes();
        buffer[9] = a;
        buffer[10] = b;
        buffer[12] = c;
        buffer[13] = d;
        assert!(read_metadata(&buffer, 4, 70).is_err());
    }

    #[test]
    fn borrowed_paint_crops_match_full_conversion_and_check_bounds() {
        // Includes every alpha and component value, not just opaque p5 shapes.
        let mut bgra = vec![];
        for alpha in 0..=255u8 {
            for value in 0..=255u8 {
                bgra.extend_from_slice(&[value, 255 - value, value / 2, alpha]);
            }
        }
        let frame = PaintFrame::new(&bgra, 256, 256, 1024, PaintFormat::PremultipliedBgra).unwrap();
        let full = bgra_to_rgba(&bgra, 256, 256).unwrap();
        for (x, y, w, h) in [(0, 0, 256, 256), (63, 128, 64, 17), (255, 255, 1, 1)] {
            assert_eq!(
                frame.crop_rgba(x, y, w, h).unwrap(),
                crop_rgba(&full, 256, 256, x, y, w, h).unwrap()
            );
        }
        for (x, y, w, h) in [
            (-1, 0, 1, 1),
            (0, -1, 1, 1),
            (0, 0, 0, 1),
            (255, 255, 2, 1),
            (0, 255, 1, 2),
            (i32::MAX, 0, i32::MAX, 1),
        ] {
            assert!(frame.crop_rgba(x, y, w, h).is_err());
        }
        assert!(
            PaintFrame::new(
                &bgra[..bgra.len() - 1],
                256,
                256,
                1024,
                PaintFormat::PremultipliedBgra
            )
            .is_err()
        );
        assert!(PaintFrame::new(&bgra, 256, 256, 1023, PaintFormat::Rgba).is_err());
        assert!(PaintFrame::new(&bgra, 256, 256, usize::MAX, PaintFormat::Rgba).is_err());
        assert!(PaintFrame::new(&[], 0, 1, 0, PaintFormat::Rgba).is_err());
    }

    #[test]
    fn borrowed_paint_reads_metadata_across_padded_rows_in_both_formats() {
        let width = 2;
        let height = 70;
        let stride = 12;
        let payload: Vec<_> = (0..200).collect();
        let nonce = 0xf123abcd_u32;
        let mut message = vec![255, 192, 128];
        message.extend_from_slice(&nonce.to_le_bytes());
        message.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        message.extend_from_slice(&payload);
        let mut rgba = vec![99; stride * height];
        for (i, value) in message.into_iter().enumerate() {
            let pixel = i / 3;
            let offset = pixel / width * stride + pixel % width * 4;
            rgba[offset + i % 3] = value;
            rgba[offset + 3] = 255;
        }
        let frame = PaintFrame::new(&rgba, width, height, stride, PaintFormat::Rgba).unwrap();
        assert_eq!(frame.metadata_nonce().unwrap(), nonce);
        assert_eq!(frame.read_metadata().unwrap(), payload);
        assert_eq!(
            frame.crop_rgba(1, 69, 1, 1).unwrap(),
            rgba[69 * stride + 4..69 * stride + 8]
        );
        let mut bgra = rgba.clone();
        for row in 0..height {
            for col in 0..width {
                bgra.swap(row * stride + col * 4, row * stride + col * 4 + 2);
            }
        }
        let bgra_frame =
            PaintFrame::new(&bgra, width, height, stride, PaintFormat::PremultipliedBgra).unwrap();
        assert_eq!(bgra_frame.metadata_nonce().unwrap(), nonce);
        assert_eq!(bgra_frame.read_metadata().unwrap(), payload);
        // The encoded length must never reach beyond the reserved metadata rows.
        rgba[stride + 4] = 255; // RGB index 9, the third length byte.
        assert!(
            PaintFrame::new(&rgba, width, height, stride, PaintFormat::Rgba)
                .unwrap()
                .read_metadata()
                .is_err()
        );
        assert!(
            PaintFrame::new(&[0; 16], 1, 4, 4, PaintFormat::Rgba)
                .unwrap()
                .metadata_nonce()
                .is_err()
        );
    }
}
