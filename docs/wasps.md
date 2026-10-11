# Wasps: Sammy's ammo and weapon (implementation spec)

This spec covers everything about wasps (French *guêpes*): pickups, firing, flight, homing, hits, corpses, the remote-controlled wasp, hives, weak doors and planks, plus the target-side interfaces (lions, doors, bonus). It replaces reading `bullets.wdl`. Every rule cites `file:line` in the original scripts (`extracted/*.wdl`, CP1252). Line numbers are unchanged by `iconv`.

Legend: **[R]** read directly from the code. **[I]** inferred from usage or the engine. **[U]** uncertain WDL/engine semantics, so verify in the running game. **[Q]** quirk or bug in the original. **[DEAD]** code that cannot run with the shipped data.

Time conventions follow `docs/wdl-porting.md:21-22,46`:
- 1 tick = 1/16 s.
- `time` is the last frame's duration in ticks.
- `waitt n` yields for n ticks; `wait n` yields for n frames.
- `mintime = min(time,1)`.
- `accel v,a,f` means `v = v*(1-min(f*time,1)) + a*time` [I].
- Run the port at a fixed 16 Hz with `time = 1`. All pseudo-code below takes `t = time` (in ticks) as a parameter.
- Angles are A4 convention: pan is counter-clockwise from +X, positive tilt is nose-up.
- Distances are in quants.

---

## 0. Corrections to `gameplay.md` §9 / §5.4

| gameplay.md claim | Actual behaviour |
|---|---|
| §9.1 "Skill3 poison" | WED Skill3 is only kept for types 2-7 when **Skill7 ≠ 0** (`bullets.wdl:1332-1380`). Type 1/0 always gets 3 (`:1326-1331`). In the shipped data Skill7 is 0 on every pickup, so **the per-type table always wins** and the WED poison and scale are ignored. |
| §9.1 "skin = type" | Skin = Skill7 (`:1391`). The defaults set Skill7 = type for types 2-7, but **type 1 leaves Skill7 untouched (0)**, so it gets skin 0 [Q][U]. |
| §9.1 idle "slow spin" | Initial pan is forced to 0 and tilt to 20, then pan += 1°/tick (`:42-44,55`). The WED pan is lost. |
| §9.2 type 5 "climbs 3 ticks, levels 5, then dives" | Tilt +20°/tick for 3 ticks, hold 5 ticks, ease to 0 over 5 ticks, then dive at −0.2°/tick. **Homing is disabled until the ease phase ends** (`Skill11 <= 0`, `:658`). |
| §9.2 "Wasps also open Flag4 doors" | **[DEAD]** A door only reacts to a wasp *scan* (scan id 2) and only if the door's Skill7 == 2 (`mm1.wdl:3016-3023,3099-3102`). All 277 shipped doors have Skill7 = 25 and none has Flag4. Only scanning types (1/4/5/6) could ever do it. |
| §9.2 "corpse tumbles with death animation" | The tumble runs for every death, but the wasp is **invisible** except after a lion hit (`Wasptat` 1004, `:509,:552,:570` versus the commented-out line `:579`). |
| §9.3 "When hit by the player it emits toward him" | **[DEAD]** The hive is `passable` (`:1203`), so it never gets impact events. Even if it did, emission needs `You.Skill1 == hive.Skill5` (0 in the data). The player (3333) would only be bounced back. |
| §9.3 cap | The cap compares against the **player's ammo count `NbrAmmo1`**, not against live wasps (`:1221`). Emission is also unconditional while the player holds another type. |
| §9.3 "optional lifetime" | Emitted wasps get lifetime = hive Skill1 + bounce count. With the shipped hives (12/16), **a landed wasp fades about 1-2 s after landing** unless picked up. Pickup is only possible after landing [Q]. |
| §9.3 "Bug: fate 1001 ends action, `:1007`" | It is `:1013`. Fate 1000 also does not "stay": `WaspTemp1` was already set at `:1010`, so it fades after ~1000 ticks [Q]. |
| §9 BonusVie hit by wasps | **[DEAD]** BonusVie is passable with no `enable_impact`, and it rejects scans whose id ≠ 1 (`mm1.wdl:2640-2648,2682-2686`). |
| §5.4 "TirA then TirB at 20%/tick × factor" | Only **TirA** is scaled by the per-type factor. TirB is always 20 (`anims.wdl:440-445`). The wasp spawns **after TirA ends**. |
| (new) | **Type 6 (remote) never appears in shipped levels.** No pickup or hive gives it. Only the `Shift+X` cheat does (`mm1.wdl:3696`, which actually gives type 1). See §11. |

---

## 1. Entity slots, globals, constants

### 1.1 Skill aliases (wasp-related only)
The same SkillN slot carries different names per actor (`mm1.wdl:187-499`). Cross-actor writes go through these slots.

| Slot | Wasp name | Other aliases that matter | Meaning for wasps |
|---|---|---|---|
| Skill1 | `IdWasp` (`mm1.wdl:448`) | `IdPlayer` (:203), `IDEnnemy` (:312) | Identity. **9999 = fired wasp** (`bullets.wdl:217`). Pickups and hive wasps hold their type 1-7 here. Lions 8888, elephant 1111, Sammy 3333. |
| Skill2 | `SpeedWasp` (:450) | `SpeedBalle` (:315) | Fired: speed/tick. Pickup: **ammo count** (`:3`, `:1178`). Hive wasp in flight: horizontal speed. |
| Skill3 | n/a | n/a | Pickup: poison as read by `Ammo1_Hit` (`:131,:138`). It is rewritten to the effective poison at `:1390/:1470`. |
| Skill4 | n/a | `BlobVSpeedOrg` (:209) | Hive wasp: bounce speed. |
| Skill5/6/7/8 | n/a | n/a | WED parameters (see per-actor tables). |
| Skill9, 10, 11, 16 | n/a | n/a | Types 5/6: climb, hold and ease timers, dive rate (`:278-281,:295-298`). |
| Skill12 | `WaspWaspScan` (:455) | n/a | Homing scan range (0 = no homing). |
| Skill13 | `WaspSpeedTilt` (:457) | n/a | Homing tilt gain. |
| Skill14 | `SpeedWaspOrg` (:459) | n/a | Base speed. |
| Skill15 | `WaspDeltaPan` (:461) | n/a | Homing pan step. |
| Skill18 | `GuepeAnimStep` (:352) | `BlobHautSol` (:237) | Anim step (0-100). **Clobbered by the ground Z in hive flight** (`:1037`) [Q]. |
| Skill19 | `SpeedWaspReel` (:464) | n/a | Corpse move result (unused). |
| Skill20 | `WaspTemp1` (:467) | `PlayerDestPan` (:241) | Lifetime / timer. **It is also the hive-wasp bounce counter** (`:999,:1057`) [Q]. |
| Skill25-27 | `BounceWaspX/Y/Z` (:474-476) | n/a | Corpse velocity. |
| Skill28 | `GuepeAnimSpeed` (:373) | n/a | Anim speed in %/tick. |
| Skill29 | `FindEnnemy` (:478) | n/a | Fear-tagging cooldown (ticks). |
| Skill30 | `WaspLifeTime` (:479) | n/a | Pickup lifetime seed. |
| Skill31 | `Wasptat` (:483) | `LionEtat` (:381), `PlayerEtat` (:262) | Death kind: 1001-1004. |
| Skill37-39 | n/a | `PlayerSpeedX/Y/Z` (:276-278), `EnnemySpeedX/Y/Z` (:392-394) | Wasp: last frame's displacement `my_speed` (`:379-381`). **The lion reads this as the knockback source**. |
| Skill43 | `WaspPoison` (:406) | `playerKey` (:288), `EnnemyResistance` (:404) | Damage. |
| Skill44 | n/a | `BounceForce` (:297) | Knockback multiplier. |
| Skill47 | `IdWaspSpec` (:421) | `IdEnnemySpec` (:420) | Wasp type 1-7. |
| entity2 | n/a | n/a | Shooter (Sammy). Ignored by the wasp's own MOVE (`:204,:377`). |

