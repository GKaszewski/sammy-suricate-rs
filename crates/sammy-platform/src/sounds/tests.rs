use super::*;

#[test]
fn decodes_pcm() {
    let mut w = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
    w.extend(16u32.to_le_bytes());
    w.extend(1u16.to_le_bytes()); // PCM
    w.extend(1u16.to_le_bytes()); // mono
    w.extend(11025u32.to_le_bytes());
    w.extend(11025u32.to_le_bytes());
    w.extend(1u16.to_le_bytes());
    w.extend(8u16.to_le_bytes());
    w.extend(b"data");
    w.extend(3u32.to_le_bytes());
    w.extend([128, 255, 0, 0]);
    let p = decode_wav(&w).unwrap();
    assert_eq!((p.rate, p.channels), (11025, 1));
    assert_eq!(p.samples, vec![0, 127 << 8, -128 << 8]);
}
