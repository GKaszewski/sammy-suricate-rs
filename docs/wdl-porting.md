# WDL (Acknex A4 v4.22) to Rust + Bevy porting guide: Sammy Suricate

This guide is for porting the 23 WDL scripts in `extracted/*.wdl` (about 27k lines). Read it alongside `gameplay.md`, which describes what the scripts do; this file covers how the language and engine behave and how to map them to Bevy.

**Conventions**
- Citations are `file.wdl:line`. Line numbers match the original CRLF files and the UTF-8 conversions in the scratch directory.
- **[R]** means read directly from the scripts.
- **[I]** means inferred from usage plus general Acknex A4/A5 knowledge. Treat [I] as a hypothesis and confirm it in the running game (Wine install in `/mnt/drive/Games/sammy-suricate`) before relying on it.

**Encoding.** The scripts are CP1252/Latin-1, but the Polish UI strings are **CP1250** bytes (for example `panels.wdl:121` decodes to "Ćwicz zręczność 1" only under CP1250) [R, verified with iconv]. A few French comments look like CP850 ("‚" for é). When extracting strings for the port, decode `STRING` literals in `panels.wdl`, `SamTexts.wdl`, etc. as CP1250. The comments do not matter for the port.

**Engine version caveat.** The scripts are A4-style WDL (`Define 422`), but the shipped Polish exe appears to run on **A5** (`mm1.wdl:815`: "level compiled for A5.203, the clip clips blocks", which is why clip range is ×1.5; the intro shows an A5 logo, `Intro2.wdl:11-23`). Where A4 and A5 semantics differ (sonar vs trace, `ent_move`), the observed game reflects the A5 runtime. Verify against the running game.

---

## 1. Units and conventions

| Concept | A4 meaning | Port mapping |
|---|---|---|
| Distance | 1 **quant**, roughly 1 inch. Sammy is a model scaled ×8 (`player.wdl:34-36`) whose centre sits 35.2 quants above the floor (`HeightFootPlayer 4.4 × scale 8`, `mm1.wdl:36`, `player.wdl:71`). | Keep quants as world units (1.0 = 1 quant) so every constant ports verbatim. Scale only the render/camera if needed. |
| Time | 1 **tick** = 1/16 s [I, consistent with `WaspTemp1=128` commented "8s de vie", `bullets.wdl:212`]. `time` = duration of the last frame in ticks. `TIME_FAC` is related (used for fps smoothing, `mm1.wdl:793`). | Run gameplay in `FixedUpdate` at **16 Hz** (exact tick semantics, `time == 1`), or at 64 Hz with `time = 0.25`. Many smoothing formulas use per-frame constants not scaled by time (see §5.6), so a fixed rate is strongly preferred. |
| `mintime` | Script global `Min(time,1)`, recomputed each frame in the game loop (`mm1.wdl:793,898`). | Same as `time` when ticking at ≥16 Hz. |
| Axes | X forward/east, Y left/north, Z up. Right-handed, Z-up [I]. | Bevy is Y-up, -Z forward. Convert with `bevy = (x, z, -y)`, a proper rotation (det +1). Do the conversion once at the render/asset boundary and keep all gameplay maths in A4 space. |
| Angles | Degrees. `pan` = yaw about +Z, 0 = +X, increasing counter-clockwise seen from above [I]. `tilt` = pitch, positive nose up. `roll` = bank. `Ang(x)` wraps to (-180,180] [I]. | Store `Euler { pan, tilt, roll }` in degrees. Build the quaternion as `Rz(pan)·Ry(-tilt)·Rx(roll)` in A4 space [I, verify the tilt sign against the camera code: camera tilt is clamped `<= 20` and the default is about -56°, `camera.wdl:850`]. |
| "Forward" for MOVE | Relative vector `(x,0,0)` rotated by the entity's pan/tilt. | `rotate(euler) * rel`. |
| Colours | 0-255 per channel for lights and particles (`lightred`, `my_color.red`). | `Color::srgb_u8`. |
| Alpha | `my.alpha` 0-100 with `transparent=1`. `ent_alphaset(frame, alpha)` [I]. | `alpha/100`. |
| Numeric type | A4 `var` is **fixed point** (22.10, about 0.001 resolution, range about ±2 million) [I]. Scripts compare floats with `==` (`gems.wdl:82,91` scale 1.5/0.75, `lion.wdl` `Int(x==8)`), which works only because of fixed point. | Use `f32`, but replace exact-equality tests with state flags or tolerance (see §6). |
| Vectors | Every `SKILL` is a 3-component value: `.x/.y/.z` alias `.pan/.tilt/.roll` (e.g. `ScanSphere {Pan 360; Tilt 360;}` then `ScanSphere.Z=…`, `skills.wdl:170`, `player.wdl:222`) [R]. | Model global skills as `Vec3` (or `f32` where only `.x` is used). |

---

## 2. Language constructs

