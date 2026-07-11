//! Bounded dimension extraction for image formats accepted by Media.

use crate::error::MediaError;

pub(crate) const MAX_HEADER_BYTES: usize = 1024 * 1024;
const MAX_DIMENSION: u32 = 20_000;
const MAX_PIXELS: u64 = 40_000_000;

pub(crate) fn parse_bounded_dimensions(
    content_type: &str,
    bytes: &[u8],
) -> Result<Option<(u32, u32)>, MediaError> {
    let dimensions = match content_type {
        "image/png" => parse_png_dimensions(bytes),
        "image/gif" => parse_gif_dimensions(bytes),
        "image/jpeg" => parse_jpeg_dimensions(bytes),
        "image/webp" => parse_webp_dimensions(bytes),
        _ => Err(MediaError::BadRequest("unsupported preview image type".into())),
    }?;
    if let Some((width, height)) = dimensions {
        let pixels = u64::from(width).saturating_mul(u64::from(height));
        if width == 0
            || height == 0
            || width > MAX_DIMENSION
            || height > MAX_DIMENSION
            || pixels > MAX_PIXELS
        {
            return Err(MediaError::BadRequest("image dimensions exceed preview limits".into()));
        }
    }
    Ok(dimensions)
}

fn parse_png_dimensions(bytes: &[u8]) -> Result<Option<(u32, u32)>, MediaError> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < PNG_SIGNATURE.len() {
        return Ok(None);
    }
    if &bytes[..8] != PNG_SIGNATURE {
        return Err(MediaError::BadRequest("invalid PNG preview header".into()));
    }
    if bytes.len() < 24 {
        return Ok(None);
    }
    if bytes[8..12] != 13u32.to_be_bytes() || &bytes[12..16] != b"IHDR" {
        return Err(MediaError::BadRequest("invalid PNG preview header".into()));
    }
    Ok(Some((
        u32::from_be_bytes(
            bytes[16..20]
                .try_into()
                .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?,
        ),
        u32::from_be_bytes(
            bytes[20..24]
                .try_into()
                .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?,
        ),
    )))
}

fn parse_gif_dimensions(bytes: &[u8]) -> Result<Option<(u32, u32)>, MediaError> {
    if bytes.len() < 6 {
        return Ok(None);
    }
    if &bytes[..6] != b"GIF87a" && &bytes[..6] != b"GIF89a" {
        return Err(MediaError::BadRequest("invalid GIF preview header".into()));
    }
    if bytes.len() < 10 {
        return Ok(None);
    }
    Ok(Some((
        u16::from_le_bytes([bytes[6], bytes[7]]).into(),
        u16::from_le_bytes([bytes[8], bytes[9]]).into(),
    )))
}

fn parse_jpeg_dimensions(bytes: &[u8]) -> Result<Option<(u32, u32)>, MediaError> {
    if bytes.len() < 2 {
        return Ok(None);
    }
    if bytes[..2] != [0xff, 0xd8] {
        return Err(MediaError::BadRequest("invalid JPEG preview header".into()));
    }
    let mut cursor = 2usize;
    while cursor < bytes.len() {
        if bytes[cursor] != 0xff {
            return Err(MediaError::BadRequest("invalid JPEG preview marker".into()));
        }
        while cursor < bytes.len() && bytes[cursor] == 0xff {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            return Ok(None);
        }
        let marker = bytes[cursor];
        cursor += 1;
        if marker == 0xd9 || marker == 0xda {
            return Err(MediaError::BadRequest("JPEG dimensions are missing".into()));
        }
        if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
            continue;
        }
        if cursor + 2 > bytes.len() {
            return Ok(None);
        }
        let segment_length = usize::from(u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]));
        if segment_length < 2 {
            return Err(MediaError::BadRequest("invalid JPEG segment length".into()));
        }
        let is_start_of_frame = matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        );
        if is_start_of_frame {
            if segment_length < 7 {
                return Err(MediaError::BadRequest("invalid JPEG frame header".into()));
            }
            if cursor + 7 > bytes.len() {
                return Ok(None);
            }
            return Ok(Some((
                u16::from_be_bytes([bytes[cursor + 5], bytes[cursor + 6]]).into(),
                u16::from_be_bytes([bytes[cursor + 3], bytes[cursor + 4]]).into(),
            )));
        }
        let next_cursor = cursor.saturating_add(segment_length);
        if next_cursor > bytes.len() {
            return Ok(None);
        }
        cursor = next_cursor;
    }
    Ok(None)
}

