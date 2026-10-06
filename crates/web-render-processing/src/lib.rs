//! Small, Bevy-independent contract between the AviUtl2 module and its native worker.
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MANIFEST: &str = "web-render.processing.json";
pub const VERSION: u32 = 1;
pub const MAX_PIXELS: usize = 3840 * 2160;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub objects: Vec<Object>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Object {
    pub id: String,
    pub label: String,
    pub scene: Scene,
    #[serde(default)]
    pub no_smooth: bool,
    #[serde(default = "default_count")]
    pub count: u32,
    #[serde(default)]
    pub animated: bool,
    #[serde(default)]
    pub seed: u32,
}
fn default_count() -> u32 {
    1000
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scene {
    Shapes,
    Text,
    Image,
    Alpha,
}

impl Project {
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let value: Self = serde_json::from_slice(&std::fs::read(root.join(MANIFEST))?)?;
        anyhow::ensure!(
            value.version == VERSION,
            "Unsupported native project version"
        );
        // Names become script directory/file names. Reject traversal and Lua header injection.
        fn safe_name(s: &str) -> bool {
            !s.is_empty()
                && s.len() <= 160
                && !s.contains(['/', '\\', ':', '\n', '\r', '\0'])
                && s != "."
                && s != ".."
                && !s.ends_with(['.', ' '])
                && !s.contains(['<', '>', '"', '|', '?', '*'])
        }
        anyhow::ensure!(safe_name(&value.name), "Invalid native project name");
        anyhow::ensure!(
            !value.objects.is_empty() && value.objects.len() <= 32,
            "Invalid object count"
        );
        let mut ids = std::collections::HashSet::new();
        let mut labels = std::collections::HashSet::new();
        for object in &value.objects {
            anyhow::ensure!(
                safe_name(&object.id) && safe_name(&object.label),
                "Invalid object name"
            );
            anyhow::ensure!(
                ids.insert(&object.id) && labels.insert(object.label.to_lowercase()),
                "Duplicate native object"
            );
            anyhow::ensure!(object.count <= 5000, "Native count exceeds 5000");
        }
        Ok(value)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Request {
    pub version: u32,
    pub nonce: i32,
    pub object: String,
    pub object_id: i64,
    pub width: u32,
    pub height: u32,
    pub time: f64,
    pub count: u32,
}
impl Request {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.version == VERSION && self.nonce > 0,
            "Invalid native request version/nonce"
        );
        anyhow::ensure!(
            self.width > 0
                && self.height > 0
                && self.width <= 3840
                && self.height <= 2160
                && (self.width as usize) * (self.height as usize) <= MAX_PIXELS,
            "Native size exceeds 3840x2160"
        );
        anyhow::ensure!(
            self.time.is_finite() && self.count <= 5000,
            "Invalid native time/count"
        );
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    pub version: u32,
    pub nonce: i32,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
    pub render_ms: f64,
    pub created_graphics: bool,
    pub error: Option<String>,
}

/// Standard Bevy alpha blend stores linear-premultiplied RGB in an sRGB texture.
/// AviUtl2 putpixeldata("rgba") expects straight, sRGB-encoded RGB instead.
pub fn straight_rgba(bytes: &mut [u8]) {
    // 64 KiB covers every byte RGB/alpha combination. Avoid millions of powf
    // evaluations per transparent 1080p frame while preserving the sRGB contract.
    static TABLE: std::sync::LazyLock<Box<[u8; 65536]>> = std::sync::LazyLock::new(|| {
        let linear: [f32; 256] = std::array::from_fn(|i| {
            let s = i as f32 / 255.;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        });
        let mut table = Box::new([0u8; 65536]);
        for a in 1..256 {
            for c in 0..256 {
                let l = (linear[c] * 255. / a as f32).min(1.);
                let s = if l <= 0.0031308 {
                    l * 12.92
                } else {
                    1.055 * l.powf(1. / 2.4) - 0.055
                };
                table[a * 256 + c] = (s * 255.).round().clamp(0., 255.) as u8;
            }
        }
        table
    });
    for pixel in bytes.chunks_exact_mut(4) {
        match pixel[3] {
            255 => {}
            0 => pixel[..3].fill(0),
            a => {
                let row = a as usize * 256;
                for channel in &mut pixel[..3] {
                    *channel = TABLE[row + *channel as usize];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linear_premultiplied_srgb_converts_to_straight() {
        let mut bytes = [
            137, 137, 137, 64, 187, 187, 187, 127, 7, 6, 5, 0, 2, 3, 4, 255,
        ];
        straight_rgba(&mut bytes);
        assert_eq!(
            bytes,
            [
                255, 255, 255, 64, 255, 255, 255, 127, 0, 0, 0, 0, 2, 3, 4, 255
            ]
        );
    }
    #[test]
    fn rejects_oversized_and_nonfinite_requests() {
        let mut r = Request {
            version: VERSION,
            nonce: 1,
            object: "x".into(),
            object_id: 1,
            width: 1920,
            height: 1080,
            time: 0.,
            count: 1000,
        };
        assert!(r.validate().is_ok());
        r.width = u32::MAX;
        assert!(r.validate().is_err());
        r.width = 1920;
        r.time = f64::NAN;
        assert!(r.validate().is_err());
    }
}
