use crate::protocol::libserver::{BatchRenderResponse, render_response::Response};

/// Conservative, bounded-cost sampling avoids gzip work for small or detailed images.
/// Gzip is negotiated by tonic, so clients without support still receive plain protobuf.
pub fn should_compress(response: &BatchRenderResponse) -> bool {
    let images: Vec<_> = response
        .render_responses
        .iter()
        .filter_map(|r| match &r.response {
            Some(Response::Success(image)) => Some(image.image_data.as_slice()),
            _ => None,
        })
        .collect();
    if images
        .iter()
        .fold(0usize, |total, image| total.saturating_add(image.len()))
        < 64 * 1024
    {
        return false;
    }
    images.iter().all(|image| {
        let pixels = image.len() / 4;
        if pixels < 2 {
            return false;
        }
        let samples = (pixels - 1).min(256);
        let repeated = (0..samples)
            .filter(|i| {
                let offset = i * (pixels - 1) / samples * 4;
                image[offset..offset + 4] == image[offset + 4..offset + 8]
            })
            .count();
        repeated * 4 >= samples * 3
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::libserver::{RenderResponse, SuccessRenderResponse};

    fn batch(images: Vec<Vec<u8>>) -> BatchRenderResponse {
        BatchRenderResponse {
            render_responses: images
                .into_iter()
                .enumerate()
                .map(|(i, image_data)| RenderResponse {
                    render_nonce: i as i32 + 1,
                    response: Some(Response::Success(SuccessRenderResponse {
                        width: 256,
                        height: (image_data.len() / 1024) as i32,
                        image_data,
                    })),
                })
                .collect(),
        }
    }

    #[test]
    fn compresses_large_flat_images_but_skips_small_detailed_and_mixed_images() {
        assert!(!should_compress(&batch(vec![])));
        assert!(!should_compress(&batch(vec![vec![0; 64 * 184 * 4]])));
        assert!(should_compress(&batch(vec![vec![0; 640 * 144 * 4]])));
        assert!(should_compress(&batch(vec![
            [25, 60, 100, 255].repeat(640 * 144)
        ])));
        let detail: Vec<u8> = (0..640_u32 * 144).flat_map(|i| i.to_le_bytes()).collect();
        assert!(!should_compress(&batch(vec![detail.clone()])));
        assert!(!should_compress(&batch(vec![
            vec![0; 640 * 144 * 4],
            detail
        ])));
    }
}
