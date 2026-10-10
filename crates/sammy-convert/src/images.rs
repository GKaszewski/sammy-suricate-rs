use std::path::Path;

use anyhow::Context;
use formats::image::RgbaImage;

pub fn run(out: &Path, files: &[std::path::PathBuf]) -> anyhow::Result<()> {
    std::fs::create_dir_all(out)?;
    for f in files {
        let data = std::fs::read(f)?;
        let pcx = formats::pcx::parse(&data).with_context(|| f.display().to_string())?;
        let name = f.file_stem().unwrap().to_string_lossy();
        write_png(&out.join(format!("{name}.png")), &pcx.image)?;
    }
    println!("converted {} images", files.len());
    Ok(())
}

pub fn write_png(path: &Path, img: &RgbaImage) -> anyhow::Result<()> {
    let w = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(w, img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&img.data)?;
    Ok(())
}
