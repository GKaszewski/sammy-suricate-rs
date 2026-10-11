use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub fn to_bevy(img: &formats::image::RgbaImage, srgb: bool, repeat: bool) -> Image {
    to_bevy_filtered(img, srgb, repeat, true)
}

/// `linear = false` gives nearest-neighbour sampling (WDL `NoFilter`).
pub fn to_bevy_filtered(img: &formats::image::RgbaImage, srgb: bool, repeat: bool, linear: bool) -> Image {
    let format = if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm };
    let mut image = Image::new(
        Extent3d { width: img.width, height: img.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        img.data.clone(),
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    let mode = if repeat { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        mag_filter: if linear { ImageFilterMode::Linear } else { ImageFilterMode::Nearest },
        min_filter: if linear { ImageFilterMode::Linear } else { ImageFilterMode::Nearest },
        ..default()
    });
    image
}

/// A4 colour key: the original ran in 16-bit colour, so a pixel is transparent when it
/// becomes 0 in RGB565 (e.g. the (2,2,2) background of `boink22.pcx`).
pub fn is_key(p: &[u8]) -> bool {
    p[0] < 8 && p[1] < 4 && p[2] < 8
}