| Construct | Count | Semantics | Bevy / Rust equivalent |
|---|---|---|---|
| `ACTION name { … }` | 555 decls | A coroutine body. Attached to entities in WED (the entity-lump action string) or created via `Create`. `MY` = owner entity. | Per-entity **state machine component** plus systems (§3). Level loader maps the action string to a spawn function. |
| `FUNCTION` | 0 | not used | n/a |
| `CALL act` | ~434 | Runs `act` synchronously **until it first yields**. If `act` waits, the caller continues and `act` keeps running as a separate coroutine with the same `MY` [I, required by `player.wdl:1269`: `HandleSpeedPlayer` → `Call FinJump`, and FinJump loops with `waitt 1` waiting for physics that only HandleSpeedPlayer advances; a blocking call would deadlock. Also `mm1.wdl:947` `Call AnimGagnee` (endless loop) followed by `Create <win.mdl>`]. | A plain Rust fn for non-yielding helpers. For yielding callees, **spawn a sub-task** (a new component/state on the same entity, or a separate "task entity" pointing at it). |
| `BRANCH act` | ~123 | Tail call: replaces the current coroutine with `act` (same `MY`); never returns [R usage, e.g. `gems.wdl:73`]. | State transition: set the entity's state enum to the target. |
| `GOTO label` / `label:` | ~116 | Jump inside the action, including into `If` blocks (`lion.wdl` RandomizeLion "beurk"). | Restructure as loops/states. |
| `END` | ~363 | Return from the action or event. | `return` / remove task. |
| `IF/ELSE`, `WHILE` | 2542 / 240 | No `else if`. Conditions use `==, !=, <, >, &&, \|\|` and a bitwise `&` (`mm1.wdl:2315-2356`, `(PlayerKey & Skill6)`). | Rust control flow. |
| `SET a,b` / `a=b`, `+=`, `-=`, `*=` | ~2500 | Assignment. Both forms are equivalent [R]. | `=` |
| `WAIT n` | ~41 | Yield n **frames**. | Yield one step of the state machine. |
| `WAITT n` | ~416 | Yield until n **ticks** of game time have passed (time-corrected) [I]. | `Timer` countdown in the state, decremented by tick dt. |
| `SKILL name {VAL v;}` / `VAR name[n]` | ~476 / 14 | Global variable (vec3) / global array (`MusicOrder[50]`, `LevelsInfo[]`, `BestScores[]`, `Player1RestartX[]`). | Fields on `Resource` structs grouped by domain (§7). |
| `DEFINE X,val;` / `DEFINE X,SkillN;` / `DEFINE flag;` | 247 | Text substitution. Used for numeric constants (`RangeScanDoor,500` at `mm1.wdl:165`), **entity-field aliases** (`Define LionEtat,Skill31`), and conditional-compile flags. | Rust `const`. Field aliases become named fields of per-type components (§4). |
| `IFDEF / IFNDEF / IFELSE / ENDIF` | 83 / 11 / 10 / 94 | Conditional compilation. Active defines: `422, polish, frenchfullversion, tgawinlose, greenpepper, musique1` (`sammypolish.wdl:1-15`). **Not defined:** `softrender, voodoo, beta, shareware, demoversion, realnet, dutchversion, mountainking, sammynewversion`. | Port only the active branches. Note that `IFDEF 422` branches (trace instead of sonar) are **active**. |
| `SYNONYM name {TYPE Entity\|Action\|Panel;}` | 94 | A global pointer to an entity, action or panel (`Pov_Syn` = player, `Camera_Syn`, `CameraHandle_Syn` = action pointer) [R `skills.wdl`]. | `Resource` holding `Option<Entity>` or an enum for action pointers. |
| `STRING name,"text";` | 546 | Global string (level titles, help, animation scene names). | `&'static str` / localisation table. |
| `INCLUDE <f.wdl>`, `PATH "dir"`, `BIND <file>` | 22 / 6 / 98 | Source inclusion, asset search path, force asset into the resource file. | Asset manifest. `BIND` lists the prefab WMBs and models that levels reference (`sammypolish.wdl:96-202`). |
| `MAIN Main;` | 1 | Entry point (`sammypolish.wdl:220`). | App startup system. |
| `ON_xxx act` / `Set On_Esc,act` | ~60 | Key-press callback. It can be rebound at runtime, and handlers are saved and restored (`options.wdl` savekeys/restoreKeys). | `ButtonInput<KeyCode>` `just_pressed` checks inside a system, gated by a `Resource` "bindings mode" enum. |

### Built-in maths and functions

| Name | Count | Semantics | Rust |
|---|---|---|---|
| `random(n)` | 486 | Uniform float in [0,n) [I]. `int(random(n))+1` gives 1..n. | `rng.gen_range(0.0..n)`. Use a seeded RNG for testability. |
| `int(x)` | 312 | Truncate toward 0 [I]. | `x.trunc()` |
| `frc(x)` | 30 | Fractional part. Used to **pack two editor parameters in one skill** (`lion.wdl:14-24`, `bullets.wdl` hive Skill3) [R]. | `x.fract()`. Do the unpacking in the level loader. |
| `abs, min, max, sign` | 91/28/23/15 | Standard. `Abs(Speed.X > 1)` and `Int(x==8)` apply them to a boolean (gives 0/1) [R `lion.wdl`]. | Standard. Translate boolean quirks deliberately. |
| `ang(x)` | 101 | Normalise an angle to (-180,180] [I]. | `wrap_deg()` |
| `sin, cos, asin` | 9/10/2 | Degrees [I]. | `.to_radians()` wrappers. |
| `vec_set, vec_add, vec_length, vec_dist` | 55/10/3/3 | Vector ops. | `Vec3` |
| `normalize v,len` | 14 | Scales v to length `len` [I]. One call passes a vector as the length (`particul.wdl:1308`, quirk). | `v.normalize_or_zero()*len` |
| `vecrotate v,angles` | 25 | Rotates v by pan/tilt/roll [I]. | `quat_from_euler(angles) * v` |
| `to_angle angles,dir` | 39 | Sets `angles.pan/.tilt` from a direction and **`angles.z` (roll slot) = the vector's length** [I, relied on everywhere as distance: `OuvrePortail mm1.wdl:3114`, `TestDistLionPlayer lion.wdl:944`]. | `fn to_angle(d)->(pan,tilt,len)` |
| `accel speed,acc,friction` | 29 | Velocity integrator [I]. Usage implies `speed += acc*time` with friction 0 (`player.wdl:1321`), exponential decay with acc 0 / friction 0.2 (`player.wdl:1245`), and near-assignment with friction 1 (`player.wdl:1326`). Best guess: `speed = speed*(1 - min(friction*time,1)) + acc*time`. | Write that helper and test it against the jump-arc numbers (§gameplay player physics). |
| `content(pos)` | 3 | Returns `CONTENT_SOLID` if the point is inside level geometry [I]. | Point-in-solid query on level BSP/collider. |

