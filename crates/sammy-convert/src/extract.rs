use std::path::Path;

use anyhow::Context;

pub fn run(wrs: &Path, out: &Path) -> anyhow::Result<()> {
    let data = std::fs::read(wrs).with_context(|| format!("reading {}", wrs.display()))?;
    let entries = formats::wrs::parse(&data)?;
    std::fs::create_dir_all(out)?;
    for e in &entries {
        std::fs::write(out.join(&e.name), e.data())?;
    }
    println!("extracted {} files to {}", entries.len(), out.display());
    Ok(())
}