fn parse_webp_dimensions(bytes: &[u8]) -> Result<Option<(u32, u32)>, MediaError> {
    if bytes.len() < 12 {
        return Ok(None);
    }
    if &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err(MediaError::BadRequest("invalid WebP preview header".into()));
    }
    if bytes.len() < 20 {
        return Ok(None);
    }
    let chunk_length = u32::from_le_bytes(
        bytes[16..20]
            .try_into()
            .map_err(|error| MediaError::Internal(anyhow::Error::new(error)))?,
    );
    match &bytes[12..16] {
        b"VP8X" => {
            if chunk_length < 10 {
                return Err(MediaError::BadRequest("invalid extended WebP header".into()));
            }
            if bytes.len() < 30 {
                return Ok(None);
            }
            let width = 1
                + u32::from(bytes[24])
                + (u32::from(bytes[25]) << 8)
                + (u32::from(bytes[26]) << 16);
            let height = 1
                + u32::from(bytes[27])
                + (u32::from(bytes[28]) << 8)
                + (u32::from(bytes[29]) << 16);
            Ok(Some((width, height)))
        }
        b"VP8L" => {
            if chunk_length < 5 {
                return Err(MediaError::BadRequest("invalid lossless WebP header".into()));
            }
            if bytes.len() < 25 {
                return Ok(None);
            }
            if bytes[20] != 0x2f {
                return Err(MediaError::BadRequest("invalid lossless WebP header".into()));
            }
            let width = 1 + u32::from(bytes[21]) + ((u32::from(bytes[22]) & 0x3f) << 8);
            let height = 1
                + (u32::from(bytes[22]) >> 6)
                + (u32::from(bytes[23]) << 2)
                + ((u32::from(bytes[24]) & 0x0f) << 10);
            Ok(Some((width, height)))
        }
        b"VP8 " => {
            if chunk_length < 10 {
                return Err(MediaError::BadRequest("invalid lossy WebP header".into()));
            }
            if bytes.len() < 30 {
                return Ok(None);
            }
            if bytes[23..26] != [0x9d, 0x01, 0x2a] {
                return Err(MediaError::BadRequest("invalid lossy WebP header".into()));
            }
            let width = u32::from(u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3fff);
            let height = u32::from(u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3fff);
            Ok(Some((width, height)))
        }
        _ => Err(MediaError::BadRequest("unsupported WebP preview header".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_bounded_dimensions;

    #[test]
    fn extracts_bounded_dimensions_for_every_allowed_image_type() {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[8..12].copy_from_slice(&13u32.to_be_bytes());
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&1200u32.to_be_bytes());
        png[20..24].copy_from_slice(&800u32.to_be_bytes());
        assert_eq!(parse_bounded_dimensions("image/png", &png).expect("png"), Some((1200, 800)));

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&1200u16.to_le_bytes());
        gif.extend_from_slice(&800u16.to_le_bytes());
        assert_eq!(parse_bounded_dimensions("image/gif", &gif).expect("gif"), Some((1200, 800)));

        let jpeg = [0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x03, 0x20, 0x04, 0xb0];
        assert_eq!(parse_bounded_dimensions("image/jpeg", &jpeg).expect("jpeg"), Some((1200, 800)));

        let mut webp = vec![0; 30];
        webp[..4].copy_from_slice(b"RIFF");
        webp[8..12].copy_from_slice(b"WEBP");
        webp[12..16].copy_from_slice(b"VP8X");
        webp[16..20].copy_from_slice(&10u32.to_le_bytes());
        webp[24..27].copy_from_slice(&[0xaf, 0x04, 0x00]);
        webp[27..30].copy_from_slice(&[0x1f, 0x03, 0x00]);
        assert_eq!(parse_bounded_dimensions("image/webp", &webp).expect("webp"), Some((1200, 800)));
        assert!(parse_bounded_dimensions("image/png", &png[..12]).expect("partial").is_none());
    }

    #[test]
    fn rejects_pixel_bombs_after_parsing_the_header() {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[8..12].copy_from_slice(&13u32.to_be_bytes());
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&10_000u32.to_be_bytes());
        png[20..24].copy_from_slice(&10_000u32.to_be_bytes());
        assert!(parse_bounded_dimensions("image/png", &png).is_err());
    }
}