---

## 3. Concurrency model and how to port it

### 3.1 What WDL does
- Every `ACTION` started on an entity is a **cooperative coroutine**. The scheduler runs all ready coroutines once per frame. `wait 1` yields one frame. `waitt n` yields until n ticks have elapsed.
- **Events are synchronous callbacks.** When `MOVE` collides, `SCAN` hits or `SONAR` finds a floor, the target's `EVENT` action runs *immediately inside* the mover's instruction, with `MY` = target, `YOU` = the other entity, and `EVENT_TYPE` set [I]. This is what makes the **ScanIdentite pattern** work: the caller sets a global, scans, and resets the global, all within one instruction sequence:
  ```
  SET ScanSphere.Z,50000; SET ScanIdentite,1000;   // player.wdl:886-890 (RestartSammy)
  SCAN MY.POS,MY.ANGLE,ScanSphere;                 // every scan-enabled entity's EVENT runs now
  SET ScanIdentite,0;
  ```
  Event handlers may themselves `waitt` (e.g. `Key_Hit` → `ScaleDownGem` shrinks over several ticks, `gems.wdl:351`). The handler then continues as its own coroutine [I].
- Shared state is global variables plus entity skills. There is no locking because the scheduler is single-threaded.
- `MY`/`YOU` are per-coroutine registers. Scripts reassign them freely: `my=you; branch player_Kaputt` (`mm1.wdl:4890`, `lion.wdl:495`) lets one entity "become" the player's coroutine.

### 3.2 Recommended Bevy architecture
Do **not** write a generic WDL interpreter or async executor. Hand-port each action family into explicit state machines:

1. **Tick schedule.** Put gameplay in `FixedUpdate` with `Time::<Fixed>::from_hz(16.0)` (or 32/64 Hz with `dt_ticks = 16.0 * dt_secs`, passed as `time`). Rendering and camera interpolation can run in `Update` with a `Transform` lerp.
2. **One component per actor kind** (`Gem`, `Lion`, `Door`, `Wasp`, `Platform`, `PlatformEngine`, …) holding:
   - the named fields that replace `SkillN` aliases (§4);
   - a `state: enum` that replaces `LionEtat`, `Wasptat`, door `Skill9`, …;
   - a `wait: f32` tick countdown that replaces `waitt`.
3. **`waitt n` inside a loop** becomes `if s.wait > 0.0 { s.wait -= ticks; return; }`.
4. **Sequential multi-step scripts** (cutscenes, `Player_Kaputt`, `RestartSammy`, `OuvrePortail`, intro/extro) port best as an explicit `enum Step { A, B { t: f32 }, … }` advanced each tick. If there are many of these, an alternative is a tiny **coroutine runner**: `Box<dyn FnMut(&mut World, Entity, f32) -> Poll>`, or Rust `async` with a custom single-thread executor and `TickFuture`. The coroutine runner makes the script translation more literal. For about 25 multi-step scripts, hand-written step enums are fine.
5. **Events → Bevy `Event`s processed in the same tick.** The ScanIdentite broadcast becomes:
   ```rust
   #[derive(Event)] struct Scan { from: Entity, origin: Vec3, fwd: Euler, h_fov: f32, v_fov: f32, range: f32, kind: ScanKind }
   enum ScanKind { PlayerPickup /*1*/, WaspSeek /*2*/, CameraOrDoor /*25*/, PathN(u32) /*27-99*/, GemCrush /*100*/,
                   GemTest /*101*/, DoorClear /*102*/, Reset /*1000*/, PlatformLink(u32) /*8465…*/, BonusLink /*2222*/, … }
   ```
   A `scan_system` does the spatial query over `ScanReceiver` components and writes `ScanHit { receiver, from, kind }`. Per-type handler systems consume these. Ordering: run the handlers immediately after the system that emitted the scan, using `.chain()`, so effects land within the same tick as in WDL.
6. **Collision callbacks** (`EVENT_ENTITY`, `EVENT_BLOCK`, `EVENT_IMPACT`, `EVENT_STUCK`, `EVENT_SONAR`) become events emitted by your kinematic mover (§5.1) with `{mover, other, normal, bounce}`.
7. **Global "game loop" coroutines** (`StartintroGame`, `MainMenuSammy`, `Intro`, `Extro`) become Bevy `States` (`AppState::{Boot, Intro, MainMenu, LevelLoading, LevelIntroCam, Playing, WinLose, Extro, Credits}`) plus `OnEnter`/`OnExit` systems.

### 3.3 Worked examples from the scripts