Flags: Flag1 = alive/loop. Flag2 (pickup) = 1 skips the shrink on pickup (`:164`). Flag3 (pickup/hive wasp) = 0 gives overlay ("queen" look), 1 gives flare (`:1382-1394`). WMB entity flag bits 0..7 = FLAG1..8 (`docs/formats/wmb4.md:271`).

### 1.2 Globals (`skills.wdl`, `mm1.wdl`)
| Name | Default | Meaning |
|---|---|---|
| `NbrAmmo1` | 0 (`skills.wdl:351`) | Ammo count. |
| `WaspCurrentType` | 0 (`skills.wdl:755`) | Held type (0 = none). |
| `WaspCurrentPoison` | 0 (`skills.wdl:756`) | Poison of the held type. Set only on first pickup or a type switch. |
| `NbrWaspEject` | 0 (`skills.wdl:757`) | Escaping-wasp counter. |
| `NbrWasp6Fly` | 0 (`skills.wdl:758`) | Number of type-6 wasps alive (freezes the camera). |
| `CameraOnWasp6` | 0 (`skills.wdl:759`) | Ticks left before the camera returns to Sammy. |
| `Wasp6_Syn` | null (`skills.wdl:761`) | Last type-6 wasp. **Never cleared** [Q]. |
| `Skill5_syn` | n/a | Last fired wasp (`:320`). Unused elsewhere. |
| `ScanIdentite` | 0 (`skills.wdl:169`) | Id of the scan currently running. Sammy's pickup scan = 1, wasp scan = 2, door scan = 25 (`mm1.wdl:141-163`). |
| `Ralentit2` | 1 (`skills.wdl:217`) | Speed multiplier, effectively always 1. |
| `AccGravite` | 1.5 (`skills.wdl:117`) | Fall acceleration (hive wasps, planks). |
| `gravite` | −2 (`skills.wdl:98`) | Initial fall speed. |
| `HeightGem` | 25 (`skills.wdl:697`) | Hover height of a landed hive wasp. |
| `AverageFps` | 50 (`skills.wdl:437`) | Selects the particle-rate tier. |
| `ticks1` | n/a | Frame counter (`mm1.wdl:790`). |
| `RangeScanDoor` | 500 (`mm1.wdl:165`) | Door scan range. |
| `SpeedWasp1/2/3/5` | 32 / 100 / 25 (unused) / 36 (`mm1.wdl:96-99`) | Speed in quants per tick. |
| `GuepeAnimSpeedWalk/Run/Stand/Death` | 0.5 / 1.1 / 3 / 6 (`mm1.wdl:24-27`) | Animation speeds in % per tick. Anim names "stand", "walk", "run", "attack", "death" (`mm1.wdl:29-33`). |
| `PlayerAnimSpeedDebutTir/FinTir` | 20 / 20 (`mm1.wdl:46-47`) | Anims "TirA" and "TirB" (`:73-74`). States `EtatAnimDebutTir` = 5, `EtatAnimFinTir` = 6 (`:59-60`). |
| `DistMuzzle` | 32 (`skills.wdl:793`) | Enables the lion head-vertex ("muzzle") tracking. |
| `EnnemyFrightened` 0.05, `EnnemyLachete` 0.5 | (`skills.wdl:790-792`) | Lion fear decay (§6.3). |

### 1.3 Per-type tables

**Fired wasp** (`InitWasp`, `bullets.wdl:218-314`; speeds `mm1.wdl:96-99`):

| Type | Colour (light, `gems.wdl:460-466` via `:155-161`) | Skin | Scale | Speed q/tick | Scan range | Tilt gain | BounceForce | Lifetime ticks | Extra |
|---|---|---|---|---|---|---|---|---|---|
| 1 "queen" | orange (250,100,0) | 1 | 1.0 | 32 | 500 | 0.1 | 1 | 128 | overlay=1 |
| 2 | yellow (250,250,0) | 2 | 0.7 | 100 | none | n/a | 2 | 128 | n/a |
| 3 | blue (0,0,250) | 3 | 0.5 | 100 | none | n/a | 2 | 128 | 3-wasp fan |
| 4 | green (0,250,0) | 4 | 0.5 | 100 | 500 | 0.1 | 2 | 128 | 3-wasp fan, homing |
| 5 | magenta (250,0,250) | 5 | 0.7 | 36 | 700 | 0.3 | 3 | 128 | climb/dive: Skill9=3, Skill10=5, Skill11=5, Skill16=0.2 |
| 6 remote | red (250,0,0) | 6 | 1.0 | 38.4 (32×1.2) | 1000 | 0.2 | 2 | **200** (`:299`) | climb/dive 4/8/10/0.1, steerable, camera |
| 7 | white (150,150,150) | 7 | 0.5 | 100 | none | n/a | 2 | 128 | n/a |

Poison of a fired wasp = `WaspCurrentPoison` at fire time (`:211`).

**Pickup defaults** (`initSkinScaleWaspWed`, `:1324-1396`; applied only when Skill7 == 0, except type 1, which is always applied):

| Type | 1 (also 0) | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|
| Poison | 3 | 2 | 1 | 1 | 4 | 6 | 1 |
| Scale | WED (unchanged) | 0.8 | 0.65 | 0.65 | 0.8 | 0.8 | 0.65 |
| Skin (=Skill7) | Skill7 as is (0) [Q] | 2 | 3 | 4 | 5 | 6 | 7 |

**Hive-emitted wasp defaults** (`initSkinScaleWaspHive2`, `:1411-1476`; always applied):

| Type | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|
| Poison (→ Skill3) | 3 | 2 | 1 | 1 | 4 | **5** | 1 |
| Scale | unchanged | 1.0 | 0.8 | 0.8 | 1.0 | 1.0 | 0.8 |

**Particle names** (all in `particul.wdl`):

| Type | Flight trail (`bullets.wdl:410-447`) | Explosion (`:82-109`, `:517-544`) |
|---|---|---|
| 1 | `particle_WaspRed` every v1 frames + `particle_WaspYellow` every v2 frames | `particle_WaspExplodeYellow` |
| 2 | `particle_WaspRed` every v3 frames | `particle_WaspExplodeRed` |
| 3 | `particle_WaspBlue` every v3 frames | `particle_WaspExplodeBlue` |
| 4 | `particle_WaspGreen` every v3 frames | `particle_WaspExplodeGreen` |
| 5 | `particle_WaspMagenta` every v3 frames | `particle_WaspExplodeMagenta` |
| 6 | `particle_WaspGradient` every v3 frames | `particle_WaspExplodeGradient` |
| 7 | `particle_WaspBlanc` every v3 frames | `particle_WaspExplodeBlanc` |

- Rate tiers (by `ticks1 % v`): AverageFps > 50 gives (v1,v2,v3) = (2,4,2); > 30 gives (3,5,4); otherwise (4,8,8) (`:401-409`). Escaping wasps have only the last two tiers (`:866-869`).
- Port: emit at a fixed time rate, e.g. the >50 tier at 60 fps ≈ 30/s and 15/s.
- Other emitters: `particle_hit1` (lion-hit puff), `particle_GemExploseRouge` and `particle_explose3` (dead states), `Particle_WaspHive` (hive wasp in flight, colour by Skill6, `particul.wdl:2739`).

