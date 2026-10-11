# Particles and particle-like effects (implementation spec)

This spec covers every `EMIT` effect of the original game, its emitters, and the sprite effects spawned by the same scripts. It replaces reading `extracted/particul.wdl` (2993 lines) plus the call sites. It refines `gameplay.md` §12 and `wdl-porting.md` §3.3(e).

Conventions follow the other docs: Acknex coordinates X forward, Y left, Z up, in quants. Time is in ticks (16/s, `time` ≈ 1 tick when a frame is exactly 1/16 s). Every fact cites `file:line` in `extracted/*.wdl` (CP1252, CRLF; the line numbers below are the file's own). **[R]** = read directly from the scripts. **[I]** = inferred. **[U]** = engine semantics not confirmed (verify against the Wine install in `/mnt/drive/Games/sammy-suricate`). **[Q]** = quirk or bug in the original. **[DEAD]** = cannot run with the shipped data.

`Random(x)` is uniform in [0, x) [I, `wdl-porting.md`]. All bitmaps named below exist as `.pcx` in `extracted/` (checked with a script: every `Bmap` in `particul.wdl:1-109` resolves, case-insensitively).

---

## 1. A4 particle semantics as used by these scripts

### 1.1 `EMIT n, pos, function`
- `EMIT n,pos,fn` creates `n` particles at the vector `pos`, each driven by the action `fn` (e.g. `bullets.wdl:414`, `gems.wdl:371`). `pos` is any vector: `MY.POS`, `My.Pos`, `model_pos`, `You.pos`, `Target` (`mm1.wdl:4148`, `camera.wdl:563`). `n` may be a non-integer expression (`gems.wdl:368-371`, `Emit V` with `V = averageFps*0.3`) [U: truncation].
- A particle function can itself `EMIT` (nested): `particle_GemExplose*` emits dots (`particul.wdl:1672`), `particle_Getlife` emits `particle_life` (`:2444`). Engine limit `MAX_PARTICLES 2000` (`wdl-porting.md:232`) [I].
- **The globals read at birth.** Several particle functions read globals set just before the `EMIT`: `V4`, `Temp.XYZ`, `SpeedParticle3/4`, `VitesseWaspExplode`, `VitesseFumee`, `TailleFumee4`, `SizePartExplo`, `V1`, `V2`, `TaillePoussiere1` (e.g. `gems.wdl:365-366`, `bullets.wdl:78-80`, `particul.wdl:1459`). This only works if the `MY_AGE == 0` branch runs **synchronously inside `EMIT`** (or at the latest before the emitting action yields) [I]. Port: copy these globals into the particle at spawn time (a per-emit parameter struct).

### 1.2 Particle-function variables seen in the scripts [R]
| Variable | Use | Notes |
|---|---|---|
| `MY_AGE` | read only | Compared as `MY_AGE == 0` (init), `Int(MY_AGE) == N` (bitmap swap), `MY_AGE >= N` (death). Age is in **ticks** [I]: lifetimes such as 10/30/40 only make sense as ticks, and `particle_Wasp*` frame times (2,3,4,5…16) are tuned for a 1-2 s effect. |
| `MY_SPEED.X/.Y/.Z` | read/write | Velocity vector; scripts add gravity to `.Z` (`-= 4*time`) and damp `.X/.Y` (`*= 0.9`). |
| `MY_SIZE` | read/write | 100-3000. Scale unit unknown [U]; see §1.5. |
| `MY_COLOR.RED/.GREEN/.BLUE` | write | 0-255. |
| `MY_MAP` | write | A `Bmap` (a `.pcx`). When never set, the particle has no bitmap [U: plain coloured dot]. |
| `MY_FLARE` | write (`Set My_Flare,1`) | Additive/translucent flag [U]. Every mapped particle sets it. |
| `MY_ACTION` | write (`SET MY_ACTION,NULL`) | Kills the particle (§1.4). |
| `MY_POS` | read (`Emit 1,My_pos,...`) | The particle's own position, used for child emission (`particul.wdl:1672,2441`). |

**Not used anywhere:** `MY_ALPHA`, `MY_TRANSPARENT`, `MY_VECTOR`, `MY_LIGHTMAP` (grep of `particul.wdl`). The effects fade only by shrinking `MY_SIZE`, by swapping to dimmer bitmaps, or by dying. A port does not need per-particle alpha.

### 1.3 Run model
- The particle function runs **every frame** from birth until `MY_ACTION` is nulled [I]. At birth `MY_AGE == 0` (the init branch). Scripts end the init branch with `END;` in most old definitions, so the per-frame code below the branch is **skipped on the birth call** (e.g. `particul.wdl:500`, `1407`, `1435`).
- There is no "switch to a second function" pattern: no particle sets `MY_ACTION` to another function (grep: every `MY_ACTION` write is `NULL`). Everything is one function with `IF (MY_AGE == 0)` init + per-frame update + age test.
- Because `MY_AGE` advances by the frame time (ticks), `Int(MY_AGE) == N` fires on several consecutive frames when fps > 16 (harmless; idempotent bitmap set) and can be skipped below 16 fps. **At a fixed 16 Hz port every `Int(age) == N` fires exactly once.**
- Position is advanced by the engine: `pos += MY_SPEED` (times `time`?) each frame [U]. `wdl-porting.md:192` assumes "per frame". The scripts apply gravity as `My_Speed.z -= k * time` (a per-tick acceleration), which suggests speeds are quants per tick [I]. Check against the Wine build. Default for the port: **speeds in quants/tick, applied once per 16 Hz tick** (as `crates/sammy/src/particles.rs` already does).

### 1.4 Death
- `SET MY_ACTION,NULL` after `MY_AGE >= L` (most particles), or after `MY_SIZE <= 1` / `<= 100` (shrinking particles: `particle_BalleTrainee*`, `particle_Boost1`, `particle_interface1`). Several functions also have a second, never-reached size guard.
- `particle_Teleport/Teleport2` die at size ≤ 100 *or* age ≥ 20/16 (`particul.wdl:2349-2355,2387-2392`).
- Nothing else kills particles (no collision, no floor test: they pass through geometry [I]).

### 1.5 Size and units
- `MY_SIZE` values: 100-150 (dots), 400-1500 (hearts/stars), 2000-3000 (shards, dust, wasp trails at birth). The bitmaps are tiny (8×8 stars, 16×16 smoke/hearts/wasp discs, 13×15..11×20 gem shards, 18×18..26×25 StarHit). Unit unknown [U]. `gameplay.md:803` suggests size/1000 world units [I]; `particles.rs` uses quants. Calibrate by screenshot against the Wine build; a starting guess is `quad_width_quants = size / 100` (wasp trail 3000 -> 30 quants, shard 2000-4000 -> 20-40) [I]. Sizes are relative, so one global factor suffices.
- `MY_COLOR` with `MY_MAP`: probably ignored or multiplied [U]. Evidence for "ignored when a bitmap is set": `particle_WaspGreen` sets a **yellow** colour (220+random(20), 220, 0) over a green bitmap (`particul.wdl:2067-2090`, copy-paste from Yellow), and `particle_WaspMagenta` and `particle_WaspGradient` share the colour (250,0,250) over different bitmaps. Port: ignore `MY_COLOR` whenever `MY_MAP` is set; use it only for map-less dots [I].
- Map-less particles (the "dots", `particle_rouge` etc.) are coloured squares/points of `MY_SIZE` [U].
- `MY_FLARE = 1`: render additive (black = transparent) [U]. The bitmaps have a black background (`nonblack` pixel counts in the table of §2.12).

### 1.6 Frame-rate dependence (important for a 16 Hz port)
Three different clocks appear:
1. **Ticks** (`MY_AGE`, `waitt n`, anything `* time`): exact at 16 Hz.
2. **Per-frame constants without `time`** (a particle function runs once per *frame*): `My_Size -= 2`, `My_Speed.z -= 0.08`, `My_Speed.X *= 0.9`, `My_Size -= 20`. At the original 30-60 fps these ran 2-4 times per tick. Table of them (per-tick equivalent = value × f, with f = frames per tick = fps/16):
   | Particle | Per-frame constant | f = 2.5 (40 fps) per tick |
   |---|---|---|
   | `particle_Poussiere1` | `My_Speed.z -= 0.08` (`:1445`) | -0.2/tick |
   | `particle_Explose{Rouge,Jaune,Bleu}` | `My_Size -= 2` while > 10 (`:1489-1490,1525-1526,1561-1562`) | -5/tick |
   | `particle_RangeLion1/2` | `My_Size -= 20` / `-= 30` (`:2827,2842`) | -50 / -75 per tick |
   | `particle_GemRestart` | `Speed.XY *= 0.9` (`:1899-1900`) | ×0.77 per tick |
   | `particle_TileDeath` | `Speed.XY *= 0.9` (`:1928-1929`) | ×0.77 per tick |
   | `particle_Teleport` / `Teleport2` | `Speed.XY *= 0.9` / `0.85` (`:2341-2342,2377-2378`) | ×0.77 / ×0.66 |
   The RangeLion numbers are a tuning clue: size 500 minus 20 per frame reaches ~0 over its 8-tick life only at f ≈ 3 (24 frames); size 700 minus 30 per frame reaches ~0 over 12 ticks at f ≈ 2. The author tuned at roughly **f = 2-3 (32-48 fps)** [I]. Use f = 2.5 unless the Wine build says otherwise.
3. **Frame counters.** `ticks1` is a **frame** counter (`skills.wdl:228` "compteur de frames", `mm1.wdl:893`: `ticks1 += 1` once per main-loop frame, reset to 0 every 64 frames at `mm1.wdl:904`). `ticks1 % N == 0` therefore means "every N frames", not ticks. How often it fires depends on how often the caller runs:
   - a caller that loops with `waitt 1` runs once per tick, so `ticks1 % N == 0` passes with probability about **1/N per tick** (it samples the counter once per tick; with f integer and N dividing f it can alias to always/never, but real fps jitter breaks this) [I];
   - a caller that loops with `wait 1` runs once per frame, so it fires f/N times per tick (e.g. Sammy's `Marche` loop, `player.wdl:254`; thrown gems/wasps `wait 1`, `gems.wdl:933,1509`, `bullets.wdl:1156`).
   Each emitter below states which case applies.
4. **`AverageFps` gates** (`mm1.wdl:895-905`: mean of `fps*16` over 64 frames, `fps` smoothed from `TIME_FAC`) turn emitters down at low frame rates. Port at a fixed rate and pretend `AverageFps` is in the top tier (≥ 50), as `wdl-porting.md:238` recommends. Wherever an `AverageFps` threshold exists, §3 lists the top-tier branch.

---

## 2. Particle functions (gameplay)

Birth values are the `MY_AGE == 0` branch. "Gravity" is the per-update change of `MY_SPEED.Z` (negative = falls). `rand(x)` = `Random(x)`. Life = ticks until `MY_ACTION = NULL`. Position is the `EMIT` position unless noted.

### 2.1 Dots: `particle_rouge/jaune/orange/Bleu/magenta/Vert` (`particul.wdl:117-274`)
Map-less coloured dots. Used as the gem-shard trail, the key-trail fallback, and the wasp-hive fallback.

```rust
// all six
vel = (rand(2)-1, rand(2)-1, rand(1)-4);   // z in [-4,-3): falls
life = 10;
// size / colour per variant
```
| Variant | Size | Colour | Re-rolled each frame? |
|---|---|---|---|
| rouge (`:117`) | 200 | (250,150,50) fixed | set every frame (outside the init branch), constant |
| jaune (`:144`) | 100+rand(50) once | (200+rand(50), 230+rand(20), rand(10)) | **yes**, colour flickers every frame (`:164-166`) |
| orange (`:170`) | 100+rand(50) | (200+rand(50), 130+rand(20), rand(10)) | yes |
| Bleu (`:197`) | 100+rand(50) | (rand(50), 100+rand(80), 230+rand(20)) | yes |
| magenta (`:222`) | 100+rand(50) | (230+rand(20), rand(20), 230+rand(20)) | yes |
| Vert (`:249`) | 100+rand(50) | (rand(10), 230+rand(20), rand(10)) | no (set in the init branch) |

No gravity, no size change, no `my_map`, no flare. **[Q] name clash:** `particle_rouge` (`:117`) and `particle_Rouge` (`:476`) are the same identifier (WDL is case-insensitive). Which definition A4 binds is unknown [U] (`wdl-porting.md:253`). The second one (`:476-505`) is a different effect:
```rust
// particle_Rouge, second definition (explose)
birth: vel = (rand(6)-3, rand(6)-3, rand(1)-4); size = 100+rand(50);
       colour = (220+rand(20), 60+rand(10), rand(10)); END;   // rest of function skipped at age 0
each frame (age>0): colour = (80+rand(20), 10+rand(10), rand(10));  // dark red
life = 20;
```
Used by name in: gem shard trail (`:1672`, GemExploseRouge), key trail fallback (`:1637,1645`), hive fallback (`:2779,2786`), win fallback (`:2981`). Safest port: pick the first definition (bright orange-red dot, life 10) and compare in the Wine build; if the red gem explosion trail looks dark-red and longer, switch to the second.

### 2.2 Star sprites: `particle_rougeBmp/jauneBmp/orangeBmp/BleuBmp/magentaBmp/VertBmp` (`:275-466`)
Used by the key/gem trail at fps > 35 and the Win/hive effects.
```rust
vel = (rand(6)-3, rand(6)-3, rand(1)-4);
size = rand(300) + TaillePartGems;        // TaillePartGems = 750 (skills.wdl:112) -> 750..1050
map = star_N[random(10) > 5 ? A : B];     // tistar{N}{a,b}.pcx, 8x8 / 7x7
flare = 1; life = 10;
```
| Variant | Map | Extra colour set |
|---|---|---|
| rouge | `SmallStar1A/1B` = `tistar1a/1b.pcx` (red) | none |
| jaune | `SmallStar2A/2B` (yellow) | none |
| orange | `SmallStar3A/3B` (orange) | none |
| Bleu | `SmallStar4A/4B` (blue) | (rand(50), 100+rand(80), 230+rand(20)) |
| magenta | `SmallStar5A/5B` (magenta) | (230+rand(20), rand(20), 230+rand(20)) |
| Vert | `SmallStar6A/6B` (green) | (rand(10), 230+rand(20), rand(10)) |

No gravity, constant size.

### 2.3 Gem shards: `particle_GemExplose{Rouge,Jaune,Orange,Bleu,Magenta,Vert}` (`:1659-1782`)
Gem destroyed (timed-out, crushed, delivered, or ejected key ended). One shard is a falling chunk that drops dots behind it.
```rust
// V4 = rand(5)+5 (gems.wdl:365), SizePartExplo = 2000 (gems.wdl:366), both set just before EMIT
birth: vel = (V4*(rand(6)-3), V4*(rand(6)-3), V4 + rand(30));
       size = rand(200) + SizePartExplo;      // 2000..2200
       map = fragGem{1..6}; flare = 1;
each frame: vel.z -= 4*time;                  // gravity 4/tick
            if int(age) % 2 == 0 { emit(1, pos, dot_<colour>) }   // every frame while int(age) even
life = 30;
```
Colour table (shard bitmap, trail dot):
| Skill6 key | Colour | Shard bitmap | Trail dot |
|---|---|---|---|
| 1 | red | `morgem1.pcx` (13×15) | `particle_rouge` (§2.1 clash) |
| 2 | yellow | `morgem2.pcx` (15×17) | `particle_jaune` |
| 3 | orange | `morgem3.pcx` (15×18) | `particle_orange` |
| 4 | blue | `morgem4.pcx` (15×18) | `particle_Bleu` |
| 5 | magenta | `morgem5.pcx` (11×20) | `particle_magenta` |
| 6 | green | `morgem6.pcx` (15×17) | `particle_Vert` |
(The comments `:36-41` "bleu / jaune" are stale; measured mean colours of the PCX files are red/yellow/orange/blue/magenta/green.) Colour `MY_COLOR` is never set for the shard, so the bitmap is shown untinted [I]. Dot emission is per frame (≈ f emissions per even tick) -> at 16 Hz emit 1 dot on each even-age tick [I].

### 2.4 Gem catch burst: `particle_GemCatch{Rouge,Jaune,Orange,Bleu,Magenta,Vert}` (`:1784-1887`)
Sammy catches a gem. `V4 = Random(2)+1` (`gems.wdl:396`).
```rust
birth: vel = (V4*(rand(2)-1), V4*(rand(2)-1), V4 + rand(20));   // rises 1..23 per tick
       size = rand(2000);  map = tistar{N}{a|b} (50/50); flare = 1;
each frame: vel.z -= 1*time;
            if size > 100 { size -= 50*time }
life = 30;
```
Maps: Rouge=`SmallStar1A/B`, Jaune=`2A/B`, Orange=`3A/B`, Bleu=`4A/B`, Magenta=`5A/B`, Vert=`6A/B`.

### 2.5 Gem-mix end burst: `particle_Explose{Rouge,Jaune,Bleu}` (`:1462-1568`)
Two source gems finish merging into the mixed colour (`gems.wdl:596-605`; only the three primaries emit). `SpeedParticle4 = (rand(10)+10,)×3` (`gems.wdl:596-598`), `TailleFumee4 = 1000` (`:601`). `NORMALIZE v,SpeedParticle4` normalises to length `SpeedParticle4` (the vector is read as its X) [U].
```rust
// Jaune/Bleu (same as Rouge except the init)
vel = SpeedParticle4 * (rand(5)-2.5) per axis; normalize(vel, SpeedParticle4.x);   // speed 10..20
size = TailleFumee4 + rand(50);            // 1000..1050 (Rouge: +rand(100))
colour: Jaune (230+rand(20), 230+rand(20), rand(20)); Bleu (rand(20), rand(20), 230+rand(20))
        Rouge (250, 50+rand(20), 50+rand(20)); Rouge speed = SpeedParticle4 + (rand(50)-25) per axis, then normalize
map = star{1,2,4}{A|B}; flare = 1;
each frame: if size > 10 { size -= 2 }          // per frame, no `time`
            vel.z -= 1.5*time;
life = 30;
```
Rouge uses the *random-biased* direction (`:1468-1470`); Jaune/Bleu uniform in the cube.

### 2.6 `particle_ExploseDiam` (`:1570-1602`)
Diamond bonus (`Bonus3_Hit`).
```rust
vel = normalize((rand(5)-2.5)x3, 10); size = 400+rand(100);
map = tistar0a|0b (50/50, white-ish); flare = 1;
v = 200+rand(50); colour = (v,v,v);
each frame: if size > 20 { size -= 30*time }
life = 8;
```

### 2.7 Dust: `particle_Poussiere1` (`:1417-1450`) + emitter `DustunderSammy` (`:1451-1461`)
```rust
birth: vel = (rand(6)-3, rand(6)-3, rand(2)+1 + VitesseRaq.x*0.05);   // VitesseRaq.x = Sammy forward speed
       size = rand(100)+400; map = fumeeC5; flare = 1; colour (180+rand(20),120+rand(20),rand(20));
each frame: age==20 -> map fumeeC7; age==30 -> fumeeC9;     // fumeeC5/7/9.pcx: 16x16 brown smoke, fading
            if size < TaillePoussiere1-10 { size += (TaillePoussiere1 - size)*0.2*time }   // grows to target
            vel.z -= 0.08;                                  // per frame (§1.6)
life = 40;
```
`TaillePoussiere1 = IntensitePoussiere * V2` (`:1459`), `IntensitePoussiere = 1` (`InitLvl.wdl:37`), `V2` = 2000-5000 per caller (§3.5). Global, so the value at birth is whatever the last caller set (the slope slide at `player.wdl:1594` does **not** set it [Q], it reuses the last). `DustunderSammy` returns immediately if `Video_depth == 8` (`:1453`) or `ticks1 % TicksPoussiere != 0` (`:1454`); spawn position `(x+rand(12)-6, y+rand(12)-6, BlobHautSol)` (the ground under Sammy), `EMIT IntensitePoussiere` (= 1) particles.

### 2.8 Restart marker: `particle_GemRestart` (`:1889-1907`)
```rust
birth: vel = (rand(40)-20, rand(40)-20, rand(10)+3); size = rand(2000);
       map = greenpr1|greenpr2 (50/50; 5x12 green); flare = 1;
each frame: vel.xy *= 0.9;  vel.z += 1*time;  if size > 100 { size -= 50*time }
life = 30;
```

### 2.9 Smoke: `particle_TileDeath` (`:1908-1937`)
Death tiles, lava, lose title.
```rust
birth: vel = (rand(64)-32, rand(64)-32, rand(10)+3); size = rand(1500); map = fumeeB1; flare = 1;
each frame: map by int(age): 4 fumeeB3, 6 fumeeB5, 7 fumee3, 10 fumee5, 12 fumee6, 15 fumee7, 17 fumee8, 19 fumee9;
            vel.xy *= 0.9; vel.z += 1*time; if size < 2500 { size += 50*time }
life = 22;
```
`fumeeB*` are orange-brown, `fumee*` grey, fading out (non-black pixels 220/256 -> 11/256).

### 2.10 Wasp trail: `particle_Wasp{Red,Yellow,Blue,Green,Magenta,Gradient,Blanc}` (`:1938-2230`)
One behaviour, 7 colour/bitmap variants. Spawns at the wasp's origin and stays put.
```rust
birth: vel = 0; size = 3000; map = WaspEf<C>1; flare = 1; // colour below, probably ignored (§1.5)
each frame: map frame by int(age): 2->3, 3->5, 4->7, 5->1, 6->3, 7->5, 8->7, 10->1, 12->3, 14->5, 16->7   (letter-index swap, see table)
            if size > 100 { size += (100-size)*0.2*time }       // 3000 -> ~100, 0.8^t decay
life = 18;
```
| Type | Letter | Bitmaps (frame idx -> file) | Script colour |
|---|---|---|---|
| Red | R | 1->`WaspEfR0`, 3->`R2`, 5->`R4`, 7->`R6` | (220+rand(20),0,0) |
| Yellow | Y | `WaspEfY0/Y2/Y4/Y6` | (220+rand(20),220,0) |
| Blue | B | `WaspEfB0/B2/B4/B6` | (0,0,250) |
| Green | G | `WaspEfG0/G2/G4/G6` | **(220+rand(20),220,0), yellow [Q]** |
| Magenta | M | `WaspEfM0/M2/M4/M6` | (250,0,250) |
| Gradient | X | `WaspEfX0/X2/X4/X6` (blue-grey) | (250,0,250) |
| Blanc | W | `WaspEfW0/W2/W4/W6` | (250,250,250) |
(`Bmap` aliases, `:67-109`: `WaspEfR1 = WaspEfR0.pcx`, `R3 = R2`, `R5 = R4`, `R7 = R6`; `WaspEfR9 = WaspEfR9.pcx` etc. are only used by commented lines `:1958-1964,...`.) Blanc uses `WaspefW*` spelling but the same bitmaps. All discs are 16×16 on black.

### 2.11 Wasp death burst: `particle_WaspExplode{Red,Yellow,Blue,Green,Magenta,Gradient,Blanc}` (`:2451-2738`)
Same frame sequence, size rule and life as §2.10, plus movement:
```rust
birth: vel = VitesseWaspExplode * rand(5) per axis;   // vector set by the emitter: rand(4)-2 (wasp death) or rand(20)-10 (hive)
       size = 1500; map = WaspEf<C>1; flare = 1;      // colours as §2.10 (Green again yellow)
each frame: frames/size as §2.10; life = 18;
```

### 2.12 Hit star: `particle_hit1` (`:2231-2265`)
Lion hits Sammy (and the dead wasp-hits-lion path).
```rust
birth: vel = normalize((rand(8)-4)x3, 10); size = 1500+rand(500); colour (250,0,0); map = StarHit1; flare = 1;
each frame: if int(age) % 4 == 0 { map = StarHit{1..5} random }      // StarHit1..5.pcx, 16..26 px, yellow/orange stars
            if size > 50 { size += (50-size)*0.1*time }
life = 16;
```
Bitmap table (mean non-black colour): StarHit1 (141,134,6) 18×18; StarHit2 (161,151,7) 25×26; StarHit3 (166,103,8); StarHit4 (164,133,81); StarHit5 (122,97,68).

### 2.13 `particle_Boost1` (`:2266-2288`)
```rust
birth: vel = (rand(2)-1, rand(2)-1, rand(4)+2); size = rand(250)+600; map = SmallStar1A; flare = 1; colour (230+rand(20),50+rand(50),rand(50));
each frame: size -= 60*time;
dies when size <= 100;      // ~ 8-12 ticks
```

### 2.14 `particle_SpeedySammy` (`:2289-2320`)
```rust
birth: vel = (rand(2)-1, rand(2)-1, rand(2)-4); size = 2000+rand(500); map = StarHit1; flare = 1; colour (250,0,0);
each frame: if int(age) % 6 == 0 { map = StarHit{1..5} random }
            if size > 50 { size += (50-size)*0.2*time }
life = 24;
```

### 2.15 Teleport: `particle_Teleport` (`:2321-2356`) and `particle_Teleport2` (`:2357-2393`)
```rust
// Teleport (tile idle sparkle)
birth: vel = (rand(64)-32, rand(64)-32, rand(10)+3); size = rand(1500)+1500; map = StarHit1; flare = 1;
each frame: if int(age) % 5 == 0 { map = StarHit{1..5} random }
            vel.xy *= 0.9; vel.z += 1.2*time;
            if size > 100 { size += (100-size)*0.1*time } else { die }
life = 20;
// Teleport2 (around Sammy while teleporting)
birth: vel = (rand(32)-16, rand(32)-16, rand(5)+3); size = rand(1500)+1000; map = StarHit1; flare = 1;
each frame: int(age) % 6 == 0 -> random StarHit map; vel.xy *= 0.85; vel.z += 1.1*time;
            same size rule; life = 16;
```

### 2.16 Hearts: `particle_life` (`:2394-2418`), `particle_Getlife` (`:2419-2450`), `particle_SmallHeart` (`:2796-2818`)
```rust
// life: the actual heart sprite
birth: vel = (rand(4)-2, rand(4)-2, -8); size = rand(500)+500; map = PartHearth1; flare = 1;
frames by age: 2->PartHearth2, 3->3, 4->4, 5->5, 6->6, 7->7, 8->4, 9->1;     // phearth1..7.pcx, 16x16 red->green
life = 10;
// Getlife: heart bonus pickup; emits a life heart every frame
birth: vel = (rand(30)-15, rand(30)-15, rand(15)-5); size = rand(1000)+500; map = PartHearth1; colour (200+rand(50), rand(20), rand(20)); flare = 1;
each frame: if size > 50 { size -= 5*time }; emit(1, my_pos, particle_life);
life = 20;
// SmallHeart: small heart pickup
birth: vel = normalize((rand(8)-4)x3, 10); size = 400+rand(400); map = PartHearth1; colour (250,0,0); flare = 1;
each frame: if size > 50 { size -= 30*time }
life = 12;
```
`PartHearth1` is dark red, `PartHearth7` is green (mean colours (156,0,0) ... (10,163,61)).

### 2.17 Eye of Truth: `particle_RangeLion1/2` (`:2819-2848`), `particle_TakeEyeTruth` (`:2849-2873`)
```rust
// RangeLion1: size 500, map peye1 (green 8x8), flare; each frame size -= 20 (per frame); life 8
// RangeLion2: size 700, map peye2 (yellow 8x8), flare; each frame size -= 30 (per frame); life 12
// TakeEyeTruth (pickup): vel = normalize((rand(8)-4)x3, 10); size = 400+rand(600); colour (250,0,0); flare;
//                        each frame: map = peye1 or peye2 (random each frame); if size > 50 { size -= 20*time }; life 16
```
No velocity for RangeLion (speed stays 0, `:2822` commented `MY_SPEED.Z`).

### 2.18 Menu/interface-only (skip, one line)
`particle_interface1` (`:1173`, `interfac.wdl:279`, menu sparkles: speed ±2 per axis, size 200-250 shrinking 10/tick, `SmallStar{0,2,4}A`, `My_Speed.Z += 0.1*time`, dies at size ≤ 1). Debug-only: `particle_test1/2/3` (`:507-569`; `camera.wdl:563` needs `totoskill1==999`, `:786` in a comment, `player.wdl:1634` in a comment).

### 2.19 Unused / dead particle functions (not referenced from any live path)
`particle_BalleTrainee1-7` (ball trail: size 2000 -250/tick, `PartHearth1-7`), `particle_fumee` (`:912`), `particle_fumee2` (`:1273`), `particle_explose3` (`:570`), `particle_explose4` (`:1308`), `particle_balle` (`:1354`), `particle_impact` (`:1385`), `particle_poussiere` (lowercase dust, `:1026`), `particle_JackPot1` (inside a comment `:1209-1246`), `particle_fade2` (comment `:2926-2958`), `particle_explose5` (undefined; its only call `mm1.wdl:4450` is commented). Emitters `TraineeBalle1/2`, `TraineeRocket1`, `EmitParticle1/2/3`, `InitFumee`/`Fumeemonte`/`NearFumee`, `InitBullesA/B/C`: no live caller (`ActionParticleBlob_Syn = ActionNulle` for `TypeJeu 25`, `mm1.wdl:1799-1807`; `ActionParticleBalle_Syn = EmitParticle2` at `InitLvl.wdl:56` but nothing calls it; no level object uses `InitFumee`: grep of `extracted/*.wdl` finds it only in `particul.wdl`). Agrees with `gameplay.md:840`.

Bitmap summary (mean non-black colour, size): `tistar0a/b` white-grey 7×8; `tistar1a/b` red; `2a/b` yellow; `3a/b` orange; `4a/b` blue; `5a/b` magenta; `6a/b` green (all 7-8 px); `fumee1/3..9` grey 16×16 fading (220→11 opaque pixels); `fumeeB1..B9` orange-brown; `fumeeC5/7/9` brown; `greenpr1/2` green 5×12; `peye1/2` green/yellow 8×8; `fumeC+9.pcx` is a 144×16 strip (9 frames) used only by the dead `Fumeemonte`.

---

## 3. Gameplay EMIT call sites

Position is the entity origin (`MY.POS`) unless noted. "p/tick" = particles per tick expected at the top fps tier. The line numbers cite the file's own lines (`grep -an`).

### 3.1 Wasps (ammo and enemies)
`IdWaspSpec` colour mapping: 1 = Red + Yellow, 2 = Red, 3 = Blue, 4 = Green, 5 = Magenta, 6 = Gradient, 7 = Blanc (`bullets.wdl:410-446`). Trail tiers: `AverageFps > 50 -> (v1,v2,v3) = (2,4,2)`; `> 30 -> (3,5,4)`; else `(4,8,8)` (`:401-409`).

| Site | Enclosing action / trigger | Emit | Rate |
|---|---|---|---|
| `bullets.wdl:410-446` | `Wasp` (`:348`): player-fired wasp, while flying. Loop is `waitt 1` (once per tick). | 1 `particle_Wasp<type>` at wasp origin | spec 1: Red when `ticks1 % V1 == 0` and Yellow when `ticks1 % V2 == 0`; specs 2-7: one when `ticks1 % V3 == 0`. Top tier -> p(Red)=1/2, p(Yellow)=1/4, others 1/2 per tick [I]. The wasp is invisible for its first moments (`:365-375`) but the trail still emits. |
| `bullets.wdl:507-546` (EMIT at `:517-543`) | `Wasp`, after the flight loop ends (`Wasptat == 1001`, i.e. hit something (`Wasp_Hit` events `:707,:745`) or lifetime 128 ticks / 200 for type 6 (`:212,:299,:465-470`)). Wasp invisible, death-tumble runs. | 1 `particle_WaspExplode<type>` per tick for 4 ticks (`WaspTemp1 = 4`, `-= time`, `waitt 1`, `:512-546`) = 4 particles | `VitesseWaspExplode = (rand(4)-2)x3` re-rolled each tick (`:513-515`). Spec 1 -> Yellow only; 2 Red; 3 Blue; 4 Green; 5 Magenta; 6 Gradient; 7 Blanc. |
| `bullets.wdl:550-576` (`Wasptat` 1002/1003) | `Wasp` death variants: `EMIT 5 GemExploseRouge` / `EMIT 100 particle_explose3` | **[DEAD]**: nothing sets `Wasptat` to 1002/1003 (grep: only 1001 set, `:470,:707,:745`; 1004 only inside a comment `:736`). Same for `bullets.wdl:577-602` (`particle_Hit1` 10/15/10/5/5 with `waitt 3,2,1,1`). |
| `bullets.wdl:80-108` | `InitWaspWed` (`:37`): a hive-emitted wasp pickup whose life ran out. Loop: `WaspTemp1 -= time`, fades alpha in its last 16 ticks (`:59-63`), then `WaspTemp1 = -1`, flag cleared. Pickups reach this via `LaunchNewWasp ... Branch InitWaspWed` (`:1179`). | 4 particles (1 per tick for 4 ticks `:76-114`), same `particle_WaspExplode<type>` mapping, `VitesseWaspExplode = (rand(4)-2)x3` | Only when `WaspTemp1` was positive (a countdown was running). Level-placed pickups have `WaspTemp1 = 0` and never explode. |
| `bullets.wdl:866-909` (EMIT `:875-906`) | `WaspEscape` (`:847`): when Sammy picks up a different wasp type, the old wasps (up to 15, `:749-806`) fly away for 80 ticks (`WaspTemp1 += time` until 80, `:853-917`) | trail as `Wasp`, scale ×0.5 | tiers `AverageFps > 30 -> (3,5,4)`, else `(4,8,8)`; `waitt 1` loop |
| `bullets.wdl:1142` | `LaunchNewWasp` (`:973`): a hive-emitted wasp flying (before it lands); `wait 1` loop. Condition `My.BlobVspeedOrg > 5` (`:1139`) | `Call Particle_WaspHive` (§3.2) | per frame |
| `bullets.wdl:1255-1272` | `InitHive` (`:1197`): the hive is exhausted (`Skill3` wasps emitted, `:1213,1243`) and "explodes" | per loop: 6 `particle_WaspExplode*` (Yellow, Red, Blue, Green, Magenta, Gradient), `VitesseWaspExplode = (rand(20)-10)x3`; loop `WaspTemp1 = 12`, `-= time`, `waitt 3`: **12 bursts, 3 ticks apart (36 ticks)** = 72 particles | hive origin |
| `bullets.wdl:1754-1790` | `WaspEnnemyFly` (`:1697`), extro enemy wasps (`IdWaspSpec` 101-106). `waitt 1` loop | trail as `Wasp`; 101 -> Red+Yellow, 102 Red, 103 Blue, 104 Green, 105 Magenta, 106 Gradient. Tiers `(2,4,2)` if `AverageFps > 40`, `(3,5,4)` if `> 24`, else `(4,8,8)` (`:1745-1753`) | extro only (`wasps.md` §10) |
| `bullets.wdl:1810-1821` | `WaspEnnemyFly`, `IdEnnemy == 99` ("You Win" wasp in the extro): picks a random mesh vertex (`ENt_Vertex (My_pos, int(random(172)))`, `:1817-1818`) and calls `Particle_Win` | see §3.3 | per tick |

Unused: `bullets.wdl:1262-1267` repeats at hive; `mm1.wdl:4698` (`Particle_WaspHive` call) is inside a comment (planks).

### 3.2 Gem colour trails (`Particle_GemKeys`, `Particle_WaspHive`)
`Particle_GemKeys` (`particul.wdl:1604-1657`) is called with the gem's colour in `My.Skill6` (1-6, 255 = all). Gates: `AverageFps < 20` -> nothing; `> 35` -> every 3 frames emit 3 star sprites (`Particle_<c>Bmp`), 255 -> 1 of each of the six colours; otherwise every 6 frames emit 3 dots (`Particle_<c>`). Skill6 = 3 maps to Orange. `Particle_WaspHive` (`:2739-2794`) is the same pattern for hive wasps: `AverageFps > 40`: every 2 frames; else every 4 frames, <20 nothing. Mapping by `Skill6`: 1 -> Jaune×1 + Orange×2; 2 Rouge×3; 3 Bleu×3; 4 Vert×3; 5 Magenta×3; 6 Vert+Rouge+Jaune ×1 each (`:2753-2776`; Bmp stars at high fps, dots at low).

Callers (all in `gems.wdl`, per frame or per tick as noted):
| Site | Action | When | Loop clock |
|---|---|---|---|
| `gems.wdl:571,593` | `AnimGemMix1` (`:527`) | the two source gems of a mix orbit in/out (spiral) | `waitt 1` -> p ≈ 1/3 per tick × 3 stars (top tier) |
| `gems.wdl:918` | `LaunchYouKey` (`:702`) | a thrown/ejected key gem while `BlobVspeedOrg > 10` | `wait 1`: f/3 events per tick |
| `gems.wdl:1101,1126,1201,1256` | `AnimGemKey` (`:1043`) | a gem flying to the door / orbiting Sammy | `waitt 1`: p ≈ 1/3 per tick |
| `gems.wdl:1506` | `LaunchNewKey` (`:1317`) | a newly ejected gem while `BlobVspeedOrg > 2` | `wait 1` |
| `bullets.wdl:1142` | `LaunchNewWasp` | hive-emitted wasp while rising (`BlobVspeedOrg > 5`) | `wait 1` |
Ported at 16 Hz: per active flying gem, roughly **1 event/tick = 3 dots/stars** keeps the look (the original at 50 fps gives 3×(f/3) ≈ 3/tick for the `wait 1` callers). `Skill6 == 255` (the win/lose title models set it, `mm1.wdl:5227,5329`, but they call `Particle_Win` / `TileDeath` instead, so it is unused).

### 3.3 `Particle_Win` (`particul.wdl:2963-2992`)
Emits the colour burst from `My_pos`: `AverageFps > 12`: every 3 frames, 1 star of each of the 6 colours (`Particle_*Bmp`); else every 6 frames, 1 dot of each of the 6 colours. Callers: `mm1.wdl:5270,5303` (`InitTItleWin`, the "You Win" `win.mdl` title that flies to the camera and idles; random vertex of the model each tick, `:5259,5293`; `waitt 1` loop) and `bullets.wdl:1821` (extro wasp 99). Rate ≈ p(1/3 per tick) × 6 stars ≈ 2 stars/tick [I].

### 3.4 Gems and bonuses
| Site | Action / trigger | Emit | Notes |
|---|---|---|---|
| `gems.wdl:371-391` | `GemExplose` (`:364`): called when (a) a timed gem's `Skill9` runs out (`GemWaitForPlayer` `:105`), (b) a lion crush scan (`ScanIdentite == 100`, `lion.wdl:1806`) hits a gem (`Key_Hit :227`), (c) a gem reaches the door (`AnimGemKey :1139`, `Skill19 <= 20`), (d) a decorative key finishes (`LaunchNewKey :1535`, `PlayerOldKey == 0`) | `Emit V`, `V = averageFps*0.3` (≈ 15 at 50 fps, ≈ 5 at 16 fps) shards `particle_GemExplose<colour>` (by `Skill6` 1-6) at the gem | `V4 = rand(5)+5` (5..10), `SizePartExplo = 2000`; plays `broken4` (`:367`) |
| `gems.wdl:399-449` | `GemCatch` (`:395`): called from `Key_Hit :282` when Sammy collects a gem | **25 stars of the colour** (`particle_GemCatch<colour>`): 5 now, +10 after `waitt 2`, +10 after `waitt 4` more; if `AverageFps < 40` only the first 5 (`:399-403`) | `V4 = rand(2)+1` at `:396` |
| `gems.wdl:603-605` | `AnimGemMix1` end (`:527`): after the two source gems merge | `Emit 70` `particle_Explose{Rouge|Jaune|Bleu}` by `Skill6` 1/2/4 (primary colours only, 3 emitting entities -> 210 particles) | `SpeedParticle4 = (rand(10)+10)x3`, `TailleFumee4 = 1000` (`:596-601`); then `waitt 16` and remove |
| `mm1.wdl:1277` | `Bonus3_Hit` (`:1240`): a diamond bonus is hit by a wasp (`Scanidentite == 1`) | `Emit 15` `particle_ExploseDiam` at the bonus | also spawns the "+50/+100" popup sprite (FlyingPCX) |
| `mm1.wdl:2704` | `BonusVie_Hit` (`:2681`): ammo bonus ("life") touched | 1 `particle_Getlife` every `waitt 2` while `LightRange` falls from `Skill3` to 64 at 40/tick (`:2696-2706`; `Set SizePartExplo,200` unused) | the number of emits = ceil((Skill3-64)/40) |
| `mm1.wdl:4794` | `SmallHeart_Hit` (`:4749`): heart pickup (when `WindowHearts < 48`) | `Emit 30` `particle_SmallHeart` at the heart | pickup sprite shrinks `*0.8` per frame after |
| `mm1.wdl:5045` | `EyeOfTruth_Hit` (`:5018`): eye pickup | `Emit 30` `particle_TakeEyeTruth` | |
| `mm1.wdl:5002-5006` | `InitEyeOfTruth` (`:4954`): idle loop `waitt 1` | `ticks1 % 8 == 0` -> p = 1/8 per tick: 1 particle at `(x,y,z) + rand(50)-25` each axis; 60% `RangeLion1` (`random(10) > 3`), 40% `RangeLion2` | |
| `mm1.wdl:4360-4362` | `InitBoost` (`:4315`) boost arrow, `waitt 1` loop | `ticks1 % 6 == 0 && AverageFps > 30 && Flag2 == 0`: 2 `particle_Boost1`, at mesh vertices 3 and 11 (`ent_vertex`) | p ≈ 1/6 per tick (×2) |
| `mm1.wdl:3814` | `InitRestart` (`:3771`): idle restart marker, `waitt 1` loop | `ticks1 % 4 == 0 && AverageFps > 30`: 1 `particle_GemRestart` at (x, y, z-100) | p ≈ 1/4 per tick |
| `mm1.wdl:3850-3857` | `RestartHit` (`:3821`) when Sammy activates the restart point | bursts of 25, 20 (after `waitt 2`), 15 (+4), 20 (+6) `particle_GemRestart` (only the first 25 if `AverageFps < 40`) | **[Q]** emitted at `my.pos` after `My.Z = -9000` was set (`:3846`), so the burst is far below the level: invisible in the original [I]. Keep only the idle trail |

### 3.5 Sammy
| Site | Action | Emit | Rate / notes |
|---|---|---|---|
| `anims.wdl:370` | `AnimPlayer` (`:352`), Walk animation state (called every frame from `Marche`, `player.wdl:246,254`) | `Call DustunderSammy` with `TicksPoussiere = 4`, `V2 = 3000` | 1 dust per 4 frames ≈ f/4 per tick (≈ 0.6-0.8 at 40-50 fps) |
| `anims.wdl:383` | `AnimPlayer`, Turn state | `TicksPoussiere = 8`, `V2 = 2000` | 1 per 8 frames ≈ 0.3-0.4/tick |
| `player.wdl:465` | `FinJump` (`:435`): landing animation, `waitt 1` loop | `TicksPoussiere = 1`, `V2 = 3000`: **1 dust every tick** while the jump-end anim runs (`PlayerAnimStep < 90`, ≤ 16 ticks) | |
| `player.wdl:485` | `FinJump`, hard landing (`ForceChute < -70`) | once, `TicksPoussiere = 1`, `V2 = 5000` | big puff |
| `player.wdl:495` | `FinJump`, moving landing (`PlayerIntention.x != 0`) | once, `TicksPoussiere = 2`, `V2 = 3000` (fires only when `ticks1` is even) | |
| `player.wdl:506` | `FinJump` end loop (≤ 24 ticks, `waitt 1`): `V2 = 3000`, `TicksPoussiere` left from above (1, or 2) | `Call DustunderSammy` each tick | 0.5-1 per tick |
| `player.wdl:1594` | `handlePente` (`:1490`): Sammy sliding on a steep slope (per frame) | `EMIT 1,My.POS,particle_Poussiere1` when `ticks1 % V == 0` and `video_depth > 8`; `V = 6`, or `max(int(24 / max(sonar_dist, 0.1)), 2)` if `ResultPente > 6` | `TaillePoussiere1` = leftover global [Q] |
| `player.wdl:1805` | `LessBoost` (`:1799`), called from `HandleBoostPlayer` (per frame) when Shift/Joy_3 is held and the boost bar is not empty | every 2 frames: 1 `particle_SpeedySammy` at `(x+rand(32)-16, y+rand(32)-16, z+rand(100))` | ≈ f/2 per tick (≈ 1.2-1.5/tick) |
| `Extro.wdl:314` | `SammyExtro` (`:274`), Shift boost in the extro | same as `LessBoost` | |
| `player.wdl:1034` | `FlyingHearth` (`:994`), the heart model flying off when Sammy loses hearts (`Create "lifeA.mdl"`, `player.wdl:769`), `waitt 1` loop | 1 `particle_life` at the heart every tick for its whole flight (`Skill12` counts down) | ≈ 1/tick |
| `mm1.wdl:4109` | `InitTeleport` (`:4099`): idle teleport tile (`Flag3 != 1`) `waitt 1` | `ticks1 % 4 == 0`: 1 `particle_Teleport` | p ≈ 1/4 per tick |
| `mm1.wdl:4148,4169,4203,4257,4273` | `Teleport_Hit` (`:4115`) around Sammy (`You.pos`) while teleporting | 5 once at start (`:4148`); then each loop iteration 3 (`:4169`, fade-out, `Alpha -= 10*time`), 2 (`:4203`, fade-in), and per tick 1 while `Alpha` ramps down (`-= 2*time`, `:4257`) and up (`+= 1.5*time`, `:4273`) | `particle_Teleport2` |
| `mm1.wdl:4033,4043` | `InitTileDeath` (`:4023`): death-tile idle (`Flag3 == 0`), loop `waitt 2` | 1 `particle_TileDeath` every 2 ticks per tile | many tiles -> cull by distance/view if needed (the original had a commented-out screen test `:4036-4042`) |
| `mm1.wdl:4878` | `InitLava` (`:4869`): lava idle (`Flag3 == 0`), loop `waitt 2` | `ticks1 % 9 == 0` -> p ≈ 1/9 per 2 ticks | `particle_TileDeath` |
| `mm1.wdl:5353,5387` | `InitTItleLoose` (`:5314`): "Lose" title model, per tick random vertex (`:5352,5386`) | 1 `particle_TileDeath` per tick | smoke from the Lose model |

### 3.6 Lions
| Site | Action / trigger | Emit |
|---|---|---|
| `lion.wdl:499` | `Lion_Hit` (`:313`): lion state 10 (swipe) with `LionAnimStep > 60` and `BaffeDonne == -999`: the swipe connects (`lion.wdl:491-506`) | `TailleFumee4 = 1000`, `EMIT 10 particle_Hit1` at the lion origin |
| `lion.wdl:510` | `Lion_Hit`: lion in state 2 (touching Sammy) switches to 10 | `EMIT 1 particle_Hit1` |
| `lion.wdl:628` | `Lion_Hit`, `EVENT_IMPACT`: Sammy impacts a lion in state 10 with `LionAnimStep > 60` | `EMIT 1 particle_Hit1` per impact event |
Lion **death** has no particles via `EMIT`. `particle_Gemrestart` at `lion.wdl:1973` is inside a comment; `ParticleEnnemyRAnge` (`lion.wdl:1511-1583`) is inside `/* ... */`. The enemy range indicator is the 3D ring model (`DisplayEnnemyRAnge`, `enemy-ai.md:867`), not particles. Player side (`player.wdl:558-583`) has no `EMIT` (lion hits are emitted from the lion's handler).

### 3.7 Not gameplay
`interfac.wdl:279` (`ObjetMenuSelect`: menu sparkles, `particle_interface1`); `camera.wdl:563,786` and `player.wdl:1519,1609,1634` (`particle_test*`: debug or commented).

### 3.8 Particle-like effects done with `Create`d sprites (not `EMIT`)
- **Dead:** bubbles (`InitBullesA/B/C`, `particul.wdl:618-686`, called only by commented `Create <bulA2+9.bmp>` / the dead `TraineeBalle2` `:1094-1124`); the smoke sprite `Fumeemonte` (`:954-1015`, `fumeC+9.pcx` 144×16) started by `InitFumee` (`:825-911`) in sprite mode: no level object uses `InitFumee` (§2.19).
- **Live popups made elsewhere** (not particles): `FlyingPCX` score popups from `Bonus3_Hit` (`mm1.wdl:1253-1272`); gem-mix source sprites `AnimGemMix1` (`gemre+17.pcx`...); the eye pickup shrink; hive wasps. None are started from `particul.wdl` actions.

---

## 4. Porting priority

Ordered by how often a player sees it and how much it carries the feel.

**Tier 1: constant, on-screen in most levels**
1. **Wasp trail** (`particle_Wasp*`, §2.10 / §3.1): every flying wasp, 7 colours, p ≈ 0.5/tick. The signature effect.
2. **Gem catch burst** (`GemCatch`, 25 stars, §2.4/§3.4): every gem pickup.
3. **Gem explode** (`GemExplose`, shards + dot trails, §2.3): timed-out, crushed and door-delivered gems.
4. **Gem flight trails** (`Particle_GemKeys`, stars/dots, §3.2): any thrown/flying/orbiting gem.
5. **Dust under Sammy** (`Poussiere1`, §2.7): walking, turning, landing (dense on landing). Always on screen.
6. **Wasp death burst** (`WaspExplode*`, §2.11): every wasp impact/expiry (4 particles).

**Tier 2: frequent, level-dependent**
7. **Hit stars** (`particle_hit1`): every lion hit on Sammy (10 particles).
8. **Gem-mix end burst** (`Explose{Rouge,Jaune,Bleu}`, 210 particles): each colour mix.
9. **Death-tile / lava smoke** (`TileDeath`): only levels with such tiles (many tiles; cull).
10. **Win title** (`Particle_Win` + `win.mdl`, §3.3): end of every successful level, and the extro "You Win" wasp. Highly visible but rare; cheap to add.
11. **Hive** (`Particle_WaspHive` trail, 12 × 6 burst at exhaustion): only hive levels.
12. **Boost trail** (`SpeedySammy`) and **boost arrows** (`Boost1`): boost use.
13. **Teleport sparkle** (`Teleport`, `Teleport2`).
14. **Restart marker idle** (`GemRestart`, only the idle trail; the activation burst is invisible [Q]).

**Tier 3: pickups and minor**
15. Diamond burst (`ExploseDiam`, 15), heart pickup (`SmallHeart`, 30), eye pickup (`TakeEyeTruth`, 30) and idle ring (`RangeLion1/2`), ammo-bonus (`Getlife`/`life`), lost-heart trail (`life` from `FlyingHearth`).

**Skip:** menu sparkles, `particle_test*`, ball trails, rocket smoke, bubbles, `InitFumee`, `EmitParticle*`, `particle_explose3/4/5`, JackPot, lion death/range (no particles), wasp `Wasptat` 1002-1004 branches. About 40% of `particul.wdl` is dead code.

**Implementation order for the port:** (a) a pool of {pos, vel, size, colour, map, flare, age} and a per-kind update at 16 Hz (`crates/sammy/src/particles.rs` already has this) with a `Behaviour` per family (21 families, below); (b) the wasp trail + death burst; (c) gem catch/explode/trail; (d) dust; (e) the rest.

---

## 5. Summary tables

**Distinct live particle behaviours (21):** dots (6 colour variants), star sprites (6), gem shards (6), gem catch (6), mix burst (3), diamond burst, dust, restart green, tile death smoke, wasp trail (7), wasp explode (7), hit star, boost star, speedy star, teleport (2 variants), heart (life), getlife, small heart, eye range (2 variants), eye take. **Emitter actions (4):** `Particle_GemKeys`, `Particle_WaspHive`, `Particle_Win`, `DustunderSammy`.

**Port defaults** (all [I]/[U], tune against the Wine build): fixed 16 Hz; speeds in quants/tick; gravity/acceleration constants taken as written (`*time` = per tick); per-frame constants multiplied by f = 2.5 (§1.6); `ticks1 % N` at `waitt 1` callers -> Bernoulli(1/N) per tick; `wait 1` callers -> f/N per tick; `AverageFps` branches use the top tier; colour ignored when a bitmap is set; flare = additive.

## 6. Port status

Implemented in `crates/sammy/src/particles.rs` (pool, 16 Hz step, nested/delayed emits, batched camera-facing quads, `SIZE_SCALE` = size/100 quants) and `crates/sammy/src/effects.rs` (all live behaviours of §2 and the emitters of §3). Approximations: dust from Sammy's state (walk f/4, turn f/8, landing 1/tick, hard landing 5000 puff); slope-slide dust and `FlyingHearth` hearts not emitted; boost-arrow stars at ±12 quants instead of mesh vertices 3/11; the "You Win" burst uses a random offset instead of a random vertex; idle emitters beyond 3000 quants of the camera target are skipped. `particle_rouge` uses the first definition (§2.1).