**(a) Gem idle loop** (`gems.wdl:75-175`, `GemWaitForPlayer`):
```
While ((My.Flag1 == 1) && (ScoreJoueur1 < ScoreGoal)) {
    if (my.Skill9 > 0) { …blink…; my.skill9 -= time; if (my.Skill9 <= 0) { Call GemExplose; my.flag1=0; } }
    V = My.Frame + My.Skill31*time; if (V >= 18) {V=1;} my.frame = V;
    If (My.Skill4 != 0) { My.pan += my.skill4*time; }
    …gravity if flag7…
    waitt 1;
}
If (My.Flag3 == 1) { invisible; passable; enable_scan=1; event=GemWaitForNewStart }  // sleep until reset scan
else { invisible; wait lightrange==0; waitt 16; remove my; }
```
Port:
```rust
enum GemState { Idle, Hidden /*persistent, waits for Reset scan*/, Despawning { t: f32 }, Shrinking, … }
fn gem_idle(mut q: Query<(&mut Gem, &mut Transform, &mut SpriteAnim)>, g: Res<GameState>, tick: Res<Tick>) {
    for (mut gem, mut tf, mut anim) in &mut q {
        if gem.state != GemState::Idle { continue; }
        if g.score >= g.score_goal { gem.state = if gem.persistent { GemState::Hidden } else { GemState::Despawning { t: 16.0 } }; continue; }
        if let Some(life) = gem.lifetime.as_mut() { *life -= tick.dt; /* blink */ if *life <= 0.0 { /* explode */ } }
        anim.frame += gem.anim_speed * tick.dt; if anim.frame >= 18.0 { anim.frame = 1.0; }
        tf.pan += gem.spin * tick.dt;
    }
}
```

**(b) `Call` of a yielding action** (`player.wdl:1253-1272`): when the floor drops away by more than 64, `HandleSpeedPlayer` calls `FinJump`, which then runs in parallel waiting for landing (`player.wdl:445-449`). Port: set `player.air = AirState::Falling{ loop_security: 64.0 }`. A separate `player_air_system` runs after physics each tick and performs FinJump's steps. HandleSpeedPlayer itself never blocks.

**(c) Timed door** (`mm1.wdl:3114-3259`, `OuvrePortail`): rise at `Skill4*time` until Z ≥ Skill13, hold `Skill8` ticks with bell blinking in the last 48 ticks, then lower. Port as a `DoorState::{Closed, Rising, Open{t}, Lowering, WaitClear}` machine. Player proximity scan events (kind 25) refresh `Open{t}` exactly like `Skill9=4` does.

**(d) Lion AI** (`lion.wdl:1678-2240`, `EnnemyBehavior`): already a state machine over `LionEtat` (0-13, §gameplay). Port 1:1 as `enum LionState` with one system calling `fn behave(&mut Lion, …)` per tick, followed by `anim_lion`, `move_lion`, `gravity_lion`.

**(e) Particles** (`particul.wdl`): each particle runs its action every frame and reads `MY_AGE`. Port as a CPU particle pool, since 2000 max is tiny: `struct Particle { age, pos, vel, size, color, map, flare, kind }` with a `match kind` per-tick update, rendered as billboards. `bevy_hanabi` is optional; the per-particle frame swaps (`if int(MY_AGE)==N { MY_MAP=… }`) map more naturally to CPU code.

---

## 4. Entity model: skills, flags, fields