**Sounds** (`sons.wdl`):

| Sound | File | Used for |
|---|---|---|
| `Wasp_son` | wasp.wav (:23) | Launch, vol 1000 |
| `TakeWasp_son` | takewasp.wav (:24) | Pickup, 2000 |
| `clog3` | Clog3.wav (:14) | Pickup, 500 |
| `WaspKaputt_son` / `WaspKaputt2_son` / `WaspKaputt3_son` | WaspKapu / WaspKap2 / WaspKap3 .wav (:43-45) | Death (1500), hive bounce (500) |
| `Error1` | error1.wav (:11) | Fire with no ammo, `play_sound` vol 50 |
| `broken4` | broken4.wav (:17) | Dead state 1003 |
| `plank1_Son`, `Plank2_son` | plank1/2.wav (:53-54) | Weak-door damage, plank bounce |

Third argument of `play_EntSound` = volume/range as written [U].

---

## 2. Ammo pickup (`InitAmmo1`, `bullets.wdl:21-177`)

WED: model `wasp1.mdl`.
- Skill1 = type (0 means 1).
- Skill2 = ammo given.
- Skill3 = poison (see §0).
- Skill7 = skin (0 = defaults).
- Flag2 = no shrink.
- Flag3 = flare instead of overlay.

```rust
// init (InitAmmo :27-36, initSkinScaleWaspWed :1324-1396, InitWaspWed :37-49)
ambient = 80;                       // :29
poison = skill3;                    // :30
spec = int(skill1);                 // :1325
if spec == 0 || spec == 1 { spec = 1; skill1 = 1; poison = 3; }          // :1326-1331
else if skill7 == 0 { skill7 = spec; scale = TABLE_PICKUP[spec].scale; poison = TABLE_PICKUP[spec].poison; } // :1332-1380
overlay = !flag3; flare = flag3;    // :1382-1385, :1394
if poison == 0 { poison = 1; }      // :1386-1389
skill3 = poison; skin = skill7; anim_speed = 3 /*Stand*/; passable = true;  // :1390-1393
if skill6 == 0 { skill6 = 1; }      // :1395
enable_scan = true; enable_entity = true; event = Ammo1_Hit; flag1 = true;   // :38-41
roll = 0; tilt = 20; pan = 0;       // :42-44
anim_step = random(100);            // :45
if life_seed == 0 { life_seed = 1; } // :46-49 (Skill30, unused afterwards)
// timer = Skill20 = 0 for WED pickups → never expires

// per tick while flag1 (:50-71)
set_cycle("stand", anim_step); anim_step += anim_speed * t; if anim_step > 100 { anim_step = 0; }
pan += 1.0 * t;
if timer > 0 {                      // only hive-emitted pickups (§9)
    timer -= t;
    if timer < 16 { transparent = true; alpha = timer * 6.25; }   // linear fade-out, %
    if timer <= 0 { timer = -1; flag1 = false; }
}
waitt(1);
// after loop (:72-117)
if timer < 0 {                      // expired: puff
    invisible = true; timer = 4;
    while timer > 0 { VitesseWaspExplode = (random(4)-2, random(4)-2, random(4)-2);
                      emit(1, pos, EXPLODE[spec]); timer -= t; waitt(1); }
}
waitt(1); remove();
```

**Pickup trigger.** The only accepted trigger is an EVENT_SCAN while `ScanIdentite == 1` (`:122`). That scan is Sammy's per-frame bonus scan: a full sphere of radius = Sammy's WED Skill5 (`BlobZoneScan`; 64/80/100/104 in the levels) (`player.wdl:217-226`). Other events (entity contact, wasp scans with id 2, door scans with id 25) are ignored.

```rust
fn ammo1_hit(me) {                                   // :121-177
    if ScanIdentite != 1 { return; }
    me.event = None;                                 // :125 single use
    if WaspCurrentType != me.skill1 {
        if WaspCurrentType == 0 { WaspCurrentType = me.skill1; WaspCurrentPoison = me.skill3; }
        else {
            escape_old_wasps();                      // §2.1, uses the OLD type and the OLD NbrAmmo1
            NbrAmmo1 = 0;
            WaspCurrentType = me.skill1; WaspCurrentPoison = me.skill3;
            init_current_pan_wasp();                 // HUD skin/scale (panels.wdl:1411-1460)
        }
    }   // same type: poison NOT updated
    NbrAmmo1 += me.skill2;                           // :143 (no cap)
    init_panel_wasp_3d();                            // shows HUD wasp if hidden (panels.wdl:1397-1410)
    me.ambient = 100;
    waitt(1);
    play_ent_sound(me, clog3, 500); play_ent_sound(me, TakeWasp_son, 2000);   // :152-153
    gem_light(me, COLOUR[skill1]);                   // spawns a coroutine: light range 300 → shrinks 15*t per frame until ≤100, then 0 (gems.wdl:454-477)
    if !me.flag2 { while me.scale_x > 0.05 { me.scale *= 0.95; waitt(1); } }   // :164-173, ≈59 ticks from scale 1, not time-scaled
    me.lightrange = 0;
    me.flag1 = false;                                // idle loop exits → removed next tick
}
```
The idle loop keeps spinning during the shrink.

### 2.1 Escaping wasps (`EscapeOldWasps :749-810`, `WaspNEscape :811-845`, `WaspEscape :847-921`)

`n = min(NbrAmmo1, 15)` wasps of the old type are created at `Pov_Syn.pos` (Sammy). Each one runs:

```rust
spec = old_type; skin = old_type; scale = 0.5 (model scale ×0.5);   // :848-850
timer = 0; pan = random(360); tilt = 60 + random(40); speed = 20 + random(10); passable = true;
while timer < 80 {                                  // 80 ticks
    move_relative(me, vec3(speed * t, 0, 0));       // along pan/tilt
    pan += 20 * t;                                  // spiral upward
    trail particles (tiers >30 / else only);
    set_cycle("walk", anim_step); anim_step += anim_speed*t*speed;   // anim_speed is 0 → frozen frame [Q]
    timer += t; waitt(1);
}
remove();
```
They are purely cosmetic: passable, no events.

---

## 3. Firing (`PlayerTire`, `player.wdl:271-384`)

Bound to `on_space` and `on_joy2` (`InitLvl.wdl:219-220`). Each press starts one coroutine.

