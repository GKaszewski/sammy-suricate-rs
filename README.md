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
cargo run -p sammy                              # play from level 1 (or: -- <slot 1-49> | <file.wmb>)
cargo run -p sammy -- --model suricat1.mdl walkA # model viewer with animation
cargo run -p sammy-convert -- extract "$SAMMY_DIR/sammypolish.wrs" extracted
cargo run -p sammy-convert -- level out extracted/levels/*.wmb
cargo run -p formats --example wmbinfo -- extracted/levels/10001.wmb
```

Debug env vars for `sammy`: `SAMMY_SCREENSHOT=out.png` + `SAMMY_SHOT_SECS=2.5,4` (capture at those seconds, log state, exit),
`SAMMY_SKIP=action1,action2` (hide objects by action), `SAMMY_NOLM=1` (no lightmaps),
`SAMMY_NODUMMY=1` (hide `*dummy*`-textured brushes),
`SAMMY_START=x,y,z[,pan]` (spawn point), `SAMMY_KEY=255` (start with a key), `SAMMY_LIVES=n`, `SAMMY_HEARTS=n`, `SAMMY_AUTOWALK=1` (hold forward).

Controls: arrows walk/turn, Ctrl jump, Enter skip intro sweep / dismiss help sign, F1 free-fly camera (RMB look, WASD, Space/Q, Shift).