| A4 field | Count | Meaning | Port |
|---|---|---|---|
| `SKILL1..SKILL48` | heavy (skill1 175, skill6 182, …) | Per-entity numeric slots. In WED, Skill1-8 are **level-editor parameters**; higher slots are runtime state. The same slot is aliased differently per actor type (`mm1.wdl:187-495` Define tables; e.g. Skill31 = `PlayerEtat` / `LionEtat` / `Wasptat`, Skill43 = `playerKey` / `EnnemyResistance` / `WaspPoison`) [R]. | The **level loader** reads raw `skill[1..8]` and `flag[1..8]` from the WMB entity and builds a typed component per action name (`InitKey` → `Gem{color: skill6, …}`). Runtime-only fields become plain struct fields. **Cross-type writes through shared slots must become explicit events**, e.g. lion.wdl writes `You.LionEtat=1004` to kill a wasp, which becomes `WaspKilled(wasp)`. |
| `FLAG1..FLAG8` | flag1 143, flag3 50, … | Editor booleans and runtime flags. `Flag1` is generally "alive/loop" [R]. | `bool`s / bitflags in the typed component. |
| `ENTITY1`, `ENTITY2` | 96 / 10 | Entity pointers (shadow, linked platform chain, door to gem). | `Option<Entity>` |
| `STRING1/2` | not used | n/a | n/a |
| `x,y,z,pan,tilt,roll,scale_x/y/z` | many | Transform. | `Transform` plus an `Euler` component. |
| `frame, next_frame, skin` | 88 / 14 / 52 | Vertex-animation frame (float, interpolated toward `next_frame`) and skin index [I]. | Custom MDL vertex-anim component and material skin index. |
| `ambient, albedo, lightrange, lightred/green/blue` | 75 / 12 / 41 / 12 each | Self-illumination (-100..100), light reflection, dynamic point light radius and colour. | `StandardMaterial.emissive` approximation; `PointLight { range, color }`. |
| `invisible, passable, transparent, alpha, flare, overlay, facing, oriented, unlit, metal, nofilter, near, shadow, fat, narrow, push` | 152 invisible, 110 passable, … | Render flags (`facing` = billboard, `oriented` = flat sprite using its angles, `flare` = additive, `overlay` = colour-key transparency, `near` = drawn in front, `shadow` = engine shadow) and collision flags (`passable`, `fat/narrow` = hull size class, `push` = collision priority; an entity can't be pushed by lower push) [I]. | `Visibility`, `AlphaMode::{Blend, Add, Mask}`, billboard component, collision layer/priority fields. |
| `enable_scan/entity/impact/block/stuck/sonar/detect/trigger/touch/click/release`, `event`, `trigger_range` | 54 / 26 / 22 / 13 / … | Which events the entity receives, and the handler action. | Marker components (`ScanReceiver`, `SonarReceiver`, …) plus the handler being the typed component's system. |
| `min_x/max_x/…/max_z` | small | Bounding box (from the model; scripts shrink trees `mm1.wdl:1717`). | Collider AABB. |
| `my_speed` (entity) | n/a | Actual displacement of the last MOVE [I]. | Stored by the mover. |

---

## 5. Engine instructions and predefined variables

### 5.1 Movement and collision

| Built-in | Count | Semantics [mostly I] | Bevy equivalent |
|---|---|---|---|
| `MOVE ent, rel, abs` | ~32 | Kinematic move: `rel` rotated by the entity's angles plus `abs` in world space. Collides the entity hull (fat/narrow box/ellipsoid) against the level and non-passable entities, **glides** along walls, steps over small ledges. Fires `EVENT_BLOCK` (level), `EVENT_ENTITY` (mover hits entity), `EVENT_IMPACT` (target was hit), `EVENT_PUSH`. Sets `RESULT` = distance moved and `my_speed` = displacement; `NORMAL` and `BOUNCE` = contact normal and reflected direction. Scripts **pre-multiply by `time`**, so MOVE itself is not time-scaled. | Custom kinematic character controller (collide-and-slide) against the level collision mesh from WMB. Avian3d/Rapier's `KinematicCharacterController` / shape-casts can do it, but the faithful behaviour (glide, `bounce`, events) is easier with your own shape-cast loop. |
| `ENT_MOVE(rel, abs)` + `MOVE_MODE` (`IGNORE_YOU+GLIDE+IGNORE_ME+IGNORE_PASSABLE+IGNORE_PASSENTS`) | 2 | A5-style variant (`bullets.wdl:1100-1110`) | same mover with flags |
| `SONAR ent, dist` | ~24 | Downward ray from the entity (scripts raise Z by 8 first: `player.wdl:1211-1213`), up to `dist`. Sets `TARGET` = hit point, `NORMAL`, `RESULT`, and fires `EVENT_SONAR` on the entity below (platforms, exit/death tiles, lava, teleports use this). | Raycast down. Then emit `SonarHit{ from, under }` if the hit entity has `SonarReceiver`. |
| `TRACE(from,to)` with `TRACE_MODE` flags `IGNORE_ME, IGNORE_YOU, IGNORE_PASSABLE, IGNORE_PASSENTS, IGNORE_SPRITES, IGNORE_MODELS, USE_BOX, ACTIVATE_SONAR` | ~21 | Line or box trace. `RESULT` = distance to hit (0 = no hit, -1 = start inside solid [R usage `lion.wdl` muzzle trace]). `YOU` = hit entity; `TARGET`, `NORMAL` set. `ACTIVATE_SONAR` makes it fire `EVENT_SONAR` like SONAR. Because `Define 422` is active, `IFDEF 422` branches use trace instead of sonar (`mm1.wdl:1210`, `3913-3923`). | Raycast / shape-cast with a filter. Return `Option<Hit>`. Keep the "0 means clear" convention in mind. |
| `SCAN pos, angles, vec(hfov, vfov, range)` | ~30 | Fires `EVENT_SCAN` (or `EVENT_DETECT` for `enable_detect`) on every entity within range and inside the cone (360/360 = sphere), `YOU` = scanner [I]. Used as broadcast (ScanIdentite). | Spatial query (brute force is fine: entity counts are about 100-300 per level) and then Bevy events (§3.2). |
| `ENT_VERTEX(vec, n)` | ~12 | World position of mesh vertex n (heads for muzzle traces `lion.wdl`, particle spawn points `mm1.wdl:4360`) [I]. | Read from the current interpolated MDL frame. |
| `CONTENT(pos)` | 3 | Solid test. | Point-in-mesh/BSP. |
| `EVENT_TYPE` constants `EVENT_SCAN, _DETECT, _ENTITY, _BLOCK, _IMPACT, _STUCK, _SONAR, _TRIGGER, _TOUCH, _CLICK, _RELEASE` | 41 tests | Which event fired. `_TRIGGER` = another entity entered `trigger_range`. `_TOUCH/_CLICK/_RELEASE` = mouse over/click on an entity (menu). | Separate Bevy event types. |
| `YOU, MY, PLAYER, TARGET, NORMAL, BOUNCE, RESULT` | 6192 MY, 570 YOU, 61 RESULT | Registers. `PLAYER` is not used; the scripts use `Pov_Syn`/`Player_Syn`. | Explicit function arguments / event fields. |
| `my_pos, my_angle, model_pos, angles, temp, v…` | many | Script scratch globals (some engine predefined). | Locals. |

### 5.2 Entity lifecycle and rendering

| Built-in | Count | Semantics | Bevy |
|---|---|---|---|
| `CREATE <file>, pos, action` | ~85 | Spawn a model/sprite/WMB entity running `action`, with `YOU` = creator inside the new action [I] (`LaunchKey gems.wdl:693` reads `You.entity1`). `+N` in a sprite filename (`gemre+17.pcx`) = horizontal strip of N frames [R comment `particul.wdl:620`]. | `commands.spawn(bundle_for(action, creator))` |
| `REMOVE ent` | ~44 | Despawn. | `despawn_recursive` |
| `MORPH <file>, ent` | ~20 | Swap the entity's model/WMB in place (doors `doordark.wmb`, weak door damage stages `doorwk2-4.wmb`, platforms `platfor1.wmb`, markers `small.pcx`) [R]. | Replace `Handle<Mesh>`/material plus collider. |
| `SET_CYCLE ent, "scene", pct` / `SET_FRAME …` / `ENT_CYCLE` | 41 / 17 / 1 | Pose the model at `pct`% (0-100) through the named frame sequence ("walkB", "sautA"…). Cycle = loops with interpolation back to start; frame = clamp [I]. | MDL animation component: `scene` maps to a frame range from the MDL frame names (e.g. `walkB1..walkB12`), sampled at pct with vertex lerp. |
| `ENT_ALPHASET(frame, alpha)` | 18 | Set alpha for transparent sprites [I]. | Material alpha. |
| `ENT_PRELOAD, ENT_PURGE` | 1 / 1 | Texture cache management. | ignore |
| `EMIT n, pos, particle_action` | ~185 | Spawn n particles. Each runs `particle_action` every frame with `MY_AGE, MY_SIZE, MY_SPEED, MY_COLOR, MY_MAP, MY_FLARE, MY_ACTION, MY_POS`. Particles move by `MY_SPEED` per frame [I]. Set `MY_ACTION=NULL` to kill. | CPU particle pool (§3.3e). |
| `TO_ANGLE`, `VECROTATE` | see §2 | | |

### 5.3 Sound and music

| Built-in | Count | Semantics | Bevy |
|---|---|---|---|
| `PLAY_SOUND snd, vol` | ~50 | 2D sound, volume 0-100. Returns a handle in `RESULT` [I]. | `AudioPlayer` + `PlaybackSettings::DESPAWN.with_volume(vol/100)` |
| `PLAY_ENTSOUND ent, snd, range` | ~90 | 3D positional sound. The third argument (100-3000) is the **audible range in quants** [I], not volume. | `SpatialAudioSink`, with attenuation based on `range`. |
| `PLAY_LOOP snd, vol` / `STOP_SOUND h` / `TUNE_SOUND h, vol, freq` | 1 / 5 / 10 | Looping 2D sound, stop, retune volume/pitch (slide sound `player.wdl:1670-1710`; lion roar pitch `200*Weight+random(50)-25` = frequency % [I]). | `AudioSink::set_volume/set_speed` |
| `PLAY_SONG mid, vol` / `PLAY_SONG_ONCE` | 14 / 1 | MIDI music, looping / once. | Pre-render the `sammy0N.mid` files to OGG with a GM soundfont and play them as looping music (or use a MIDI synth crate such as `rustysynth`). |
| `MIDI_VOL, SOUND_VOL, MIDI_PLAYING` | 2/2/2 | Global volumes 0-100, music playing flag. | Global volume resource. |
| `SOUND name,<wav>` / `MUSIC name,<mid>` | 53 / 9 | Asset declarations (`sons.wdl`). | Asset handles resource. |

### 5.4 Screen, UI, input

| Built-in | Count | Semantics | Bevy |
|---|---|---|---|
| `BMAP name,<pcx>[,x,y,w,h]` | ~192 | Bitmap asset. | `Handle<Image>` (PCX converted to PNG in `assets/images`). |
| `FONT name,<pcx>,w,h` | ~11 | Bitmap font: a PCX grid of fixed-size cells (`fontes.wdl`) [R]. | Custom bitmap-font text renderer (glyph atlas, fixed cell). |
| `PANEL { pos_x; pos_y; layer; bmap; flags refresh,overlay,transparent,d3d; DIGITS x,y,len,font,factor,var; WINDOW x,y,w,h,bmap,varx,vary; BUTTON x,y,bmapOn,bmapOff,bmapOver,fnClick,fnLeave,fnOver; HSLIDER x,y,len,knob,min,max,var; HBAR …; }` | ~71 | 2D overlay panel. `DIGITS` renders `var*factor` as an integer (factor 0.051 extracts the tens digit, `panels.wdl` PanLifeNumber). `WINDOW` shows a w×h sub-rectangle of a bitmap scrolled by var (bars and gem icon strip). `.visible` toggles it; `layer` = z-order [R/I]. | Bevy UI `Node`s with absolute positioning, `ImageNode` with `rect` for WINDOW, custom digit text, `Button` + `Interaction`. |
| `TEXT { pos_x; pos_y; font; string; flags center_x,center_y,narrow; layer; }` | ~33 | Text overlay. | `Text` node using the bitmap-font renderer. |
| `ENTITY name { type <mdl>; view CAMERA; x; y; z; layer; }` (view entities) | 3 | 3D model drawn in camera space as a HUD element (`PanWasp`, `PanEye`, `PanBlack` fade quad, `panels.wdl:409-448`) [R]. | Second camera with `RenderLayers` (HUD 3D), or a mesh parented to the camera. |
| `ON_ESC, ON_F1, ON_F5, ON_F10, ON_SPACE, ON_CTRL, ON_ALT, ON_ENTER, ON_CUU, ON_CUD, ON_MOUSE_LEFT, ON_JOY1..4, ON_A…ON_X, ON_TAB` | ~60 | Key-down callbacks. | input systems |
| `KEY_FORCE.x/.y` | 15 | Cursor-key axes -1..1. **Doubled while Shift is held** [I, required by the boost maths in `player.wdl:1763-1798`]. | Build the same axis value yourself: arrows → ±1, ×2 when Shift. |
| `JOY_FORCE`, `JOY_1..4` | 4 / ~20 | Joystick axes/buttons. | `Gamepad` |
| `KEY_ANY, KEY_ESC, KEY_ENTER, KEY_SHIFT, KEY_CTRL, KEY_ALT, KEY_SPACE, KEY_x` | ~80 | Polled key states. | `ButtonInput<KeyCode>::pressed` |
| `MOUSE_MODE, MOUSE_MAP, MOUSE_POS, MOUSE_LEFT/RIGHT, MOUSE_RANGE, MOUSE_SPOT, MICKEY, POINTER` | 14/1/43/16+7/1/1/6/2 | Software cursor (arrow.pcx), position, buttons, pick range for `EVENT_CLICK` on 3D entities, hot-spot, raw mouse delta, OS cursor position. | Custom cursor sprite, `Window::cursor_position`, `MeshPickingPlugin` (bevy_picking) for 3D menu items. |
| `FREEZE_MODE` | 6 | 1 pauses all entity actions (options overlay) [I]. | `Time<Virtual>::pause()` or a `Paused` state gating `FixedUpdate`. |
| `SWITCH_VIDEO mode,depth,window`, `VIDEO_MODE`, `VIDEO_DEPTH`, `SCREEN_WIDTH/HGT` | 23 | Resolution change (modes 1-9 = 320×200 … 1280×960, `interfac.wdl:1114-1197`). | `Window.resolution`. HUD layout scales from an 800×600 design (`Ratio`). |
| `LOAD_LEVEL "x.wmb"` | ~54 | Unload everything and load a level. Entity actions start (one frame later) [I]. | `AppState::LevelLoading`: despawn the level root, spawn the parsed WMB. |
| `SAVE_INFO "name",n` / `LOAD_INFO "name",n` | 14 / 3 | Save/load **all SKILLs flagged for saving** to `name<n>.SAV` [I; see gameplay.md §Saves]. Scripts toggle the `Siger` variable around save so it isn't written. | `serde` struct to a file under the user data dir. |
| `EXIT "msg"`, `EXEC "file"` | 8 / 8 | Quit (with message) / run an external exe (registration, dead). | `AppExit` |
| `BEEP` | 3 | Debug beep. | ignore |

### 5.5 World and render globals

| Built-in | Count | Semantics | Bevy |
|---|---|---|---|
| `CAMERA.x/y/z/pan/tilt/roll/arc/ambient/fog/diameter` | 356 refs | The single view. `arc` = FOV (deg), `fog` = fog % (100 = fully fogged, used for fades), `diameter` = camera collision radius [I]. | `Camera3d` + `Projection::Perspective{fov}` + `DistanceFog`. |
| `FOG_COLOR` (index 1-4), `CLIP_RANGE`, `SUN_LIGHT`, `SUN_ANGLE`, `MIP_FLAT/SHADED`, `FLOOR_RANGE`, `RENDER_INFLATE`, `GAMMA`, `D3D_*`, `MAX_PARTICLES 2000`, `MAX_ENTITIES 1000` | | Fog colour palette index (`fog_color=4` used for black fades [I]), far clip, sun brightness/direction, mipmap bias, … | `DistanceFog` colour table, `Projection.far`, `DirectionalLight`. Ignore the D3D knobs. |
| `TIME, TIME_FAC` | 388 / 7 | Frame duration in ticks; time factor. | `Res<Tick>` |
| `SKY` entities (`Move_Sky`, `InitSkyMenu`) | n/a | Script-driven sky models following the camera (`mm1.wdl:3300-3459`). | Skybox meshes parented to the camera translation. |

### 5.6 Frame-rate dependence (important)
- The scripts mix time-scaled updates (`x += v*time`) with **per-frame constants**: camera target-Z easing `/4` per frame (`camera.wdl:444-446`), mode-2 turning `*0.25` (`player.wdl:172-177`), particle `My_Size -= 2` per frame, `My_Speed.z -= 0.08` per frame (`particul.wdl`). They also scale effects by measured `AverageFps` (50/40/35/30/24/20/16/12 thresholds; `adjustAngRotCam1 = 4/AverageFps`, `mm1.wdl:897`; `AdjustFpsCam3`, `mm1.wdl:848-856`).
- **Recommendation:** run at 16 Hz fixed ticks and apply each per-frame constant once per tick. Pretend `AverageFps` = a constant in the "high" tier (≥50), so every fps-gated effect is enabled. Where the game feels different from the original, compare against a recording at the original's typical 30-60 fps and retune individually. The camera follow constants are the most sensitive (gameplay.md §Camera).

---

## 6. Quirks to replicate or fix deliberately (summary)

| Quirk | Where | Recommendation |
|---|---|---|
| Exact float equality (`Scale_X==1.5`, `Int(GuardStuckBalle==8)`) | `gems.wdl:82,91`; `lion.wdl:1996,2125` | Gem blink: use a bool phase. Lion jitter: fire when the counter crosses 8. |
| Boolean inside `Abs`/`Int` | `lion.wdl:1319` | `if speed_x > 1` |
| `== ` used as a statement (no-op) | `gems.wdl:543,1074` | Keep as no-op. |
| Off-by-one editor skills | `mm1.wdl:1950` (RangeCall=Skill7), `mm1.wdl:2818-2824` (InitCamera2), `mm1.wdl:3379-3459` (sky rotation tests Skill5-7 applies 2-4), `mm1.wdl:2603` (Roll from Tilt) | **Replicate**: level data was tuned against the buggy behaviour. |
| DoorLight reads `V` but the caller sets `V1` | `mm1.wdl:3261` | Colour comes from stale `V` (usually the last GemLight colour). Use the door's key colour or no light; cosmetic. |
| SetAnimPlayer re-arms with the wrong event | `mm1.wdl:5522-5527` | Replicate only if the extro depends on it. |
| `Call` of yielding actions continuing in parallel | §3 | Model explicitly. |
| `particle_rouge` / `particle_Rouge` name clash | `particul.wdl:117, 476` | Case-insensitive duplicate. Which one A4 binds is unknown; check visually (gem-explosion trail colour). |
| Polish build defaults `language=1` (German) | `sammypolish.wdl:3` | Yes/No dialogs accept J/N only. Port: accept T/N/Y/J. |
| DRM / registration (`Initpublic`, `Initprotection`, `Singer`, `Siger`, `Protection1`) | `mm1.wdl:1981-2025` | Drop. Treat as registered (`Singer=1`), so bonus levels unlock per the rules in gameplay.md. |
| Debug and cheat keys always bound | `mm1.wdl:108-128` | Put them behind a `--cheats` flag. |

---

## 7. Suggested Rust crate / module breakdown (in dependency order)

Size estimates are lines of Rust excluding tests, assuming the WMB/MDL parsers are done by the other workstream.

| # | Module | Contents | Depends on | Est. LOC |
|---|---|---|---|---|
| 1 | `units` / `math` | Quant/tick constants, `Euler` (pan/tilt/roll), A4↔Bevy axis conversion, `ang()`, `to_angle()`, `vecrotate()`, `accel()`, seeded RNG wrapper. | n/a | 250 |
| 2 | `assets` | Loaders: WMB (other agent), MDL (vertex frames, named scenes, skins), PCX→texture, `+N` sprite strips, bitmap fonts, WAV, MIDI→OGG. Asset manifest from `sons.wdl`, `BIND`, `BMAP` lists. | 1 | 1500 (excluding WMB) |
| 3 | `tick` | `FixedUpdate` at 16 Hz, `Res<Tick{dt}>`, pause (`FREEZE_MODE`), debug single-step. | 1 | 100 |
| 4 | `collision` | Level collision mesh from WMB, kinematic mover (`MOVE` glide plus events, fat/narrow hulls, push priority), `sonar`, `trace` with ignore flags, `content`, `scan` query. Bevy events: `Blocked, EntityHit, Impacted, Stuck, SonarHit, ScanHit, Detect, Trigger`. | 1, 2 | 1200 |
| 5 | `level` | Level registry (49 slots → WMB file), loader that maps the entity-lump action name plus skills/flags to typed components (one `match` over ~120 action names), global level setup (InitLevelGlobal / InitLevelN defaults), level-parameter entities (InitGame..8, InitLionSkills1/2, InitCamera1-4, InitExtraBalles). | 2, 4 | 1500 |
| 6 | `state` | `GameState` resources (score, goal, lives `BallesJoueur1`, hearts, boost, keys, flags, EyeOfTruth, level progress `LevelsInfo`, MusicOrder), `AppState` enum, save/load (`BestSco0.SAV`, `level*.SAV`, `Ver*.SAV` equivalents). | 5 | 600 |
| 7 | `anim` | MDL vertex animation by scene name + percent (`set_cycle`/`set_frame`), cross-fade (lion `LionAnimPreviousStep`), sprite-strip frames. `AnimPlayer` and `AnimLion` state tables. | 2 | 700 |
| 8 | `fx` | Particle pool + ~45 live particle kinds, dynamic lights (`GemLight`, door light, flashes), billboard popups (`FlyingPCX`), sound helpers (2D/3D, tune), music. | 2, 3 | 1800 |
| 9 | `player` | Input → intention, HandleSpeedPlayer physics (rotation, forward, sonar snap, gravity/jump, slope slide, knockback, platform force), jump/FinJump, U-turn, boost, fire, damage/death/respawn, shadow. | 4, 6, 7, 8 | 1600 |
| 10 | `camera` | Types 1/2/3, CamFollowMe 4-ray visibility logic, emergency camera, LookAtTarget, camera zones (SetCamera/Surveillance), level-start sweep, death orbit. | 4, 9 | 1200 |
| 11 | `gems` | Gem pickup, colour mixing, eject/throw-back physics, gem→door flight, temporary/blinking gems, boulder emitters, reset on death. | 4, 6, 8, 9 | 1300 |
| 12 | `doors` | Coloured doors (InitDoor/movePortail/OuvrePortail), exit door score check, weak doors + planks, restart points, exit/death tiles, lava, teleports, trampolines, rotating/remove objects. | 4, 6, 8, 11 | 1300 |
| 13 | `platforms` | Platform engines, linked bodies, key-locked platforms, carry forces. | 4, 6 | 500 |
| 14 | `enemies` | Lion/scorpion/elephant AI (EnnemyBehavior 14 states, patrol paths, LOS, call-friends rings, flee, respawn), hit handling. | 4, 6, 7, 8 | 2000 |
| 15 | `wasps` | Ammo pickups, 7 projectile types (homing, climb-dive, remote-controlled type 6), hives, escaping wasps, wasp-enemy clone for extro. | 4, 6, 8, 14 | 1300 |
| 16 | `pickups` | Hearts, lives, boost, Eye of Truth, diamonds/Bonus3, Bonus5 + MdlBonus, triggers (StartEnnemy, SetPovSyn, SetAnimPlayer, SetFinExtro). | 6, 8 | 700 |
| 17 | `hud` | Lives/hearts, score/goal, gem icon, boost bar, ammo + 3D wasp, Eye panel, fades (PanBlack), resolution-relative layout. | 6, 2 | 800 |
| 18 | `menu` | 3D main menu scene (menu_greenpepper.wmb) with picking, level-select camera cuts, level icon states and tooltips, options overlay (sliders, keyboard/joypad, help pages, credits), Yes/No dialog, quit. | 5, 6, 10, 17 | 1500 |
| 19 | `flow` | Boot (CheckingSystem, load saves, MusicOrder), Intro2 cutscene, DebutPartie/StartintroGame loop, win/lose screens (Lose.mdl / win.mdl + Particle_Win), progression (ValideLevel/SearchNextLevel), Extro + credits. | all | 1500 |
| | **Total** | | | **about 21,000** |

Suggested build order for a vertical slice: 1 → 3 → 2 (MDL, PCX) → 4 → 5 (one tutorial level, e.g. `10001.wmb`) → 9 → 10 → 11 → 12 → 17. Then 14, 15, 16, 13 → 6 saves → 18 → 19.
