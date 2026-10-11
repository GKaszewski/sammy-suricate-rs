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
The game needs your copy of the original *Sammy Suricate* (the installed folder). It looks
for it, in order:
1. `--data <folder>` on the command line, or `SAMMY_DIR`;
2. next to the executable: the game folder itself, or a `data` folder beside it (macOS
   app bundles: `Contents/Resources/data`);
3. the folder picked on an earlier run;
4. otherwise it asks with a folder dialog and remembers the choice.

Saves and settings live in the per-user data folder (`~/.local/share/sammy-suricate`,
`%APPDATA%\sammy-suricate`, `~/Library/Application Support/sammy-suricate`; `SAMMY_HOME`
overrides). The original game's saves are imported on first run. Music needs a General
MIDI SoundFont (`.sf2`): next to the executable, in that data folder, in the game folder,
or `SAMMY_SOUNDFONT`.

```sh
cargo run -p sammy                              # menu (or: -- <slot 1-49> | <file.wmb>)
cargo run -p sammy -- --data "/path/to/Sammy Suricate" 4   # a level from a given install
cargo run -p sammy -- --model suricat1.mdl walkA # model viewer with animation
cargo run -p sammy-convert -- extract "$SAMMY_DIR/sammypolish.wrs" extracted
cargo run -p sammy-convert -- level out extracted/levels/*.wmb
cargo run -p formats --example wmbinfo -- extracted/levels/10001.wmb
```

Debug env vars for `sammy`: `SAMMY_SCREENSHOT=out.png` + `SAMMY_SHOT_SECS=2.5,4` (capture at those seconds, log state, exit),
`SAMMY_SKIP=action1,action2` (hide objects by action), `SAMMY_NOLM=1` (no lightmaps),
`SAMMY_NODUMMY=1` (hide `*dummy*`-textured brushes),
`SAMMY_START=x,y,z[,pan]` (spawn point), `SAMMY_KEY=255` (start with a key), `SAMMY_LIVES=n`, `SAMMY_HEARTS=n`, `SAMMY_AMMO=count,type`, `SAMMY_AUTOFIRE=1`, `SAMMY_EYE=1`, `SAMMY_SEED=n`, `SAMMY_AUTOWALK=1` (hold forward).

Tests that need the game files find them like the game does (`SAMMY_DIR`, else the
folder the game remembered) and are skipped without them:
`cargo test --release --workspace`.

## Distributing
A release is the executable plus the bundled SoundFont; players bring their own copy of
the game (its data is not ours to ship):

```
sammy-suricate/
  sammy(.exe)
  GeneralUser-GS.sf2   # GeneralUser GS by S. Christian Collins (free to bundle)
  README.txt           # "put this folder's contents into your Sammy Suricate folder,
                       #  or start it and pick that folder"
```

Build per platform: `cargo build --release -p sammy` on Linux; Windows with
`--target x86_64-pc-windows-gnu` (cross) or natively; macOS on a Mac (an app bundle with
the data in `Contents/Resources/data` is found automatically). The web and PS Vita
builds share the same interfaces (`platform::vfs::GameFiles` from memory,
`platform::storage::Storage`) and are next.

Controls: arrows walk/turn, Ctrl jump, Space throw wasp, Enter skip intro sweep / dismiss help sign, F1 free-fly camera (RMB look, WASD, Space/Q, Shift).
