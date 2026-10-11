# Architecture and refactoring roadmap

Goal: a portable, idiomatic code base. The game rules live once, in plain Rust with no
engine dependency, so they can run on another frontend (e.g. PS Vita via vitasdk) and
be tested headlessly. Bevy is one frontend: rendering, audio, input devices, windows, UI.

## Layout

```
crates/
  formats/       file formats (WRS, WMB, MDL, PCX, SAV, LZSS, traces); no_std + alloc
  sammy-core/    the game rules, plain Rust: no Bevy, no I/O, no clocks, no threads
    time.rs angle.rs rng.rs id.rs      ticks, A4 angles, WDL random(), entity ids
    levels.rs save.rs game.rs          level table, SaveGame model, score/lives state
    anim.rs collision.rs physics.rs    MDL scenes, hulls + `Space` trait, jump params
    player.rs input.rs                 Sammy's movement (`Player::step`), `Controls`
    doors.rs keys.rs gems.rs pickups.rs platforms.rs health.rs
    wasps.rs enemies.rs                lions, wasps, hives, planks
    camera.rs flow.rs level.rs         follow camera, level phases, object dispatch
    extro.rs intro.rs menu.rs titles.rs decor.rs eye.rs
  sammy/         Bevy frontend (lib + thin binary)
    lib.rs         `SammyPlugins` (the plugin group), `run()`
    schedule.rs    `GameSet` steps and the `FRAME` order
    gameplay/      one folder per feature: mod.rs (plugin) components.rs resources.rs
                   spawn.rs systems.rs; dispatch.rs maps level objects to features
    camera/ flow/ level/
    render/        models, level material + WESL shaders, lights, shadows, particles
    ui/            hud, hud3d, text, menu/, options/, intro/
    platform/      audio, music, input (keyboard + gamepad), display, vfs, save, debug
```

Core state types are wrapped on the Bevy side in `Deref` newtype components and
resources (orphan rule); core functions return events (`DoorEvent`, `DeathEvent`,
`Fire` ...) that the systems apply. Entities cross the boundary as `Id(u64)`
(`Entity::to_bits`).

### Frame order

`schedule.rs` gives every gameplay system its own `GameSet` step; `FRAME` lists the
steps and the explicit command sync points (`ApplyDeferred`) in order. Both are part of
the game's behaviour (later steps see earlier spawns; random rolls happen in order), so
the replay test locks them. So does plugin registration order in `SammyPlugins`: it sets
component registration and so query iteration order, which decides which gem draws which
random colour.

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
- Feature plugins are self-contained and can be left out (Cargo features for optional
  parts: music, intro, debug tools).

## Safety net

`crates/sammy/tests/replay.rs` plays scenarios headlessly (levels 1, 4, 20, 36 with
scripted input, the extro, the boot intro, and a 20 s sweep of all 36 levels): fixed 60 Hz steps, seeded RNG,
single-threaded schedules. Every half second it records the game state, Sammy's
position and a hash of all named entity transforms, and compares them with
`tests/golden/*.txt`. Every refactoring step must keep it green;
`UPDATE_GOLDEN=1` is only for changes meant to alter behaviour.

## Steps

1. Library + thin binary, replay harness. **(done)**
2. `formats`: `no_std` + `alloc` (verified with `--target thumbv7em-none-eabihf`). **(done)**
3. `sammy-core` skeleton: time, angles, rng, level table, save model (`LevelState` enum), game state; the Bevy side wraps them in `Deref` newtype resources. **(done)**
4. Move gameplay into the core one feature at a time (collision, player, pickups and
   gems, doors, platforms and teleports, health, enemies, wasps, extro, intro walker,
   flow), the Bevy side becoming adapters. **(done)**
5. Restructure the Bevy crate into feature plugins (`mod/components/resources/systems`),
   explicit frame schedule, Cargo features for optional plugins. **(done)**
6. Hygiene pass: remaining magic numbers, clippy clean (`--all-targets`), docs.

Next (not started): a core-owned `Sim` that owns the world state and ticks it at 16 Hz
(`Sim::tick(&mut self, &Controls) -> Events`), so a non-Bevy frontend (vitasdk) only
draws, plays sounds and reads input.