```rust
fn player_tire() {
    let pressed = if Joypad == 1 { Joy_2 != 0 } else { Key_space != 0 };   // :273-276
    if !pressed { return; }
    if PlayerMoveMode == 0 || PartieEnCours == 0 { return; }               // :278
    let p = player;
    if p.etat != 1 { return; }                                             // :281 (idle/walking only)
    if NbrAmmo1 <= 0 { play_sound(Error1, 50); return; }                   // :282
    p.etat = 2;                                                            // blocks jump/turn/fire (they need etat==1)
    p.anim_etat = DEBUT_TIR /*5*/; p.anim_step = 0;
    p.anim_speed = 20.0 * [1.0, 2.2, 1.5, 1.2, 1.2, 1.0, 3.0][WaspCurrentType-1];   // :287-314
    while p.anim_etat == DEBUT_TIR { waitt(1); }                           // :316
    // anims.wdl:436-447: each frame set_frame("TirA", step); step += speed*t; if step>100 { step=10; etat=FIN_TIR; speed=20 }
    // TirA ticks at t=1: type1/6: 6, type2: 3, type3: 4, type4/5: 5, type7: 2
    if p.etat == 3 { return; }                                             // died meanwhile
    let off = rotate_z(vec3(40, 0, 0), p.pan + 180);                       // :325-328 (pan only)
    let spawn = vec3(p.x + off.x, p.y + off.y, p.z + 50);                  // 40 behind, 50 up
    NbrAmmo1 -= 1;                                                         // :333
    if WaspCurrentType <= 2 || WaspCurrentType > 4 {
        create("wasp1.mdl", spawn, InitWasp1);                             // pan = Pov.pan
    }
    if WaspCurrentType == 3 || WaspCurrentType == 4 {                      // :339-354
        create("wasp1.mdl", spawn, InitWasp3_1);                           // pan = Pov.pan-5, push -4
        NbrAmmo1 -= 1;
        if NbrAmmo1 >= 0 {
            waitt(2); create("wasp1.mdl", spawn, InitWasp3_2);             // pan = Pov.pan,   push -5
            NbrAmmo1 -= 1;
            if NbrAmmo1 >= 0 { waitt(2); create("wasp1.mdl", spawn, InitWasp3_3); }  // pan+5, push -6
        }
    }   // net: min(ammo,3) wasps for 1 ammo each; spawn pos is NOT recomputed, Pov.pan IS re-read at each create
    if p.etat == 3 { return; }
    if NbrAmmo1 <= 0 {                                                     // :360-368
        NbrAmmo1 = 0; PanNbrAmmo1.visible = false; PanWasp.visible = false; WindowWasp = 0;
        WaspCurrentType = 0; WaspCurrentPoison = 0;
    }
    waitt(1);
    if p.etat == 3 { return; }
    while p.anim_etat == FIN_TIR { waitt(1); }  // TirB from 10 at 20/tick → 5 ticks, then stand (anims.wdl:449-460)
    if p.etat == 2 { p.etat = 1; }
}
```
- Bug path: with `NbrAmmo1 == 1` and type 3/4, the counter goes to −1. It is reset to 0 at `:362`.
- Does Sammy's model face +pan? If so, the wasp starts behind him and flies forward over him, because the shooter is ignored by the wasp's MOVE (§4.3). Verify visually [U].

---

## 4. Fired wasp

### 4.1 Init (`InitWasp1/3_x :181-202`, `InitWasp :203-343`)
```rust
// in InitWasp1/3_x: pan set as above; push = -4/-5/-6 for the fan
shooter = YOU;                               // creator = Sammy (:204)
if push == 0 { push = -3; }                  // :206-209
spec = WaspCurrentType; poison = WaspCurrentPoison; timer = 128;   // :210-212
invisible = true; tilt = Pov_Syn.tilt;       // :214-215
id = 9999;                                   // :217 (Skill1 — what all targets test)
apply TABLE_FIRED[spec]                      // :218-314 (scan range, tilt gain, skin, speed, bounce, scale;
                                             //  enable_detect for 1,4,5,6; type6: timer=200, NbrWasp6Fly+=1,
                                             //  CameraBouge=1, Wasp6_Syn=me)
speed = speed_org;  Skill5_syn = me;         // :318-320
anim_speed = 0.5 /*Walk*/; ambient = 70;     // :322-324
enable_block = true; enable_entity = true; event = Wasp_Hit;   // :326-330 (no enable_impact)
waitt(1); fat = false; narrow = false;       // :335-337 hull choice [U]: smallest hull
flag1 = true; play_ent_sound(me, Wasp_son, 1000);
waitt(1);                                    // events can already fire during these 2 ticks
→ Wasp loop
```

### 4.2 Flight, per tick (`Wasp :348-474`)
```rust
while flag1 {
    if spec == 5 || spec == 6 { handle_vspeed(); }                  // :352-358
    if spec == 6 { pan -= PlayerIntention.x * 20.0 * t; }           // :359 steering (§7)
    let step = vec3(speed * t * Ralentit2, 0, 0);                    // local +X, rotated by pan AND tilt [I]
    if invisible && dist(Pov_Syn.pos, pos) > 80 { invisible = false; }   // :365-372
    YOU = shooter;                                                    // :377 MOVE ignores YOU [I]
    move_relative(me, step);                                          // collisions → Wasp_Hit (4.3)
    last_disp = my_speed;                                             // :379-381 → PlayerSpeedX/Y/Z (Skill37-39)
    if scan_range != 0 {                                              // :387-397
        ScanIdentite = 2;
        scan(pos, cone(pan, tilt, h_fov=120, v_fov=120), range = scan_range);  // fires EVENT_DETECT synchronously
        ScanIdentite = 0;
    }
    trail particles (§1.3);
    set_cycle("walk", anim_step); anim_step += 0.5 * t * speed; if anim_step > 100 { anim_step = 0; }   // :451-455
    timer -= t; find_cd -= t;                                          // :465-466
    if timer <= 0 { flag1 = false; wasptat = 1001; }                  // :467-471 (8 s; type 6: 12.5 s)
    waitt(1);
}
→ death (§5)

fn handle_vspeed() {                                                 // :922-948, types 5 (3,5,5,0.2) / 6 (4,8,10,0.1)
    if s9 > 0 { s9 -= t; tilt += 20.0 * t; return; }                 // climb
    if s10 > 0 { s10 -= t; return; }                                  // hold
    if s11 > 0 && tilt > 1 {                                          // ease to level
        s11 -= t; tilt -= tilt * min(0.2 * t, 0.2);
        if s11 <= 0 { tilt = 0; }
        return;
    }
    tilt -= s16 * t;                                                  // slow dive, forever
}
```
- Homing is gated on `s11 <= 0` (§4.4). For types 1 and 4, s11 = 0 from the start.
- [Q] If tilt ≤ 1 when the ease phase starts, s11 never decrements and the wasp never homes. This needs a start tilt ≤ −59/−79 and should not happen.

### 4.3 Collisions (`Wasp_Hit :653-747`)
The wasp has `enable_block` + `enable_entity`, but no `enable_impact` and no `enable_scan`. So other movers never "hit" it, and other scans never see it.

| Event | Condition | Effect |
|---|---|---|
| EVENT_SCAN | n/a | ignored (`:654-657`) |
| EVENT_DETECT | `s11 <= 0` | homing / fear (§4.4) |
| EVENT_ENTITY | the wasp's MOVE touched a non-passable entity | `flag1 = 0; wasptat = 1001` (`:704-707`). The lion-damage code here is commented out (`:709-739`). Damage is applied by the *target's* EVENT_IMPACT handler (§6). |
| EVENT_BLOCK | the wasp's MOVE touched level geometry | `flag1 = 0; wasptat = 1001` (`:742-746`) |

- What counts as a hit: any blocking contact during MOVE.
- Passable entities are ignored. These include pickups, BonusVie, hives, escaping wasps and corpses.
- The shooter is ignored (`YOU = shooter` before MOVE) [I].
- Push semantics between wasps (fan push −4/−5/−6, Sammy and wasp −3, doors −2) are [U]. Port: wasps never collide with wasps or with Sammy, and do collide with lions, weak doors, other solid entities and the world.
- **Same-move double events:** when a wasp moves into a lion or a weak door, the wasp gets EVENT_ENTITY (1001) and the target gets EVENT_IMPACT. The lion's handler writes `wasp.wasptat = 1004` (`lion.wdl:581-582`). Which write lands last is [U]. Port: a lion hit gives 1004 (hit puff, visible corpse), everything else gives 1001.

