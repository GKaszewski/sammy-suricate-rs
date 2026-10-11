use super::*;

#[test]
fn parses_wdl_strings() {
    let mut s = WdlStrings::default();
    s.parse(b"STRING  QuitYesNo_Str,   \"Wyj\x9Ccie?\";\r\n// STRING fake, \"x\";\r\nSTRING  Two,\r\n\"a\r\nb\";\r\n");
    assert_eq!(s.get("quityesno_str"), b"Wyj\x9Ccie?");
    assert_eq!(s.get("Two"), b"a\nb");
    assert!(s.get("fake").is_empty());
    assert_eq!(cp1250("Wyjście?"), b"Wyj\x9Ccie?".to_vec());
}

#[test]
fn lays_out_lines() {
    let font = FontMetrics { cell: [6.0, 9.0], ink: vec![(0.0, 6.0); 256] };
    let (glyphs, size) = layout(&font, b"ab\nc", false);
    assert_eq!(glyphs.len(), 3);
    assert_eq!(glyphs[2].dst, [0.0, 9.0, 6.0, 9.0]);
    assert_eq!(size, [12.0, 18.0]);
}
