# Architecture and refactoring roadmap

Goal: a portable, idiomatic code base. The game rules live once, in plain Rust with no
engine dependency, so they can run on another frontend (e.g. PS Vita via vitasdk) and
be tested headlessly. Bevy is one frontend: rendering, audio, input devices, windows, UI.

## Layout

```
crates/
  formats/       file formats (WRS, WMB, MDL, PCX, SAV, LZSS, traces); no_std + alloc
  sammy-core/    the whole game, plain Rust: no engine, no I/O, no clocks, no threads
    sim/           `Sim`: the world, the frame, level loading (see below)
      world.rs       entity ids and tables (bodies, looks, animators, one table per kind)
      spawn.rs       level objects by action (the WDL inits), `Presentation` → `Look`
      flow.rs player.rs platforms.rs doors.rs gems.rs pickups.rs health.rs wasps.rs
      enemies.rs extro.rs camera.rs intro.rs menu.rs      one file per feature's steps
      events.rs      what the frontend presents (sounds, effects, popups, saves)
      assets.rs      `Files` trait + parsed WMB/MDL cache
    time.rs angle.rs rng.rs id.rs      ticks, A4 angles, WDL random(), entity ids
    levels.rs save.rs game.rs          level table, SaveGame model, WDL globals
    anim.rs collision.rs physics.rs    MDL scenes, hulls + `Space` trait, jump params
    player.rs doors.rs keys.rs gems.rs pickups.rs platforms.rs health.rs wasps.rs
    enemies.rs camera.rs flow.rs extro.rs intro.rs menu.rs titles.rs decor.rs eye.rs
                   the rules per feature (state types, constants with WDL cites, steps)
  sammy/         Bevy frontend (lib + thin binary)
    game.rs        the `Sim` as a resource, ticked once per frame; events → messages
    scene/         level geometry, the mirror (game entity → Bevy entity), indicators
    camera/        main camera at the game's camera pose; fly camera; field of view
    render/        models, level material + WESL shaders, lights, shadows, particles
    ui/            hud, hud3d, text, menu/ (picking), options/, intro/ (logos, fog)
    platform/      audio, music, input (keyboard + gamepad), display, vfs, save, debug
```

### The simulation

`Sim::tick(secs, &Controls, &dyn Files)` runs one frame: `t = secs × 16` ticks, like
the original's variable `time`. Its steps run in a fixed order (the original port's
frame, listed in `sim/mod.rs`); entities update in spawn order (ids are never reused),
so behaviour never depends on an engine's internals. State lives in typed tables on
`World` (`players`, `doors`, `key_gems`, `shrinking` ...), joined by id, which keeps
the steps close to the original WDL loops. What the player sees is `Body` (position,
angles, scale, hidden, alpha) and `Look` (model / prefab / sprite, revision for morphs)
plus `Animator`/`SpriteAnim`/`Light`; anything else is presented from events.

A frontend:
1. implements `Files` (bytes by file name) and builds `Sim::new(save, seed)`;
2. each frame: `tick`, then draws the tables and `sim.camera.pose`, and handles
   `drain_events()` (sounds, effects, popups, `Saved` → write the save);
3. feeds UI decisions back: `menu.hovered`, `menu_choice(code)`, `abandon()`,
   `give_up()`, `request_load(file)`.

The Bevy frontend does exactly that (`game.rs`, `scene/mirror.rs`); a vitasdk frontend
would do the same with its own renderer.

### Cargo features (`sammy`)

| feature       | default | what                                           |
|---------------|---------|------------------------------------------------|
| `music`       | yes     | MIDI music through rustysynth + a SoundFont    |
| `intro`       | yes     | the boot intro (without it: straight to menu)  |
| `debug-tools` | yes     | `SAMMY_SCREENSHOT` timed screenshots + state log |

`cargo build -p sammy --no-default-features` must build warning-free.

Rules:
- `sammy-core` never depends on Bevy, `std::fs`, `std::time` or threads. Time arrives
  as ticks, input as a `Controls` snapshot, randomness from its seeded `Rng`.
- No magic numbers: every WDL value is a named constant with its source cite, or an
  enum / newtype.
- The frontend reads core state and events; it never changes game rules.
- Debug overrides reach the core as `DebugOptions` (the frontend reads the environment).

## Safety net

`crates/sammy-core/tests/sim_replay.rs` plays scenarios on the `Sim` alone (levels 1,
4, 20, 36 with scripted input, the extro, the boot intro, and a 20 s sweep of all 36
levels): fixed 60 Hz frames, seeded RNG. Every half second it records the game state,
Sammy, the phase and a hash of every body, and compares them with
`tests/golden/*.txt` (about a second for everything). `UPDATE_GOLDEN=1` is only for
changes meant to alter behaviour. `lions_hurt_sammy` checks lion combat directly.
`crates/sammy/tests/smoke.rs` runs the Bevy frontend headless and checks that the
scene mirrors the game.

## Steps

1. Library + thin binary, replay harness. **(done)**
2. `formats`: `no_std` + `alloc` (verified with `--target thumbv7em-none-eabihf`). **(done)**
3. `sammy-core` skeleton: time, angles, rng, level table, save model (`LevelState` enum), game state; the Bevy side wraps them in `Deref` newtype resources. **(done)**
4. Move gameplay into the core one feature at a time (collision, player, pickups and
   gems, doors, platforms and teleports, health, enemies, wasps, extro, intro walker,
   flow), the Bevy side becoming adapters. **(done)**
5. Restructure the Bevy crate into feature plugins (`mod/components/resources/systems`),
   explicit frame schedule, Cargo features for optional plugins. **(done)**
6. Hygiene pass: remaining magic numbers, clippy clean (`--all-targets`), docs. **(done)**
7. Core-owned `Sim`: the world, the frame and level loading in `sammy-core`; Bevy only
   presents it. Verified identical (game state and Sammy's path in every scenario) to
   the engine port before the switch. **(done)**