### 4.4 Homing / target selection (`Wasp_Hit :658-702`)
The scan uses `ScanIdentite = 2`, a 120° × 120° cone centred on the wasp's pan/tilt, with range 500/700/1000 [I].
- EVENT_DETECT is raised on the wasp once for **every** entity in the cone that has `enable_scan`. The engine order is [U]. There is no line-of-sight test [U].
- Only `Skill1 == 8888` (lion family: lions, scorpions, etc.) or `1111` (elephant) react. Dead lions have Skill1 = 0 and `enable_scan = 0` (`lion.wdl:648`, `anims.wdl:175-176`).

```rust
on_detect(target) {
    if spec_s11 > 0 { return; }
    if target.skill1 != 8888 && target.skill1 != 1111 { return; }
    if target.ennemy_attacked <= 0 && find_cd <= 0 {                 // :663-667 fear tagging
        find_cd = 16;
        target.ennemy_attacked += poison;                             // → lion fear (§6.3)
    }
    let shadow = target.entity1;                                      // lion's shadow sprite stores the muzzle
    let aim = if shadow.muzzle_x != 0 { vec3(shadow.muzzle_x, shadow.muzzle_y, shadow.muzzle_z - 32) }   // :676-683
              else { target.pos + vec3(0, 0, 32) };                   // :684-689
    let (dpan, dtilt) = to_angle(aim - pos);
    pan  += ang(dpan - pan) * 0.4 * t;                                // :694-696
    tilt += ang(dtilt - tilt) * tilt_gain * t;                        // :697 (0.1 / 0.3 / 0.2)
}
```
- The muzzle is the world position of the lion's head vertex (`VertexHeadNumber` 109/135/132/139/209 per model). The lion updates it every tick into its shadow entity's Skill3-5 (`lion.wdl:1122-1129`). In a port, expose `Lion.head_world_pos: Option<Vec3>`.
- If several targets are in the cone, each applies one steering step in the same tick. Port: iterate targets in ascending distance (deterministic) [U on the original order].

---

## 5. Death and corpse (`Wasp :482-651`)

```rust
event = None; passable = true; enable_impact = enable_entity = enable_detect = false;   // :482-487
if spec == 6 { NbrWasp6Fly -= 1; }                                   // :489-492
play_ent_sound(me, [WaspKaputt_son, WaspKaputt2_son, WaspKaputt3_son][int(random(3))], 1500);   // :494-506
match wasptat {
  1001 => { invisible = true; timer = 4;                             // timeout / wall / non-lion entity
            while timer > 0 { VitesseWaspExplode = (random(4)-2, random(4)-2, random(4)-2);
                              emit(1, pos, EXPLODE[spec]); timer -= t; waitt(1); } }   // :507-549
  1002 => { /* [DEAD] never set: invisible; v4 = (±(random(4)+4), ±(random(4)+4), random(10)+30)
              with sign flips if random(10)>5; SizePartExplo = 200*scale_x; emit(5, particle_GemExploseRouge) */ } // :550-567
  1003 => { /* [DEAD] invisible; SpeedParticle3 = (random(20)-10)×3; sound broken4 1000; emit(100, particle_explose3) */ } // :568-576
  1004 => { // lion hit: stays VISIBLE
            TailleFumee4 = 1000; emit(10, pos, particle_hit1); waitt(3);
            play_ent_sound(random kaputt 1..3, 1500);                 // second death sound
            emit(15, particle_hit1); waitt(2); emit(10, particle_hit1); waitt(1);
            emit(5, particle_hit1); waitt(1); emit(5, particle_hit1); }   // :577-604
}
// corpse recoil (:606-644): goes BACKWARD along the last flight displacement
bounce = vec3(-last_disp.x * 0.7, -last_disp.y * 0.7, -last_disp.z * 0.5);
anim_speed = 6 /*Death*/; anim_step = 0;
while anim_step < 99 {                                               // ≈17 ticks at t=1
    set_cycle("death", anim_step); anim_step += 6 * t;
    if bounce.x != 0 || bounce.y != 0 {
        let mut v = bounce;
        if v.x.abs() > 50 || v.y.abs() > 50 { v = v.normalize() * 50; }   // :623-626 (3D length)
        move_absolute(me, v);                // events off, still blocked by geometry [I]; passable to entities
        v *= 1 - min(0.05 * t, 1);            // accel v,0,0.05
        bounce = v; bounce.z -= 0.5 * t;      // gravity
    }
    waitt(1);
}
invisible = true; waitt(4); remove();
```
- `last_disp` is a per-frame displacement (`my_speed`), not a per-tick velocity [U]. At 16 Hz they are the same.
- For type 2 that is ≈100, so the recoil is clamped to 50.

---

## 6. Effects on targets (target-side interface)

A port should emit one event from the wasp and let each target consume it:
```rust
struct WaspHit { wasp: Entity, poison: f32, bounce_force: f32 /*1,2,3*/,
                 wasp_pan: f32, wasp_disp: Vec3 /*last frame displacement*/, kind: HitKind /*WaspMovedIntoTarget | TargetMovedIntoWasp*/ }
struct WaspScare { target: Entity, poison: f32 }   // from §4.4
```

### 6.1 Lions (`Lion_Hit`, `lion.wdl:313-643`). Only `IDEnnemy == 8888` takes damage.
Two paths with near-identical damage:
- **EVENT_IMPACT**: the wasp moved into the lion (`:531-641`). This is the normal case.
- **EVENT_ENTITY**: the lion moved into the wasp (`:385-529`).

```rust
// common pre-step, both paths: guard_stuck += ball_accel * t   (:388, :534)
// knockback (wasp BounceForce != 0 always):
lion.bounce_ball = vec3(wasp_disp.x * lion.weight_force * wasp.bounce_force,
                        wasp_disp.y * lion.weight_force * wasp.bounce_force, 0);   // :398-400, :537-539
// IMPACT path: lion.dest_pan = wasp_pan (:540,:554).
// ENTITY path: dest_pan = to_angle(contact normal or bounce) + random(10)-5, only if |BallDeltaPan| < DeltaDestPanLion (:389-403,:431-434)
// patrol: if flag3 && (ENTITY: etat != 2 &&) random_time <= 0 → random_time = TimeForNothing      (:436-439, :556-559)
if lion.id == 8888 && wasp.skill1 == 9999 {
    // IMPACT only: first hit while waiting (etat 0 or 4) switches to the secondary ranges (:567-579):
    //   if shadow.AttackRangeBis != 0 { AttackRange = AttackRangeBis; AttackRangeBis = 0 }
    //   v = int(frc(ReactRange)*10000 + 0.5); if v != 0 { ReactRange = v }
    wasp.flag1 = 0; wasp.wasptat = 1004;                           // :445-446, :581-582
    lion.resistance -= wasp.poison;                                 // :448, :583
    if lion.resistance <= 0 {
        if lion.etat != 6 { lion.pan = ang(lion.dest_pan - 180); anim_step = 0; anim_speed = 4 /*Death*/;
                            ball_vspeed = 20; ball_fric_vspeed = 3.5; etat = 6; }   // dies
    } else if lion.etat != 11 {
        ball_vspeed = 20 * weight_force; ball_fric_vspeed = 3.5;   // hop
        lion.ennemy_attacked += wasp.poison;                        // fear
        if lion.resistance < shadow.resistance_org * 0.95 {
            anim_step = 0; anim_speed = max(min(10 * resistance * 0.3, 20), 5); etat = 11;   // "Hit" reaction
        } else {
            lion.bounce_ball.xy *= 2.5 /*IMPACT*/ or 2.0 /*ENTITY*/;  // :477-478, :613-614
        }
    }
}
```
- Lion resistance comes from `frc(Skill1)*100` with per-species defaults; see the lion spec.
- The elephant (1111) gets the knockback but **no damage**. The wasp still dies with 1001 through its own EVENT_ENTITY.

