//! The sound effects declared in the original `sons.wdl` (`Sound Name, <file.wav>;`),
//! played by WDL name, and a decoder for their WAV files (PCM).

use crate::vfs::GameFiles;

/// (lower-case WDL name, file) for every sound `sons.wdl` declares.
pub fn table(files: &GameFiles) -> Vec<(String, String)> {
    let Ok(script) = files.read("sons.wdl") else { return Vec::new() };
    let text: String = script.iter().map(|&b| b as char).collect();
    text.lines()
        .filter_map(|line| {
            let line = line.split("//").next().unwrap_or("").trim();
            let rest = line.strip_prefix("Sound").or_else(|| line.strip_prefix("SOUND")).or_else(|| line.strip_prefix("sound"))?;
            let (name, file) = rest.split_once(',')?;
            let file = file.trim().trim_end_matches(';').trim().trim_start_matches('<').trim_end_matches('>');
            Some((name.trim().to_lowercase(), file.to_owned()))
        })
        .collect()
}

/// Decoded PCM: interleaved 16-bit samples.
#[derive(Clone, Debug)]
pub struct Pcm {
    pub rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

/// Decodes an uncompressed PCM WAV (8 or 16 bit, any channel count).
pub fn decode_wav(data: &[u8]) -> Option<Pcm> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }
    let (mut fmt, mut pcm) = (None, None);
    let mut at = 12;
    while at + 8 <= data.len() {
        let id = &data[at..at + 4];
        let len = u32::from_le_bytes(data[at + 4..at + 8].try_into().ok()?) as usize;
        let body = data.get(at + 8..(at + 8 + len).min(data.len()))?;
        match id {
            b"fmt " if body.len() >= 16 => {
                let format = u16::from_le_bytes([body[0], body[1]]);
                let channels = u16::from_le_bytes([body[2], body[3]]);
                let rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
                let bits = u16::from_le_bytes([body[14], body[15]]);
                fmt = (format == 1).then_some((channels, rate, bits));
            }
            b"data" => pcm = Some(body),
            _ => {}
        }
        // Chunks are padded to an even length.
        at += 8 + len + (len & 1);
    }
    let ((channels, rate, bits), body) = (fmt?, pcm?);
    let samples = match bits {
        8 => body.iter().map(|&b| (b as i16 - 128) << 8).collect(),
        16 => body.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes([c[0], c[1]])).collect(),
        _ => return None,
    };
    Some(Pcm { rate, channels: channels.max(1), samples })
}

#[cfg(test)]
mod tests;
