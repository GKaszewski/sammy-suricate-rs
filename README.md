# Sammy Suricate — reverse engineering & Rust/Bevy remake

Rebuild of *Sammy Suricate* (2002, Rémi Valantin & Jörg Henseler), a Conitec 3D GameStudio
A4/A5 game, in Rust + Bevy. The remake reads the **original game install** directly; no
game data is stored in this repo.

## Layout
- `crates/formats` — parsers: `wrs` (archive), `lzss`, `wmb` (WMB4 levels: BSP, RGB565
  textures, lightmaps, entities), `mdl` (MDL3/MDL5 models), `pcx`.
- `crates/sammy-convert` — CLI: `extract`, `images` (PCX→PNG), `level` / `model` (→ .glb for Blender).
- `crates/sammy` — Bevy game/viewer.
- `docs/formats/` — byte-level format docs. `docs/gameplay.md` — game design as implemented in
  the WDL scripts. `docs/wdl-porting.md` — WDL→Bevy porting guide.

## Running
Game dir defaults to the Wine install; override with `SAMMY_DIR=/path/to/Sammy Suricate`.

```sh
cargo run -p sammy -- 10001.wmb                 # level viewer (fly: RMB look, WASD, Space/Ctrl, Shift)
cargo run -p sammy -- --model suricat1.mdl walkA # model viewer with animation
cargo run -p sammy-convert -- extract "$SAMMY_DIR/sammypolish.wrs" extracted
cargo run -p sammy-convert -- level out extracted/levels/*.wmb
cargo run -p formats --example wmbinfo -- extracted/levels/10001.wmb
```

Debug env vars for `sammy`: `SAMMY_SCREENSHOT=out.png` (capture frames 150/250 and exit),
`SAMMY_SKIP=action1,action2` (hide objects by action), `SAMMY_NOLM=1` (no lightmaps),
`SAMMY_NODUMMY=1` (hide `*dummy*`-textured brushes).