### 6.2 Weak doors (`InitDoorWeak / DoorWeak_Hit / ExplodeDoorWeak`, `mm1.wdl:4466-4575`)
WED parameters:
- **HP** = Skill1, or Skill2 when Skill1 is 3333 or 0. In the latter case Skill1 becomes 90273, the "camera ghost" id (`:4468-4481`).
- push = −2, `enable_impact`, event `DoorWeak_Hit` (`:4482-4485`).
- In the shipped data HP equals the model: `doorwk1` = 4, `doorwk2` = 3, `doorwk3` = 2, `doorwk4` = 1.

```rust
on_impact(you) {
    if you.skill1 == 9999 { explode_door_weak(you); return; }            // fired wasps only (:4489-4492)
    if flag1 && (you.skill1 == 8888 || you.skill1 == 1111) {             // lions; flag1 = 0 on all shipped doors
        if weight_force <= 0.4 { explode_door_weak(you); return; }       // Skill45 default 0 → always [Q]
        if you.etat == 10 && you.anim_step > 60 && you.ennemy_attacked == -999 { random hit1..3 2000; explode_door_weak(you); return; }
        if you.etat == 2 { you.etat = 10; you.anim_step = 0; you.ennemy_attacked = -999; explode_door_weak(you); }
    }
}
fn explode_door_weak(you) {                                               // :4520-4575
    hp = int(hp - you.skill43);    // wasp: poison. Lion: its EnnemyResistance (same slot) [Q]
    match hp {
        1 => { sound(plank1_Son,1000); morph("DoorWk4.wmb"); return; }
        2 => { sound(plank1_Son,1000); morph("DoorWk3.wmb"); return; }
        3 => { sound(plank1_Son,1000); morph("DoorWk2.wmb"); return; }
        _ => {}   // ≤0 AND ≥4 both destroy [Q] (unreachable with HP ≤ 4)
    }
    invisible = true; passable = true; morph("small.pcx");     // the door is gone, silently
    PlayerSpeed = (40, 40, 30);                                // read by planks as YOU.*
    for batch in [[plank1, plank2], [plank3, plank1], [plank2, plank3]] {   // waitt(1) between batches
        for m in batch { create(m, pos + vec3(random(100)-50, random(100)-50, random(300)), LaunchPlank); }
    }
}
```

**Planks** (`LaunchPlank :4581-4707`, `plank_Bounce :4709-4715`). Each runs a per-*frame* (`wait 1`) loop:
```rust
passable = false; ambient = 80; event = plank_Bounce; enable_block = enable_entity = true;
bounces = 0; vz_org = 30; vz = 30; fric = 1.5; pan = random(360);
rot_tilt = 30*random(2) - 1; rot_roll = 30*random(2) - 1; tilt_dest = 90; still = 24;
v = (random(40) - 20, random(40) - 20); speed = (|v.x|+|v.y|)*0.5; push = -4; bounce_force = 1;
while flag1 && still > 0 {
    mt = min(t, 1); vstep = 0; ground = sonar_down(1000);
    if vz == 0 && z - ground > 1 { vz = -2; fric = 1.5; }
    let vsp = vz;                       // :4626 captured BEFORE the decrement (unlike hive wasps, :1054)
    if vz != 0 {
        vz -= fric * t;
        if z < ground + 1 {
            bounces += 1; sound(random plank1_Son/Plank2_son, 1500);
            if vz_org > 10 { vz_org *= 0.75; vz = vz_org - fric * t; vstep = vz * t /*accel f=1*/; }
            if speed <= 1 { vstep = ground - z; vz = 0; flag1 = false; }
        } else { vstep = vsp * t; }     // accel VerticalSpeed,Vspeed,1
    }
    move_absolute(me, vec3(mt * v.x, mt * v.y, vstep));
    v -= v * 0.05 * t; speed = (|my_speed.x| + |my_speed.y|) * 0.5;
    if speed < 1 { still -= t; speed = 1; v = (1, 1); }            // [Q] drifts at (1,1)
    else {
        if bounces <= 1 { tilt += rot_tilt * t; roll += rot_roll * t; }
        if bounces > 1 || speed < 7 { tilt += (90 - tilt) * 0.5 * t; roll -= roll * 0.5 * t; }  // settle flat
    }
    wait(1);
}
passable = true; waitt(16); remove();
// plank_Bounce: v = bounce.xy; v -= v * 0.5 * min(t, 0.99)
```

### 6.3 Lion fear (`lion.wdl:2128-2156`, for reference)
`EnnemyAttacked` (Skill19) decays by `0.05 * mintime * |resistance * weight * 0.5|` per tick. While it exceeds that threshold, a lion in state ≤ 1 switches to state 12 (flee), with `dest_pan = random(360)`. Both the scan tag (§4.4) and a non-lethal hit add `poison` to it.

### 6.4 Doors and BonusVie
- **Doors, Flag4 = "opens for wasps"** (`mm1.wdl:2881,3099-3102`). Doors only have `enable_scan`. A scan event is accepted only if `ScanIdentite == door.Skill7` (`:3016-3023`). The wasp scan id is 2, so a door needs **Skill7 = 2 and Flag4**. The opening is then the normal `OuvrePortail` rule (`:3114-3188`), including the horizontal-distance check `dist < Skill3 - 4` with Skill3 defaulting to 350 (`:2899,:3120`).
- **[DEAD]** in the shipped data: all 277 doors have Skill7 = 25 and none has Flag4.
- **BonusVie** (`mm1.wdl:2636-2715`): it is passable, so it cannot be hit. Scans with id ≠ 1 are rejected. Wasps never collect it **[DEAD]**.

---

## 7. Remote-controlled wasp (type 6)

- **Spawn** (`bullets.wdl:284-303`):
  - `NbrWasp6Fly += 1`, `CameraBouge = 1`, `Wasp6_Syn = me`, lifetime 200.
  - `CameraBouge` is never reset by wasp code; it is reset in `RestartSammy`, `player.wdl:968-989`, and in the camera-hit code at `camera.wdl:1037,1140`. While set, mouse-look is disabled (`camera.wdl:189,337`) and the chase camera uses fixed 0.5 smoothing (`camera.wdl:299-308`) [Q].
- **Flight:** same as type 5 with params 4/8/10/0.1 and homing range 1000 (gain 0.2), plus steering.
- **Steering:** `pan -= PlayerIntention.x * 20 * t` (`:359`).
  - `PlayerIntention.x` = `Key_Force.x` (keyboard, ±1) or `Joy_Force.x * 0.5` (`player.wdl:146-155`). So right input turns the wasp right at 20°/tick. Sammy keeps receiving the same input and moves too.
  - In `PlayerMoveMode == 2`, `Marche` zeroes `PlayerIntention.x` every frame (`player.wdl:184`), so **the wasp cannot be steered** [I].
- **Camera** (`HandleCamera3`, `camera.wdl:384-440`). Each frame while `Wasp6_Syn != null`:
  - `Key_Alt` held → camera follows the wasp (`CurrentPov_Syn = Wasp6_Syn`, `CameraOnWasp6 = 4`, `CamFollowMe`). Alt is also Sammy's U-turn key (`mm1.wdl:115`, `player.wdl:1642-1647`), so holding it may turn Sammy [Q].
  - else if forward/back input (`PlayerIntention.y != 0`) → `CameraOnWasp6 = 4`, camera follows Sammy.
  - else if `CameraOnWasp6 > 0` → decrement by `time` and follow Sammy normally.
  - else: if `cameramode == 2` (survey camera), restore `CameraSurveyPos` and switch to mode 1. If `NbrWasp6Fly > 0`, **return without updating, so the camera freezes in place**.
