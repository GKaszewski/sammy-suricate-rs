# Architecture and refactoring roadmap

Goal: a portable, idiomatic code base. The game rules live once, in plain Rust with no
engine dependency, so they can run on another frontend (e.g. PS Vita via vitasdk) and
be tested headlessly. Bevy is one frontend: rendering, audio, input devices, windows, UI.

## Target layout

```
crates/
  formats/       file formats (WRS, WMB, MDL, PCX, SAV, LZSS, traces); no_std + alloc
  sammy-core/    the game: no engine, no I/O, no clocks
    units.rs       newtypes: Quants, Ticks, Degrees, Percent ...
    coords.rs      A4 axes and angle rules
    rng.rs         WDL random()
    level/         level description built from a WMB (objects, actions, collision)
    player/  enemies/  wasps/  doors/  gems/  platforms/  health/  extro/  intro/
                   each: state types, constants (named, with WDL cites), tick functions
    flow/          phases, win/lose, next level, menu choices
    save/          SaveGame model and rules (serialisation, not file access)
    events.rs      what the frontend must present (sounds, effects, popups, music)
    sim.rs         `Sim::tick(&mut self, input: &Controls) -> Events` at 16 Hz
  sammy/         Bevy frontend (binary + lib)
    <feature>/     plugin.rs  components.rs  resources.rs  systems.rs  (only what it needs)
                   render/, audio/, input/, display/, hud/, menu/, options/, particles/ ...
```

Rules:
- `sammy-core` never depends on Bevy, `std::fs`, `std::time` or threads. Time arrives
  as ticks, input as a `Controls` snapshot, randomness from its seeded `Rng`.
- No magic numbers: every WDL value is a named constant with its source cite, or an
  enum / newtype.
- The frontend reads core state and events; it never changes game rules.
- Feature plugins are self-contained and can be left out (Cargo features for optional
  parts: music, intro, debug tools).

## Safety net

`crates/sammy/tests/replay.rs` plays six scenarios headlessly (levels 1, 4, 20, 36 with
scripted input, the extro, the boot intro): fixed 60 Hz steps, seeded RNG,
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
   flow), the Bevy side becoming adapters.
5. Restructure the Bevy crate into feature plugins (`plugin/components/resources/systems`),
   Cargo features for optional plugins.
6. Hygiene pass: remaining magic numbers, clippy clean, docs.