- [Q] `Wasp6_Syn` is never cleared after the wasp is removed, so these rules apply for the rest of the session, possibly with a dangling pointer [U]. Port: clear it on despawn and free the camera when `NbrWasp6Fly == 0`.

---

## 8. Hives (`InitHive :1197-1273`, `Hive_Hit :1275-1298`, `EmitWasp :1299-1321`)

WED (WMB model `hivegree`/`hiveblue`):

| Skill | Meaning |
|---|---|
| Skill1 | Wasp fate: 0 = decorative, 1-999 = lifetime in ticks, 1000 = "stays", 1001 = "persistent" |
| Skill2 | Random pan spread |
| Skill3 | int part = total wasps (0 = unlimited, 9990). frc×100 = **cap on the player's ammo** (default 10) |
| Skill4 | Vertical launch speed |
| Skill5 | Id of the entity whose impact makes it emit |
| Skill6 | Type 1-7 (999 = random 1-6) |
| Skill7 | Horizontal launch speed |
| Skill8 | Auto-emission period in ticks (0 = impact-only) |
| Flag2 | 1 = no impact handling |

```rust
// init
if !flag2 { enable_impact = true; event = Hive_Hit; }       // :1198-1202
passable = true;                                             // :1203 → impacts can never happen [I] → Hive_Hit is [DEAD]
cap = int(frc(skill3) * 100 + 0.5); if cap == 0 { cap = 10; }  // :1204-1205
total = int(skill3); bounce_force = 10; fate = skill1; vz = skill4;   // :1206-1209
if skill8 == 0 { return; }                                   // :1210
if total == 0 { total = 9990; }
waitt(32);
while skill8 > 0 && total > 0 {                              // :1213-1251
    let mut w = skill8; while w > 0 { w -= t; waitt(1); }
    if NbrAmmo1 < cap || WaspCurrentType != skill6 {
        ambient = 90; transparent = false;
        dest_pan = if skill2 != 0 { ang(pan + random(skill2) - random(skill2 * 0.5)) } else { pan };   // two randoms
        launch_speed = (skill7 + random(5), 0, skill4 + random(4));
        emit_wasp(at = pos); total -= 1;
    } else { ambient = -100; transparent = true; }           // "empty" look while capped
}
// total exhausted: explode (:1255-1272)
invisible = true; timer = 12;
while timer > 0 { VitesseWaspExplode = (random(20)-10, random(20)-10, random(20)-10);
                  emit 1 each: WaspExplodeYellow, Red, Blue, Green, Magenta, Gradient;
                  timer -= t; waitt(3); }                    // 12 iterations, 3 ticks apart (≈36 ticks)
remove();

fn emit_wasp() {                                             // :1299-1321
    type_ = if skill6 == 999 { int(random(6)) + 1 } else { skill6 };   // → hive.PlayerKey (Skill43)
    gem_light(hive, COLOUR[type_]); fate = skill1;
    create("wasp1.mdl", pos, LaunchNewWasp);
}
// Hive_Hit [DEAD]: if you.skill1 != skill5 { you.BounceForce1 = -you.EnnemySpeed.xy * 10; return }
//   else dest_pan = ang(you.pan - 180 + random(40) - 40); launch_speed.xy = you.PlayerSpeed.xy * skill7;
//   emit at you.pos (no total decrement, no cap)
```

### 8.1 Emitted wasp (`LaunchNewWasp :973-1181`)
```rust
hive = YOU; type_ = hive.PlayerKey; skill1 = type_ /*NOT 9999*/; skill6 = type_;
init_skin_scale_hive();               // §1.3 table; overlay unless flag3; flare if flag3 (:1398-1476)
passable = false; ambient = 80; event = Wasp_Bounce; enable_block = enable_entity = true;
bounces = 0;  vz_org = vz = hive.launch_speed.z; fric = 1.5;
pan = hive.dest_pan; tilt = random(20) + 20;
skill43 = hive.PlayerKey;             // overwrites poison slot with the type number [Q]; Skill3 keeps the table poison used on pickup
speed = max(|hive.launch.x| + |hive.launch.y| + 5, 15);
flag1 = true;
timer = fate;                          // Skill20 — SAME SLOT as `bounces` → bounce counter starts at `fate` [Q]
if fate == 1001 { flag3 = true; return; }  // :1013 frozen forever at spawn, solid, never pickable [Q]
bounce_force = 0.1;
while flag1 {                          // per FRAME (wait 1)
    push = -4;
    set_cycle("walk", ..);             // anim step is overwritten by ground Z (Skill18) [Q]; port: free-running walk
    vstep = 0;
    ground = trace_down(2000, ignore: hive, me, passable);         // :1031-1037
    if vz == 0 && z - ground > 26 { vz = -2; fric = 1.5; }
    if vz != 0 {
        vz -= fric * t; let vsp = vz;
        if z < ground + 25 {
            timer += 1;                                                    // "bounce"
            if timer == 1 { enable_scan = true; event = Ammo1_Hit; }       // only when fate == 0
            if speed > 0.5 {
                if vz_org > 15 { random kaputt sound 1..3 at 500 }
                vz_org *= 0.8; vz = vz_org - fric * t; vstep = vz * t;
            } else { vstep = ground + 25 - z; vz = 0; }
        } else { vstep = vsp * t; }
    }
    if timer == 0 { tilt -= 2 * t } else { tilt -= tilt * 0.1 * t }        // :1105-1112 (timer≠0 from start if fate>0)
    ent_move(rel = (mintime * speed, 0, 0), abs = (0, 0, vstep), ignore: hive, me, passable, glide);   // :1113-1114
    speed -= 0.05 * mintime;
    if |my_speed.x| + |my_speed.y| < 1 { vz_org -= 0.5 * mintime; if |vz_org| < 1.5 { flag1 = false; } }
    else { vz_org -= 0.02 * mintime; }
    if speed < 1 { speed = 1; if |vz_org| < 1.5 { flag1 = false; } }
    if vz_org > 5 { Particle_WaspHive(); }
    wait(1);
}
flag1 = true;
if timer < 5 && fate > 0 { enable_scan = true; event = Ammo1_Hit; }      // :1167-1172
passable = true;
if !invisible { skill2 = 1 /*1 ammo*/; → InitWaspWed (§2) with timer = fate + bounces }
// Wasp_Bounce (:1478-1488): pan = to_angle(bounce).pan; speed = max(speed - 8, 0.1)
```
Consequences with the shipped hives (fate 12 and 16):
- The landed pickup's timer is about 13-20, so the 16-tick fade-out starts at once. It vanishes ~1-1.3 s after landing.
- It cannot be picked up mid-air, because `timer == 1` never happens.
- It is worth 1 ammo with the table poison (type 4: 1, type 3: 1).
- [Q] A wasp picked up mid-air (fate 0) re-enters `InitWaspWed` after the shrink. It is briefly pickable again, so double pickup is possible.

---

## 9. Level usage (from `scratchpad/wmb/out/*.entities.json`)

**InitAmmo1** (633 total; type: count). Every pickup has Skill2 = 1, Skill7 = 0, flags 0 and scale 1. So the defaults table applies and each pickup gives +1 ammo.

| Level | Types | Level | Types |
|---|---|---|---|
| 10005 | 1:7, 7:32 | 111r_j | 7:95, 5:36 |
| 10006 | 7:20 | 112e_j | 1:8 |
| 10007 | 1:11 | 172c_j06 | 2:28, 1:12 |
| 10009 | 7:14 | 173b_t05 | 2:9 |
| 10010 | 7:16 | 174a_j07 | 2:6, 3:66 |
| 101o_j | 2:5 | 175c_t | 4:50, 1:17 |
| 102N_t | 3:15 | 176g_t | 4:10 |
| 103J_j | 0(→1):10 | 177e_j | 2:54 |
| 104M_j | 1:5 | 178d_t | 4:24 |
| 106g_t | 0(→1):4 | 179f_t | 4:27, 2:44 |
| 107i_j | 1:7 | 180c_t | 3:1 |

Type 6 appears nowhere, and type 5 only in 111r_j.

**InitHive** (3):

| Level | Model | Skill1..8 | Meaning |
|---|---|---|---|
| 176g_t | hivegree | [16, 360, 0.15, 20, 0, 4, 8, 20] | Type 4 green, unlimited, cap 15 ammo, every 20 ticks, vz 20 |
| 177e_j | hivegree | [12, 100, 0.40, 0, 0, 4, 8, 8] | Type 4, cap 40, every 8 ticks |
| 177e_j | hiveblue | [12, 45, 0.42, 0, 0, 3, 8, 12] | Type 3 blue, cap 42, every 12 ticks |

All have flags 0. Since the total is unlimited, **no shipped hive ever explodes**.

**InitDoorWeak** (75; HP = Skill1; some have Skill7 = 25, unused):

| Level | Doors |
|---|---|
| 10010 | 2× wk3 (HP 2) |
| 102N_t | 7× wk3 |
| 111r_j | 12× wk1 (HP 4) |
| 172c_j06 | 1× wk1 |
| 173b_t05 | 1× wk1 |
| 175c_t | 1× wk2 (HP 3), 3× wk3 |
| 176g_t | 1× wk3, 4× wk2 |
| 177e_j | 12× wk4 (HP 1), 10× wk3 |
| 179f_t | 2× wk4, 19× wk2 |

All have flags 0, so lions cannot break them (Flag1 = 0).

**InitWaspEnnemy**: extro only (19).

**Doors with Flag4**: none.

---

## 10. `InitWaspEnnemy` (extro only, brief) (`bullets.wdl:1502-1677,1697-1846`; `InitWaspExtro :1849-1938`)
- It is a copy of the lion init (push −3, `Lion_Hit` event, shadow, resistance from `frc(Skill1)`). `IDEnnemy` is forced to 8888 with per-species `IdEnnemySpec` 101-107 and default resistance 18/12/3/3/9/12/3.
- Skin and scale come from the hive table (`initSkinScaleWaspHive2`).
- `WaspEnnemyFly` loop:
  - runs the lion `EnnemyBehavior` and `AnimLion`;
  - trails by `IdWaspSpec` 101-106, with fps tiers 40/24;
  - plays a random buzz (Kaputt 1-3 at vol 100, or TakeWasp at 200) when `random(100) > 99.5`;
  - if `IdEnnemy == 99` ("You Win"), plays stand + `Particle_Win` at a random vertex.
- `InitWaspExtro` = the same without waiting for `PartieEncours`, with species 106/101.
- No interaction with the player's wasps beyond the generic lion rules.

---

## 11. Minimum viable port

**Scope:** all shipped levels except the extro and hives. Types 1, 2, 3, 4, 5 and 7 are needed; type 6 is not.

1. **Data and components.**
   - `Ammo { held_type: u8, poison: f32, count: i32 }` resource.
   - `WaspPickup { kind, amount, poison }` built from WED via the §1.3 pickup table (type 0 → 1; ignore WED Skill3/scale unless Skill7 ≠ 0).
   - `Wasp { kind, poison, speed, scan_range, tilt_gain, bounce_force, life, climb: Option<Climb>, shooter, last_disp, find_cd }`.
2. **Pickup.** Sphere test, radius = Sammy WED Skill5, each tick. Then:
   - type switch: reset ammo to 0 and set poison (escapees optional);
   - `count += amount`;
   - sounds clog3 + TakeWasp;
   - despawn (the shrink anim is optional).
3. **Firing.**
   - Gate on state 1 and ammo > 0 (else Error1).
   - Play TirA at 20×factor %/tick, then spawn at `pos + rot(pan+180)*(40,0,0) + (0,0,50)` with pan = Sammy pan and tilt = Sammy tilt.
   - Types 3/4: up to 3 wasps 2 ticks apart at pan −5/0/+5.
   - Play TirB at 20, then return to state 1.
   - Clear the type at 0 ammo.
4. **Flight.**
   - 16 Hz kinematic move of `speed` along the pan/tilt forward vector, as a swept sphere: small radius, ignore the shooter, other wasps and passable things.
   - Type 5 climb/hold/ease/dive timers.
   - Lifetime 128 ticks.
   - Invisible until 80 quants from Sammy.
5. **Homing (types 1/4/5).**
   - Every tick, among live lions/elephants within range whose direction is within ±60° pan and ±60° tilt of the wasp's heading, steer toward each (ascending distance) using the `0.4*t` pan gain and the per-type tilt gain.
   - Aim at `lion.head_world_pos - (0,0,32)` or `lion.pos + (0,0,32)`.
   - Skip while the climb has not finished.
   - Apply the fear tag (`ennemy_attacked += poison` if ≤ 0, 16-tick per-wasp cooldown).
6. **Hit resolution.** On the first blocking contact:
   - lion (8888) → send `WaspHit { poison, knockback: last_disp.xy * lion.weight * bounce_force, wasp_pan }`. The lion AI applies §6.1 (resistance, death state 6 / hit state 11 / ×2.5 knockback, hop `vz = 20*weight`, fear, dest_pan = wasp_pan, first-hit range switch). The wasp gets death kind 1004 (hit1 puff, visible recoil corpse).
   - weak door → `hp = int(hp - poison)`; morph to wk2/wk3/wk4 for hp 3/2/1 with the plank1 sound; otherwise remove the door and spawn 6 planks (§6.2; a simple physics tumble is acceptable at first). Wasp death 1001.
   - Flag4 door with Skill7 = 2 → implement for completeness via the wasp scan (cheap), even though no level uses it.
   - World or any other solid entity → death 1001 (invisible puff of 4 ticks of `WaspExplode<colour>`, random Kaputt sound).
7. **Death.** Disable collisions, random Kaputt sound, effect by kind, the 17-tick recoil only when visible (1004), then despawn.

**Later:**
- trail and explosion particles (rates in §1.3);
- light flash on pickup (`GemLight`);
- HUD wasp panel;
- escaping wasps on a type switch;
- hives and emitted bouncing pickups (176g_t, 177e_j), including the fade-after-landing quirk;
- plank physics fidelity;
- type 6 remote wasp and camera (§7), only needed for the cheat or custom levels;
- the extro wasp enemies;
- the [DEAD] paths (1002/1003, `Hive_Hit`, BonusVie) can be skipped.

**Open [U] items to verify in the game:**
- MOVE ignoring YOU (the shooter);
- push semantics between fan wasps;
- the 1001/1004 write order on lion hits;
- relative MOVE rotating by tilt (type 5 climb visibly rises: yes, if it rises);
- SCAN requiring `enable_scan` on targets and its line-of-sight behaviour;
- skin 0 rendering for type-1 pickups;
- `my_speed` being per-frame displacement.
