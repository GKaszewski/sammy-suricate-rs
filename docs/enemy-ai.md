# Enemy AI spec: lions, scorpions, elephant

This spec replaces reading `lion.wdl` and `anims.wdl:1-350`. It was written from the
WDL sources (CP1252, line numbers unchanged by `iconv`). Every rule cites
`file:line`. Tags: **[R]** read directly from the code; **[I]** inferred engine
semantics; **[?]** uncertain, check against the running game.

Engine conventions (tick = 1/16 s, `time` = ticks in the last frame, `mintime = min(time,1)`,
`CALL`/`BRANCH`/`WAITT`, synchronous events, `to_angle` returning the length in `.z`,
`accel`) are the ones in `docs/wdl-porting.md` §1-2. **Port at a fixed 16 Hz with
`time = mintime = 1`.** Many formulas below rely on that (§2.9).

Sources: `lion.wdl` (behaviour), `anims.wdl:2-350` (`AnimLion`), `mm1.wdl:300-438`
(field aliases), `skills.wdl` (global defaults), `InitLvl.wdl:1-100` (per-level
resets, **which override the skills.wdl defaults**), `player.wdl:520-620` (Sammy side),
`bullets.wdl:652-700` (wasps), and `mm1.wdl` world objects.

---

## 0. Corrections to `docs/gameplay.md` §8

| gameplay.md says | Actually |
|---|---|
| Animation speeds are "multiplied by SpeedEnnemy" | Only walk (`0.82×Speed`) and run (`0.3×Speed×LionAccelAttack`) are. Stand 3, roar 4 and death 4 are constants. Hit is `clamp(10×res×0.3, 5, 20)` (`lion.wdl:471`). `LionAnimSpeedattack=6` is **never used**: the swipe plays at the current run speed (§3). |
| "with Flag7, a roar at ≤75% resistance calls friends" | The hit-reaction roar (`anims.wdl:280`) **does not** call friends. Only the "stuck while running + player in LOS" roar in `Lion_Hit` (`lion.wdl:319-372`) calls `CallFriendsToHelp`, and only if Flag7 is set and the player is within attack range (`lion.wdl:367`). |
| "blink for about 40 ticks" | 33 ticks (`lion.wdl:654-678`). |
| Respawn / resistance bug | Real (`lion.wdl:691-694` is `IFDEF sammynewversion`, which is not defined), but **no shipped lion has more than 1 life** (Skill5 is always `1.x`), so respawn never happens in the shipped levels. |
| "Speed rotation 0.2 far / 0.02 near" | The general rate is `SpeedRotationAllLion` (0.2) every tick. The patrol near/far rates replace it only while patrolling (`lion.wdl:1820,1911,1921`). |
| `IntensiteEnnemyBounce 5`, `AmortiBounceLion 3` | `InitLevelglobal` resets them to **15** and **10** at every level load (`InitLvl.wdl:84-85`). The skills.wdl values are never effective. |
| "Elephant never damages Sammy; only crushes gems" | Correct for damage (all damage paths test `IdEnnemy==8888`). It is also **immune to wasps** (same test) but not to lava. Gem crushing needs Flag6, which no shipped entity sets. A respawned elephant becomes 8888 (`lion.wdl:729`). |
| "the second ranges" are used on chase start | Only the **attack** range switches to `AttackRangeBis` (on chase start, on an 8/9 sighting, or on a wasp hit while idle). The **react** range switches to its second value at first reaction or on a wasp impact while idle (`lion.wdl:1762-1766,574-578`). |

---

## 1. Entity data

### 1.1 Editor fields and unpacking (`InitLion`, `lion.wdl:11-219`)

Unpacking uses **f32 arithmetic**. `frc(x) = x - trunc(x)`. Because 1900.9f32 is
1900.900024 and 3000.9f32 is 3000.899902, the data contains off-by-one values
(see §1.5). Replicate them with `f32` math.

| Slot | Alias (`mm1.wdl`) | Unpacked into | Default | Notes |
|---|---|---|---|---|
| Skill1 | `IdEnnemy` (:312) | `type = int(Skill1)`. `EnnemyResistance = int(frc*100 + 0.5)` (:35-37) | res 0 → per-type default (§1.2) | After init, IdEnnemy is rewritten to the class id 8888 or 1111. |
| Skill2 | `SpeedEnnemy` (:316) | `SpeedEnnemy` = the **raw value (not truncated)**. `BounceForce (Skill44) = int(frc*100 + 0.5)` (:135) | BounceForce 0 → `LionForce` (2, `InitLvl.wdl:88`) (:136) | Units: quants per tick of forward motion before multipliers. Shipped values 5-20. Each respawn adds +1 (:739). |
| Skill3 | `LionAttackRange` (:319) | If 0 → 500 (:18). `shadow.AttackRangeBis = int(frc*10000)` with **no +0.5** (:207) | 500 | The attack range itself is never truncated, so 1900.9 is compared as 1900.9. |
| Skill4, patrol (Flag3=1 and ≠0) | `LionIdPath` (:323) | `LionRangePath = int(frc*10000 + 0.5)`; `LionIdPath = int` (:23-27) | n/a | Path id = path node `skill1` = ScanIdentite 27-99. |
| Skill4, random (Flag3=0 and ≠0) | `LionRandomTimeOrg` (:322) | `shadow.LionRandomTimeOrg2 = int(frc*1000 + 0.5)`; `LionRandomTimeOrg = int`; `LionRandomTime = Org`; `LionRandomTime2 = Org2` (:209-215) | n/a | Random mode is active iff `shadow.LionRandomTimeOrg2 ≠ 0` (:2116). |
| Skill5 | `LionNbrVie` (:325) | `LionAccelAttack (Skill21) = frc*10` with **no rounding** (:20); `LionNbrVie = int` (:21) | n/a | **Accel 0 (Skill5 an integer) ⇒ running speed and run animation speed are 0.** No shipped data has this. |
| Skill6 | `LionReactRange` (:327) | If 0 → 1000 (:19). The second react range `int(frc*10000 + 0.5)` is computed lazily at first use (:574, :1762) | 1000 | Same lazy unpacking for both uses. |
| Skill7 | `TempoStartBall` (:329) | `int()` at respawn: an extra wait in ticks (`lion.wdl:696-700`) | 0 | Only `108i_j` uses it (96). |
| Skill8 | `TimeForNothing` (:335) | raw (ticks) | 0 | "Ignore path/homing" duration after collisions. Shipped values 3-16, usually 4, 8 or 10. |
| Flag1 | | runtime "alive / loop running" (:196, :257) | | 0 ⇒ the `LionMarche` loop exits for good. |
| Flag2 | | lets **death tiles** and **teleports** act on the enemy (comment :2; `mm1.wdl:4066,4236`). This is a *tile* flag check (`my.flag2` of the tile), not a lion flag. Lion.Flag2 itself is read nowhere. | | |
| Flag3 | | patrol mode (:23, :241, :1822) | | Also changes collision reactions (:436, :556, :1443). |
| Flag4 | | **1 = never attacks**: the whole detection/chase/fear/random block is skipped (:1982) | | Used in the extro (flags 8 and 20). |
| Flag5 | | scan id 25 (doors/camera zones), radius `RangeScanDoor` = 500, every active tick (:1810-1818) | | Extro only (flags 20). |
| Flag6 | | scan id 100 (gem crush), sphere of radius `30*scale_z`, every 4th frame (:1803-1809) | | Not used in shipped data. |
| Flag7 | | calls friends (:341, :367); hit reaction → roar at ≤75% resistance (`anims.wdl:280`) | | `111r_j`, `172c_j06`, `174a_j07`. |
| Flag8 | | flies: no gravity, keeps ≥ 64 above ground, can turn in the air, tilts toward the patrol target Z (:1395-1403, :2176-2198) | | Not used by shipped lions (used by `InitWaspEnnemy`, bullets.wdl). |

Entity flags in the JSON are a bitmask: bit 0 = Flag1 … bit 7 = Flag8 (`docs/formats/wmb4.md`). For example 4 = Flag3, 64 = Flag7, 20 = Flag3 + Flag5.

### 1.2 Types (`lion.wdl:41-132`)

| int(Skill1) | `IdEnnemy` after init | `IdEnnemySpec` (Skill47) | `WeightForce` (Skill45) | `VertexHeadNumber` (Skill48) | default resistance | extra |
|---|---|---|---|---|---|---|
| 10 nano | 8888 | 1 | 0.65 | 109 | 6 | |
| 0 or 11 rasta | 8888 | 1 | 0.62 | 135 | 9 | |
| 12 hippie | 8888 | 1 | 0.60 | 132 | 12 | |
| 13 rocker | 8888 | 1 | 0.55 | 139 | 15 | |
| 14 punk | 8888 | 1 | 0.50 | 209 | 30 | |
| 2 scorpion | 8888 | 3 | 0.75 | 0 | 12 | |
| 3 scorpion | 8888 | 3 | 0.75 | 0 | 12 | `skin = 2` |
| 1 elephant | **1111** | 2 | 0.30 | 0 | 100 | |

- Any other `int(Skill1)` value keeps the raw id, `WeightForce` 0 and spec 0, and is effectively inert.
- The type comes from Skill1, **not from the model file**. `110n_t`, for example, has `rasta.MDL` with Skill1=13, which makes it a rocker.
- `VertexHeadNumber=0` disables the muzzle wall trace (:1122).

### 1.3 Other init (`lion.wdl:12-218`)

- `push = -3`, the same as Sammy (:12).
  - A MOVE passes through entities whose `push` is lower [I, from `gems.wdl:715` "player passe à travers" with gems at -4].
  - So lions pass through gems (-4) and collide with Sammy (-3) and with each other.
- `ambient 50`, `albedo 30`.
- `enable_entity`, `enable_impact`, `enable_block` and `enable_scan` are all 1 (:30-33).
- Event = `Lion_Hit`. Hull: `fat=1, narrow=0` (:139-141). The A4 BSP "fat hull" extents are engine-defined [?]. Port: use the model bbox or a capsule.
- `waitt 12`, then wait (frame by frame) until `PartieEncours == 1` (:143-148).
- `Sonar my,2000` (:151), which finds the floor → `target.z`.
- `Z = floor + 28*scale_z` (lions, elephant) or `floor + 15*scale_z` (scorpions) (:163-170).
- `HautSolBall` (Skill41) = floor Z. `HautBall` (Skill40) = Z − floor: the **hover height**, which stays constant for life (:172-173).
- `XorgBalle/YorgBalle/ZorgBalle` (Skill9-11) = the spawn position, used by respawn (:175-177).
- `BallAccel` (Skill12) = 1 (:183).
- Creates the shadow sprite `shadlio2.pcx` at `(x, y, floor+2)` with action `InitOmbreLion` (:192). It:
  - sets `lion.entity1 = shadow`, scale = lion scale ×1.2, `oriented`, tilt 90, pan 90, transparent, passable;
  - creates the range ring `circl100.mdl` with `InitRangeMdl` (`shadow.entity1 = ring`; passable, invisible, flare, ambient 100, unlit) (:742-771).
- `Flag1 = 1`, `LionEtat = 0`, `LionEtatBis = 0`, `LionAnimStep = 0`, `LionAnimSpeed = 3` (stand) (:196-201).
- Shadow skills: `CircleZ = 8 + random(12)` (:205), `EnnemyResistanceOrg = EnnemyResistance` (:206), `AttackRangeBis` (:207), `LionRandomTimeOrg2` (:211).
- `BallDestPan = pan` (:217), then `BRANCH LionMarche` (:218).

### 1.4 Runtime fields

All aliases are in `mm1.wdl:312-438`. "Shadow" means `lion.entity1`: the scripts store extra skills on the shadow sprite.

| Field | Slot | Meaning / units |
|---|---|---|
| `LionEtat` | Skill31 | state (§3). External writers may set 1004/1001/1002 (dead/frozen). |
| `LionEtatBis` | Skill14 | patrol sub-state: 0 = need a path node, 1 = heading to a node, 2 = within `FarTargetPath` of the node |
| `GuardStuckBalle` | Skill13 | "stuck" accumulator (collisions add to it, it decays 0.25/tick). Also clobbered as a tempo in states 0/4/8/9 (:1729, :1758). |
| `BallDeltaPan` | Skill15 | rotation step per tick = `ang(BallDestPan − pan) × SpeedRotationLion` (:2174) |
| `BallDestPan` | Skill20 | desired heading (degrees) |
| `BallAccel` | Skill12 | speed multiplier. It relaxes to 1 at 10%/tick (:2236). It is set to `LionAccelAttack` every tick in states 2/10/12/13. |
| `LionAnimStep` | Skill18 | animation percent 0-100 |
| `LionAnimSpeed` | Skill28 | percent per tick |
| `LionRandomTime` | Skill29 | ticks. A countdown, decremented by `time` each active tick (:2173) and in 8/9 (:1736). **>0 means "busy"**: no path steering, no homing, no chase start (non-random lions), no wall re-steer. Despite the name, it is used by every mode. |
| `LionRandomTime2` | Skill30 | ticks: the second-stage random countdown (:842-845) |
| `EnnemyTargetX/Y/Z` | Skill33/34/32 | current path node position |
| `EnnemyTargetPan/Tilt` | Skill42/16 | current path node angles |
| `TimeWithoutPatrol` | Skill46 | ticks spent in state 1 since the last node acquisition (:1824) |
| `EnnemyAttacked` | Skill19 | fear accumulator (wasp poison units) |
| `EnnemyResistance` | Skill43 | HP in wasp-poison units. Also read by others as `WaspPoison` (same slot) (`mm1.wdl:406`, `mm1.wdl:4518`). |
| `BounceBallX/Y/Z` ≡ `BounceForce1X/Y/Z` | Skill25-27 | **Same slots** (`mm1.wdl:250-252,365-367`). Collision handlers write `BounceBall*`, and `HandleSpeedLion` consumes them as the knockback velocity (`BounceForce1*`). |
| `LionForceX/Y/Z` | Skill22-24 | external force from moving platforms (`mm1.wdl:2404-2409`) |
| `BallVSpeed`, `BallFricVSpeed` | Skill35, 36 | vertical speed (quants/tick) and its deceleration per tick |
| `EnnemySpeedX/Y/Z` | Skill37-39 | last actual velocity in quants/tick (what Sammy and other lions receive as push) |
| `BounceForce` (≡ `ObjetPoussant`) | Skill44 | push multiplier given to whoever touches this entity (`player.wdl:537`) |
| `WeightForce` | Skill45 | how strongly this entity is pushed |
| shadow `BaffeDonne` | sh.Skill1 | −999 while a swipe has not yet hit, 0 after the hit (`mm1.wdl:428`) |
| shadow `EnnemyAvuPlayer` | sh.Skill2 | result of the last LOS trace (only for the Eye-of-Truth display) |
| shadow `MuzzleX/Y/Z` | sh.Skill3-5 | head-vertex position (wasps home on it, `bullets.wdl:675-689`) |
| shadow `CircleZ` | sh.Skill6 | range-ring Z offset (8-20) |
| shadow `EnnemyResistanceOrg` | sh.Skill7 | original resistance |
| shadow `HandleSonLion` | sh.Skill8 | sound handle |
| shadow `AttackRangeBis` | sh.Skill9 | pending second attack range (0 once used) |
| shadow `LionRandomTimeOrg2` | sh.Skill10 | random-mode stage-2 max; ≠0 ⇒ random mode |
| shadow `LionAnimPreviousStep` | sh.Skill11 | cross-fade progress 0-100 (100 = no fade in progress) |
| `entity1` links | | lion → shadow → range ring |

### 1.5 Globals (default, then per-level reset, then level overrides)

`InitLevelglobal` (`InitLvl.wdl:1-100`) runs at every level load. The `InitGame6/7` and
`InitLionSkills1/2` entities then override it (`mm1.wdl:1866-1889, 1928-1962`). Values
not reset by `InitLevelglobal` **persist across levels**.

| Global | skills.wdl | Per-level reset | Override entity |
|---|---|---|---|
| `SpeedRotationAllLion` | 0.2 (:138) | 0.2 (InitLvl:93) | LionSkills1.Skill3 (`177e_j` sets **0.4**) |
| `SpeedRotationLionNear` / `Far` | 0.02 / 0.2 (:124/:123) | same (:91-92) | LionSkills1.Skill1 / Skill2 |
| `DeltaDestPanLion` | 5 (:144) | **not reset** | LionSkills1.Skill4 |
| `LionLovePatrol` | 0.05 (:139) | 0.05 (:86) | LionSkills1.Skill5 |
| `LionForce` | 2 (:127) | 2 (:88) | LionSkills1.Skill6 |
| `NearTargetPath` / `FarTargetPath` | 100 / 150 (:130-131) | same (:94-95) | LionSkills1.Skill7 / Skill8 |
| `IntensiteEnnemyBounce` | 5 (:149) | **15** (:84) | LionSkills2.Skill1 |
| `AmortiBounceLion` | 3 (:121) | **10** (:85) | LionSkills2.Skill2 |
| `LimitTimeWithoutPatrol` | 64 (:142) | not reset | LionSkills2.Skill3 |
| `LionRandomAttack` | 50 (:122) | 50 (:97) | LionSkills2.Skill4 |
| `HargneEnnemy` | 2 (:787) | 2 (:98) | Game6.Skill3 |
| `EnnemyLachete` | 0.5 (:790) | 0.5 (:99) | Game6.Skill4 |
| `EnnemyFrightened` | 0.05 (:792) | 0.05 (:100) | Game6.Skill5 |
| `DistMuzzle` | 32 (:793) | 32 (:89) | Game6.Skill6 (999 ⇒ 0 = off). Shipped: 40 (`107i_j`), 64 (`177e_j`). |
| `TempoCall` | 128 (:797) | not reset | Game6.Skill7 |
| `RangeCall` | 10000 (:798) | not reset | **Game6.Skill8≠0 sets it to Skill7** (`mm1.wdl:1950`, bug; replicate) |
| `AnswerFriends` | 1 (:799) | **never reset** | (see §4.5) |
| `AttackCall` | 0 | 0 (InitLvl:33); also reset by `RestartSammy` (`player.wdl:894`) | |
| `graviteEnnemy` | -4 (:99) | -4 (InitLvl:11) | Game7.Skill5 (`180c_t`: **-1**) |
| `AccGraviteEnnemy` | 3 (:119) | not reset | none |
| `BounceOnEnnemy` | 1 (:125) | **0** (InitLvl:27) | Game7.Skill1 (shipped: 1, 2, 3) |
| `EnnemyMustDisplayRange` | 0 | 0 then **1** (InitLvl:5, :12) | Game7.Skill7 (`111r_j`: 2) |
| `DureeEyeTruthOrg` | — | 256 (InitLvl:6) | Game7.Skill6 (`111r_j`: 512) |
| `LionClimb` | 0 (:132) | — | unused (only in the commented-out old gravity code) |
| `Ralentit2` | 1 (:217) | never written | constant 1 |
| `IntensiteBlobBounce` (Sammy) | 5 (:148) | **30** (InitLvl:80) | Game4.Skill5. Not used against lions (§4.3). |
| `PoidPlayer1` → Sammy `WeightForce` | — | 0.8 (InitLvl:67) | |
| Sammy `BounceForce` | — | 1.8 (`player.wdl:124`) | |

Fixed constants (`mm1.wdl:6-12`): AnimSpeed Walk 0.82, Run 0.3, Stand 3, Rugit 4, Death 4, Attack 6 (unused), Hit 10. `RangeScanDoor` = 500 (`mm1.wdl:165`).

### 1.6 Typical shipped values (181 `InitLion` entities in 296 levels/extracts)

| Type | n | resistance (Skill1 frac) | SpeedEnnemy | AttackRange | ReactRange | AccelAttack | TimeForNothing | mode / flags |
|---|---|---|---|---|---|---|---|---|
| nano (10) | 69 | default 6 | mostly 8-10 (6-18) | 600-3500 (median 1000) | 10000 for 37 of them, else 600-5000 | 2.5-4.5 | 4 or 10 | 38 patrol, 30 plain, 1 random. Flags 4/0/64/8. |
| rasta (11) | 15 | 9 | 8-11 | 700-3000 | 10000 | 2.6-4.2 | 4 | 12 patrol |
| hippie (12) | 15 | 12 | 10-20 | 1500-5000 | 1500-10000 | 1.5-3 | 4 / 10 | 9 plain, 4 random, 2 patrol |
| rocker (13) | 41 | 15 | 6-15 | 10-5000 (median 1000) | 10000 for 22 | 2.2-5.2 | 4 | 32 patrol |
| punk (14) | 3 | 30 | 12-17 | 3000-5000 | 3000-15000 | 2-3 | 8 / 10 | |
| scorpion (2) | 38 | 12 by default; 20 for `2.2`; 12 for `2.12` | 5-15 | 300-1500 | 350-5000 | 1.2-3.5 | 8 or 10 | 35 random (Skill4 like `10.01` → Org 10 / Org2 10, `8.02` → 8 / 20, `10.004` → 10 / 4), flags 0 |

- No elephants, no Flag2/Flag6/Flag8, and every entity has 1 life.
- **First lions, level `10004`:**
  - nano `[10, 6, 1900.9, 33.03, 1.2, 1900.9, 0, 4]`, flags 4. Rasta: the same with Skill2 8 and path 32.03.
  - That gives resistance 6 or 9, speed 6 or 8, attack 1900.9 → Bis 9000, path id 33 or 32 with radius 300, 1 life, accel 2.0, react 1900 → 9000 after the first reaction, TimeForNothing 4.
  - **Level 10004 has no `initpath` nodes**, so these "patrol" lions never find a node (§2.5).
- **Path nodes** (`initpath`, `setpath.wmb`): Skill1 = path id (27-36), Skill2 = 0 (next-id switching is unused). Nodes are about 140-240 apart and their pan points along the path.

---

## 2. Per-tick algorithm

### 2.1 Main loop (`LionMarche`, `lion.wdl:239-311`)

```rust
// once, before the loop (:241-252)
if flag3 && etat_bis == 0 {
    target_pan = 999.0;
    scan(origin: pos, dir: angles, h_fov: 30, v_fov: 120, range: range_path, id: id_path); // PathScanned, §2.5
}
while flag1 {                                   // :257
    ennemy_behavior();                          // :262, §2.2
    shadow.pos = (x, y, haut_sol + 0.5); shadow.pan = pan + 90;   // :268-272
    anim_lion();                                // :273, §3.2
    display_ennemy_range();                     // :275, §4.6
    waitt(1);                                   // :284 (one tick)
}
// loop exits forever when flag1 = 0. The entity is NOT removed (removal code :288-310 is commented out).
```

### 2.2 `EnnemyBehavior` (`lion.wdl:1678-2239`), exact order

```rust
fn ennemy_behavior() {
    if etat == 7 { return; }                                         // :1679
    if etat == 6 {                                                   // :1683
        let head = ent_vertex(vertex_head_number);                   // vertex 0 for scorpion/elephant [?]
        if content(head) == SOLID { ball_accel = 0.0; }              // :1691-1696
        handle_speed_lion(); goto suite1;                            // :1697-1698
    }
    if etat == 3 { handle_speed_lion(); goto suite1; }               // :1700-1704 (roar: no steering)

    if etat == 8 || etat == 9 {                                      // :1705, random-mode idles
        test_dist_lion_player();                                     // §2.6
        if dist.z < attack_range && anim_step > 50 {                 // :1709
            player_vu = false; lion_shoot_player();                  // EVERY tick here (no modulo)
            if player_vu {
                if shadow.attack_range_bis != 0 { attack_range = bis; bis = 0; }
                nbr_lion_actifs += 1; guard_stuck = 0.0;
                etat = 1; shadow.prev_step = anim_step; anim_step = 0.0;
                anim_speed = 0.82 * speed;                           // :1715-1726
            }
        }
        guard_stuck = 6.0;                                           // :1729 (tempo hack, always >0)
        anim_lion();                                                 // :1734 → animation runs TWICE this tick (§3.2)
        handle_speed_lion();                                         // :1735 (speed.x = 0 in 8/9: gravity/knockback only)
        random_time -= time;                                         // :1736
        randomize_lion();                                            // :1737, §2.7
        guard_stuck -= time;
        shadow.pos = (x, y, haut_sol + 0.5); shadow.pan = pan + 90;
        if ennemy_must_display_range == 1 { display_range(); }      // :1744
        return;                       // no rotation update (Suite0 skipped): 8/9 never turn
    }

    if etat == 0 || etat == 4 {                                      // :1754, standby
        test_dist_lion_player();
        guard_stuck = 12.0;                                          // :1758
        if dist.z < react_range {                                    // :1760, NO line of sight needed
            let v = (frc(react_range) * 10000.0 + 0.5) as i32;       // :1762
            if v != 0 { react_range = v as f32; }                    // second react range
            nbr_lion_actifs += 1; guard_stuck = 0.0;
            etat = 1; shadow.prev_step = anim_step; anim_step = 0.0;
            anim_speed = 0.82 * speed;                               // :1768-1773
            // falls out of the if/else: nothing else this tick (no move)
        } else {
            anim_lion();                                             // :1781 (twice per tick again)
            handle_gravity_ennemy();                                 // :1782
            move_ent(rel: ZERO, abs: vertical_speed);                // :1784
            handle_forces_on_ennemy();                               // :1786 (platforms)
            shadow.pos = (x, y, haut_sol + 0.5); shadow.pan = pan + 90;
            if ennemy_must_display_range == 1 { display_range(); }
            guard_stuck -= time;
            return;
        }
        // NOTE: BounceForce1 (knockback) is NOT applied in 0/4. It is kept and applied once the lion becomes active.
        return; // [R] the active branch is the `else` of :1754, so it is not entered this tick
    }

    // ---------------- active states: 1, 2, 10, 11, 12, 13 (and anything else, e.g. 1004 if the loop still runs)
    if flag6 && ticks1 % 4 == 0 {                                    // :1803
        scan(pos, angles, 360, 360, 30.0 * scale_z, id: 100);        // gems explode (§6.6)
    }
    if flag5 { scan(pos, angles, 360, 360, 500, id: 25); }           // :1810-1818, doors/camera zones
    speed_rotation_lion = speed_rotation_all_lion;                   // :1820 (0.2)

    if etat == 1 && flag3 { patrol(); }                              // :1822-1964, §2.5

    handle_speed_lion();                                             // :1978, §2.3

    if !flag4 {                                                      // :1982
        test_dist_lion_player();                                     // §2.6
        player_vu = false;
        let period = match (spec == 3, etat == 2) {                  // :1986-2019
            (false, false) => 10, (false, true) => 4,
            (true,  false) => 16, (true,  true) => 8 };
        if ticks1 % period == 0 { lion_shoot_player(); }             // ticks1 = frame counter reset to 0 every 64 frames (mm1.wdl:893-903)

        if player_vu {                                               // :2020, chase start
            if (etat == 1 && random(100) > LION_RANDOM_ATTACK) || etat == 9 {
                if dist.z <= attack_range && dist.tilt.abs() < 45.0 {
                    let random_mode = shadow.random_time_org2 != 0;
                    if (random_time <= 0.0 && !random_mode) || random_mode {
                        if spec == 1 && random(100) > LION_RANDOM_ATTACK {
                            let h = play_sound(LionAttack1 /*lionatt1.wav*/, 100);
                            tune_sound(h, vol: 30000.0 / dist.z, freq: 200.0 * weight + random(50) - 25.0);
                        }
                        if spec == 3 && random(100) > LION_RANDOM_ATTACK {
                            play_ent_sound(ScorpionAttack /*scoratt1.wav*/, 1000);
                        }
                        warning_attack();                            // §4.2 (runs as its own coroutine)
                        etat = 2; etat_bis = 0; ball_dest_pan = dist.pan;
                        shadow.prev_step = anim_step; anim_step = 0.0;
                        anim_speed = 0.3 * speed * accel_attack;
                        if shadow.attack_range_bis != 0 { attack_range = bis; bis = 0; }   // :2059-2063
                    }
                }
            }
        }

        if etat == 2 || etat == 10 {                                 // :2069
            if dist.z > attack_range + 128.0 {                       // give up
                etat = 1; shadow.prev_step = anim_step; anim_step = 0.0; anim_speed = 0.82 * speed;
                if spec == 3 { play_ent_sound(ScorpionWalk /*scorwalk.wav*/, 1000); }
            } else {
                ball_accel = accel_attack;                           // :2088
                if guard_stuck == 8.0 { ball_dest_pan = ang(ball_dest_pan + random(20) - 10.0); }  // :2089, exact float equality: practically never
                if random_time <= 0.0 && guard_stuck < 9.0 { ball_dest_pan = dist.pan; }          // :2093, homing
            }
            if dist.z < 16.0 {                                       // :2098, too close (e.g. directly below/above)
                ball_dest_pan = ang(dist.pan - 180.0); random_time = time_for_nothing * 2.0;
            }
        } else {
            if etat == 13 {
                ball_accel = accel_attack;
                if guard_stuck > 9.0 { ball_dest_pan = ang(ball_dest_pan + random(20) - 10.0); }  // :2111
            }
            if !flag3 && shadow.random_time_org2 != 0 { randomize_lion(); }  // :2116
        }
        // :2121-2127 is dead debug code
        if ennemy_attacked > 0.0 {                                   // :2128, fear
            let v = (ennemy_resistance * weight * ENNEMY_LACHETE).abs();      // 0.5
            ennemy_attacked -= ENNEMY_FRIGHTENED * mintime * v;      // 0.05
            if ennemy_attacked > v {
                if etat <= 1 {                                       // in practice only state 1 reaches here
                    etat = 12; etat_bis = 0; ball_dest_pan = random(360);
                    anim_step = 0.0; anim_speed = 0.3 * speed * accel_attack;
                    random_time = time_for_nothing;
                }
            } else if etat == 12 {
                etat = 1; shadow.prev_step = anim_step; anim_step = 0.0; anim_speed = 0.82 * speed;
            }
        }
        if etat == 12 {                                              // :2158
            ball_accel = accel_attack;
            if guard_stuck == 8.0 {                                  // practically never
                ball_dest_pan = ang(ball_dest_pan + random(20) - 10.0);
                ennemy_attacked = 3.0 * weight;
            }
        }
    }
    // Suite0 (:2172)
    random_time -= time;                                             // :2173
    ball_delta_pan = ang(ball_dest_pan - pan) * speed_rotation_lion; // :2174
    if flag8 {
        pan += ball_delta_pan * time;
        if flag3 {
            if (z - target_z).abs() < 16.0 { tilt -= tilt * 0.25 * mintime; }
            else if z > target_z { tilt -= speed_rotation_lion * mintime; }
            else { tilt += speed_rotation_lion * mintime; }          // :2182-2196
        }
    } else if z - haut_sol < 128.0 { pan += ball_delta_pan * time; }  // :2201, cannot turn when 128+ above ground
    enable_block = ball_delta_pan.abs() <= 6.0;                      // :2206-2213 (no BLOCK events while turning fast)
    if guard_stuck > 0.0 {                                           // :2215
        guard_stuck -= 0.25 * time;
        if guard_stuck > 16.0 {
            if !flag3 { ball_dest_pan = ang(pan - 180.0); } else { etat_bis = 0; }
            guard_stuck = 0.0;
        }
    }
    // suite1 (:2233)
    ball_accel += (1.0 - ball_accel) * 0.1 * time;                   // :2236
}
```

`random(n)` is a uniform float in [0,n). `int(random(3))+1` gives a uniform choice in {1,2,3}. `ang(a)` wraps to (-180,180] [I].

### 2.3 `HandleSpeedLion` (`lion.wdl:1073-1207`)

```rust
fn handle_speed_lion() {
    let mut fwd = 0.0;
    if etat == 1 && ball_delta_pan.abs() < 10.0 { fwd = speed * time * ball_accel; }  // :1076, walk only when nearly facing
    if etat == 2 || etat == 13 { fwd = speed * time * ball_accel; }                    // :1082
    if etat == 10 { fwd = speed * time * ball_accel * 1.5; }                           // :1086, swipe lunge
    if etat == 12 { fwd = speed * time * ball_accel * 1.2; }                           // :1091, flee
    // states 3, 6, 8, 9, 11: fwd = 0
    ball_message = 999;                                              // :1098 (unused here)
    handle_gravity_ennemy();                                         // §2.4, sets VerticalSpeed.z
    let moved = move_ent(rel: (fwd, 0, 0) /*rotated by pan,tilt,roll*/, abs: (0, 0, vertical_speed.z)); // :1103
    current_lion_speed = moved;                                      // global, used by the walk-cycle rate in AnimLion
    if moved == 0.0 {
        current_lion_speed = 0.1;
        let t = rotate_pan_only((0.1, 0, 0), pan);
        ennemy_speed.xy = t.xy / mintime;                            // :1105-1114 (Z left unchanged)
    } else {
        ennemy_speed = my_speed / time;                              // :1117-1119 (quants per tick)
    }

    // Muzzle wall probe (:1122-1162)
    if vertex_head_number != 0 && DIST_MUZZLE != 0.0 {
        let head = ent_vertex(vertex_head_number);                   // current frame, world space
        shadow.muzzle = head;
        let tip = head + rotate((DIST_MUZZLE, 0, 0), (pan, tilt, roll)); // 32 ahead along the facing
        let r = trace(head, tip, IGNORE_ME | IGNORE_PASSABLE | IGNORE_YOU | IGNORE_SPRITES | IGNORE_MODELS); // level only
        if r > 0.0 { ennemy_on_block(); }                            // uses trace NORMAL, §2.8
        if r == -1.0 {                                               // head inside a solid [I]
            let a = if etat == 6 { pan } else { pan - 180.0 };
            let k = rotate((INTENSITE_ENNEMY_BOUNCE /*15*/, 0, 0), (a, 0, 0));
            bounce_force1.x = k.x; bounce_force1.y = k.y;            // push backward (forward while dying)
        }
    }

    // Knockback (:1164-1204)
    if bounce_force1 != ZERO {
        enable_impact = false; enable_entity = false;
        move_ent(rel: ZERO, abs: bounce_force1 * mintime);
        ennemy_speed = my_speed;                                     // NOT divided by time
        enable_impact = true; enable_entity = true;
        let dec = mintime * AMORTI_BOUNCE_LION /*10*/ * (80.0 / (average_fps + 1.0));   // :1182
        for c in [x, y, z] {                                         // move each component toward 0 by dec, clamp at 0
            let s = -sign(bounce_force1[c]);
            bounce_force1[c] += dec * s;
            if sign(bounce_force1[c]) == s { bounce_force1[c] = 0.0; }
        }
    }
    handle_forces_on_ennemy();                                       // :1206
}

fn handle_forces_on_ennemy() {                                       // :1208-1224
    if lion_force != ZERO {
        enable_impact = false; enable_entity = false;
        move_ent(rel: ZERO, abs: lion_force);                        // not time-scaled
        enable_impact = true; enable_entity = true;
        lion_force = accel(lion_force, ZERO, 0.2);                   // ≈ lion_force *= (1 - 0.2*time) [I, wdl-porting accel]
    }
}
```

**AverageFps dependence:** `average_fps` defaults to 50 (`skills.wdl:437`). At 50 fps, `dec ≈ 15.7·mintime` per tick. Port: use `average_fps = 50` (dec = 15.69) unless you emulate fps.

### 2.4 `HandleGravityEnnemy` (two definitions)

- **The definition at `lion.wdl:1228-1285` is inside a `/* … */` block (`:1227` to `:1286`), so it does not exist.**
- Only `lion.wdl:1288-1406` is compiled. (Had both been live, WDL would reject or redefine the duplicate; that question does not arise.)

```rust
fn handle_gravity_ennemy() {
    let r = trace(pos, pos - (0, 0, 2000),
                  USE_BOX | IGNORE_ME | IGNORE_SPRITES | IGNORE_PASSABLE | ACTIVATE_SONAR); // :1291-1295
    // ACTIVATE_SONAR fires EVENT_SONAR on the entity hit: death tiles, lava, teleports, moving platforms (§6.4)
    if r > 0.0 {
        match you {
            Some(e) if e.skill1 == 8888 || e.skill1 == 507 || e.skill1 == 77 => vertical_speed.z = 0.0, // :1306, standing on another lion, terrain prop or projectile: keep the old floor
            Some(_) => haut_sol = target.z,               // :1314 (vertical_speed.z not cleared)
            None    => { haut_sol = target.z; vertical_speed.z = 0.0; }
        }
    } else { vertical_speed.z = 0.0; ball_vspeed = 0.0; }             // :1324-1328, nothing within 2000

    if !flag8 {
        if ball_vspeed == 0.0 {
            if z - haut_sol > haut_ball {                             // airborne
                if z - haut_sol > 200.0 {
                    let v = pov.z - z;
                    if v > 100.0 && v < 400.0 { ball_vspeed = 20.0; ball_fric_vspeed = 3.0; } // :1339, hop up toward a player 100-400 above
                    else {
                        ball_vspeed = GRAVITE_ENNEMY /*-4*/; ball_fric_vspeed = ACC_GRAVITE_ENNEMY /*3*/;
                        if speed.x > 1.0 {                            // `Abs(Speed.X > 1)` = the bool
                            if random(10) > 5.0 { ball_delta_pan -= 90.0 } else { ball_delta_pan += 90.0 } // :1350
                            // effectively a no-op: ball_delta_pan is recomputed at Suite0 before use
                        }
                    }
                } else { ball_vspeed = GRAVITE_ENNEMY; ball_fric_vspeed = ACC_GRAVITE_ENNEMY; }
            } else {
                vertical_speed.z = (haut_sol + haut_ball) - z;        // :1370, snap: climbs any step instantly
            }
        }
        if ball_vspeed != 0.0 {
            ball_vspeed -= ball_fric_vspeed * time;                   // :1381
            if z < haut_sol + haut_ball {
                vertical_speed.z = (haut_sol + haut_ball) - z; ball_vspeed = 0.0;   // land
            } else {
                vertical_speed.z = accel(vertical_speed.z, ball_vspeed, 1.0);       // = ball_vspeed when time == 1 [I]
            }
        }
    } else if z < haut_sol + 64.0 {                                   // flying
        vertical_speed.z = (haut_sol + 64.0) - z; ball_vspeed = 0.0;
    }
}
```

- The enemy **hovers at a constant `HautBall`** above the traced floor. The hull bottom does not rest on the ground.
- Wasp hits set `ball_vspeed = 20·weight` (hop) or 20 on death, with fric 3.5 (§5).
- `VerticalSpeed` is a **global** shared by all actors. When time = 1 it is fully rewritten before use, except in the Flag8 path and the "standing on a foreign entity" path.

### 2.5 Patrol (`lion.wdl:1822-1964`, `PathScanned :783-814`, `InitPath :774-780`)

**Path node** (`InitPath`): small sprite, invisible, passable, `enable_scan`, event `PathScanned`. `skill1` = id, `skill2` = optional "switch the lion to path id skill2".

**`PathScanned`** runs synchronously inside the lion's SCAN, with my = node and you = lion:

```rust
if scan_identite != node.skill1 { return; }
if node.skill2 != 0 { lion.id_path = node.skill2; }       // even when the node is then rejected
if lion.etat_bis != 0 { return; }                          // a node was already accepted during this scan
if lion.target.xy == node.xy { return; }                   // never re-pick the current node
lion.target = node.pos; lion.target_pan = node.pan; lion.target_tilt = ang(node.tilt);  // written BEFORE the LOS test
if trace(node.pos, lion.pos, IGNORE_ME|IGNORE_PASSABLE|IGNORE_YOU|IGNORE_SPRITES|IGNORE_MODELS) == 0.0 {
    lion.etat_bis = 1;                                     // clear LOS: stop accepting further nodes
}
// :816-836 is unreachable
```

- **Result:** the first node (in engine scan order [?]) with level-geometry LOS wins.
- If no node has LOS, **the last node visited still overwrites the target**. EnnemyBehavior then treats any target change as "found" (:1839).

**Patrol step** (only `etat == 1 && flag3`):

```rust
time_without_patrol += time;
if time_without_patrol > LIMIT_TIME_WITHOUT_PATROL /*64*/ { etat_bis = 0; }
if etat_bis == 0 {
    let old = target.xy;
    scan(pos, angles /*lion pan & tilt*/, h_fov: 90, v_fov: 120, range: range_path, id: id_path); // :1831-1838
    if target.xy != old { etat_bis = 1; time_without_patrol = 0.0; }
    else {
        let d = to_angle((target.x - x, target.y - y, 0));            // 2-D distance in d.z
        if d.z > range_path * 1.8 && etat != 2 {                      // :1854, lost: wider scans
            etat_bis = 0;
            scan(pos, angles, 100, 120, range_path * 4.0, id_path);   // forward 100°, 4× range
            if target.xy == old {
                scan(pos, (ang(pan - 180), d.tilt /*=0*/, _), 100, 120, range_path * 3.0, id_path); // backward 100°, 3× range
                time_without_patrol = 0.0;                            // reset even if nothing found
            }
        }
    }
    // no steering this tick: the lion keeps its heading
} else {
    let d = to_angle((target.x - x, target.y - y, 0));
    let mut v = ball_dest_pan;
    if d.z < FAR_TARGET_PATH /*150*/ && (etat_bis == 1 || etat_bis == 0) {
        v = target_pan; speed_rotation_lion = 0.5; etat_bis = 2;
        let k = (LION_LOVE_PATROL * 0.5).min(1.0);                    // 0.025
        bounce_force1.xy = (target.xy - pos.xy) * k;                  // magnetic pull through the knockback channel
        tilt += ang((target_tilt - tilt) * 0.5 * mintime);
    }
    if d.z < FAR_TARGET_PATH && etat_bis == 2 {                       // also runs right after the block above
        v = target_pan; speed_rotation_lion = SPEED_ROTATION_LION_NEAR; // 0.02, overrides the 0.5 (which is never effective)
        let k = (LION_LOVE_PATROL * 0.5).min(1.0);
        bounce_force1.xy = (target.xy - pos.xy) * k;
        tilt += ang((target_tilt - tilt) * 0.5 * mintime);            // tilt eases twice on the entry tick
    }
    if d.z >= FAR_TARGET_PATH && etat_bis == 1 { v = d.pan; speed_rotation_lion = SPEED_ROTATION_LION_FAR; } // 0.2, head toward the node
    if d.z > range_path && etat_bis == 2 { etat_bis = 1; v = d.pan; speed_rotation_lion = SPEED_ROTATION_LION_FAR; } // escaped the node zone
    if random_time <= 0.0 { ball_dest_pan = ang(v); }
    else { ball_dest_pan = ang(ball_dest_pan + (v - ball_dest_pan) * LION_LOVE_PATROL); } // :1940, difference NOT wrapped: may blend the long way round [R]
    if d.z < NEAR_TARGET_PATH /*100*/ {                               // reached: look for the next node ahead of this one
        etat_bis = 0;
        let old = target.xy;
        scan(target.pos, (target_pan, target_tilt), h_fov: 30, v_fov: 120, range: range_path, id: id_path); // :1954, from the node, along its facing
        if target.xy != old { etat_bis = 1; time_without_patrol = 0.0; }
    }
}
```

- **SCAN cone** [I]: it reaches entities within `range` whose direction from the origin lies within ±h_fov/2 of `angles.pan` and ±v_fov/2 of `angles.tilt`. 360/360 is a sphere. Whether the cone width is total or half-angle is unverified [?]. Assume total.
- The `LionRandomTime > 0` gate (set to `TimeForNothing` by collisions, :438, :558, :1447) means that after bumping, path steering only blends at 5%/tick for `TimeForNothing` ticks.
- **Level 10004/10005 lions have Flag3 but no nodes.** They rescan every tick, never steer, walk straight and bounce off walls (§2.8). Replicate this; do not "fix" it with nearest-node logic.

### 2.6 `TestDistLionPlayer` (`lion.wdl:944-996`) and `LionShootPlayer` (`:908-943`)

```rust
fn test_dist_lion_player() {
    dist.z = 1.0;                                                     // :945
    if etat == 12 { return; }                                         // fleeing: dist stays 1
    dist = to_angle(pov.pos + (0, 0, 15) - pos);                      // pan, tilt, z = 3-D distance (to pov_Syn = Sammy normally)
    if ATTACK_CALL != 0.0 {                                           // a friend call is active (§4.5)
        if spec as f32 + weight == FRIEND_CALL {                      // same species+subtype; exact float compare
            if dist.z < RANGE_CALL && dist.z > 128.0 {
                if matches!(etat, 0 | 1 | 4 | 8 | 9) {
                    if dist.z > attack_range { ANSWER_FRIENDS += 1; }
                    etat = 2; etat_bis = 0; ball_dest_pan = dist.pan;
                    shadow.prev_step = anim_step; anim_step = 0.0;
                    anim_speed = 0.3 * speed * accel_attack;
                }
                dist.z = 64.0;                                        // :972, fake "in range" for the rest of this tick
            }
        }
        return;                                                       // during a call, NOBODY reverts to idle
    }
    if dist.z > react_range * 1.1 && (etat == 1 || etat == 2) {       // :977
        etat = 0; NBR_LION_ACTIFS = (NBR_LION_ACTIFS - 1).max(0);
        if random(100) > LION_RANDOM_ATTACK { etat = 4; }             // 50%: alt stand
        next_frame = 0; anim_speed = 3.0; anim_step = 0.0;
    }
}

fn lion_shoot_player() {                                              // LOS test
    // my_pos/model_pos are built (+50 Z) but the trace is from my.pos (origin) to pov + (0,0,50)
    trace(pos, pov.pos + (0, 0, 50), IGNORE_ME | IGNORE_PASSABLE | IGNORE_YOU | IGNORE_SPRITES); // models NOT ignored
    if let Some(e) = you {
        if e.id_player == 3333 { player_vu = true; shadow.ennemy_avu_player = 1; }
        else { shadow.ennemy_avu_player = 0; }
    }   // nothing hit ⇒ not seen (and EnnemyAvuPlayer unchanged)
}
```

- **LOS semantics [?].** "Seen" requires the ray to *hit Sammy's hull* before anything else.
  - The end point is 50 above Sammy's origin. Sammy's centre is about 35 above the floor (`gameplay.md` §4), so a ray ending above his box would never hit him. Whether A4 traces against model bboxes and how tall Sammy's hull is is unverified.
  - `IGNORE_YOU` ignores whatever `you` currently points at; if that is Sammy, he is invisible to this test.
  - Recommended port: `seen = raycast(lion_origin → sammy_center + up*k)` with the first hit being Sammy (level geometry and other non-passable entities block; sprites and passables don't).
- **Elevation gate:** `|dist.tilt| < 45` (chase start only).
- `NbrLionActifs` is bookkeeping only (its sole reader is commented-out code, :1513).

### 2.7 Random mode (`RandomizeLion`, `lion.wdl:839-907`)

Called from 8/9 every tick (:1737) and from active non-chasing states when `!flag3 && Org2 != 0` (:2116).

```rust
fn randomize_lion() {
    if random_time > 0.0 { return; }
    if random_time2 > 0.0 { random_time2 -= time; return; }
    let v = (random(3.0) as i32) + 1;
    if v == 1 && etat != 8 {                     // (the nested state-9 branch :856-869 is dead: state 9 is never chosen)
        etat = 8; shadow.prev_step = anim_step; anim_step = 0.0; anim_speed = 3.0;
        random_time = random_time_org * 0.5; random_time2 = random(org2) * 0.5; return;
    }
    if v == 2 {
        ball_dest_pan = random(360);
        if etat != 1 { shadow.prev_step = anim_step; anim_step = 0.0; }
        etat = 1; anim_speed = 0.82 * speed;
        random_time = random_time_org; random_time2 = random(org2) * 0.5; return;
    }
    // v == 3, or v == 1 while already in 8
    if etat != 2 { shadow.prev_step = anim_step; anim_step = 0.0; }
    ball_dest_pan = random(360); etat = 13; anim_speed = 0.3 * speed * accel_attack;
    random_time = random_time_org; random_time2 = random(org2);
}
```

- In practice: P(stand 8) = 1/3, walk = 1/3, run 13 = 1/3, and from 8: walk 1/3, run 2/3.
- **Note:** RandomizeLion can also fire while in **11 (hit), 3 is excluded (returns early), 12 (flee)** and 1004, and overrides them.
- While `random_time > 0` (e.g. after a collision set it), no re-roll happens.

### 2.8 Collisions (`Lion_Hit`, `lion.wdl:313-643`; `EnnemyOnBlock`, `:1408-1505`)

`Lion_Hit` is the lion's event. Order inside it:

1. `EVENT_SCAN` → return (:314).
2. **Stuck roar** (:319-376). Applies to any non-scan event:
   - **Trigger:** `guard_stuck > 10` and `etat ∈ {2, 12, 13}`.
   - **Effect:** save `you`; `random_time = 0`; `guard_stuck = 0`; trace from `pov + (0,0,100)` to `pos + (0,0,100)` (ignore me, passable, you, sprites, models; level only).
   - If the trace is clear (`r == 0`):
     - `ball_dest_pan = ang(pan − 180)`.
     - Sound (spec 1): if Flag7 and `AttackCall == 0`, `play_ent_sound(Tigre, 2000)` and `play_ent_sound(LionAttack1, 1500)`. Otherwise `play_sound(Tigre, 100)` tuned as in §2.2.
     - Sound (spec 3): `ScorpionAttack, 1000`.
     - `random_time = TimeForNothing×2`; `etat = 3`; `shadow.prev_step = anim_step`; `anim_step = 0`; `anim_speed = 4`.
     - **If Flag7 and `dist.z < attack_range` → `CallFriendsToHelp`** (§4.5).
     - Return.
   - Otherwise restore `you` and continue.
3. `EVENT_BLOCK` (level geometry during MOVE, only while `enable_block`) → `EnnemyOnBlock` (:380-383).
4. `EVENT_ENTITY` (this lion moved into an entity, :385-529):

```rust
guard_stuck += ball_accel * time;
let mut n = NORMAL; if n.x + n.y == 0.0 { n = normalize(BOUNCE); }
let a;
if you.bounce_force != 0.0 {                                   // pusher (Sammy 1.8, lions 2, wasps 1-3)
    bounce_force1.xy = you.ennemy_speed.xy * weight * you.bounce_force;  // Skill37/38 of the other: Sammy PlayerSpeed, wasp my_speed
    bounce_force1.z = 0.0;
    a = to_angle(n).pan + random(10) - 5.0;
} else {
    let mut v = (BOUNCE.x.abs() + BOUNCE.y.abs()) * 0.5 * weight * INTENSITE_ENNEMY_BOUNCE; // 15
    if v == 0.0 { v = INTENSITE_ENNEMY_BOUNCE; }
    bounce_force1 = n * v;
    a = to_angle(n).pan + if random(1) > 0.5 { 45.0 } else { 315.0 };
}
if ball_delta_pan.abs() < DELTA_DEST_PAN_LION /*5*/ { ball_dest_pan = ang(a); }
if flag3 && etat != 2 && random_time <= 0.0 { random_time = time_for_nothing; }
if id_ennemy == 8888 {        // elephant (1111) skips everything below
    if you.id == 9999 { wasp_hit(×2); }                        // §5.1
    if you.id == 3333 {                                        // walked into Sammy
        random_time = 0.0; ball_dest_pan = pan;                // keep the heading
        swipe_contact();                                       // §4.1
    }
    return;
}
```

5. `EVENT_IMPACT` (an entity moved into this lion, :531-641):
   - `guard_stuck += ball_accel·time`.
   - If `you.bounce_force ≠ 0`: `bounce_force1.xy = you.ennemy_speed.xy × weight × you.bounce_force`, z = 0, and `a = you.pan` (the lion turns to the pusher's direction of travel).
   - Otherwise: `v = |BOUNCE.x|+|BOUNCE.y|+|BOUNCE.z|` and `bounce_force1 = AngleNormal × v`, where **`AngleNormal` is stale**, left over from the last ENTITY event (:547) [R bug]. Then `a = to_angle(AngleNormal).pan`.
   - `ball_dest_pan = a` **unconditionally** (:554).
   - If Flag3 and `random_time ≤ 0`: `random_time = TimeForNothing`.
   - If 8888: a wasp (9999) runs `wasp_hit(×2.5)` with the idle range switch (§5.1). Sammy (3333): `random_time = 0`, then if `etat == 10 && anim_step > 60`, `WindowHearts −= res×0.25` **on every impact** (no once-per-swing guard, :625-635), plus a particle; ≤0 → `Player_Kaputt`.

`EnnemyOnBlock` (wall, from `EVENT_BLOCK` or the muzzle probe):

```rust
guard_stuck += 3.0 * time;
let mut v = (ennemy_speed.x.abs() + ennemy_speed.y.abs()) * 0.1 * weight * INTENSITE_ENNEMY_BOUNCE; // 15
if v == 0.0 { v = INTENSITE_ENNEMY_BOUNCE; }
bounce_force1 = NORMAL * v;                                   // push away from the wall
let off = random(20.0) + 25.0;                                // 25..45°
let a = to_angle(NORMAL).pan + if random(1.0) > 0.5 { off } else { -off };   // random(1) is drawn first, then random(20)
if random_time <= 0.0 && ball_delta_pan.abs() < DELTA_DEST_PAN_LION { ball_dest_pan = ang(a); }
if flag3 {
    if etat != 2 && random_time <= 0.0 { random_time = time_for_nothing * 1.5; }
    if etat == 2 {                                            // patrol lion chasing hits a wall
        // trace pov+50 → pos+50 (ignore models); `you` is then almost always null ⇒ else branch [?]
        if random_time <= 0.0 { random_time = time_for_nothing; }
        etat = 1; anim_speed = 0.82 * speed;                  // patrol lions give up the chase at walls
        // (if `you` were Sammy: r > 200 ⇒ stay 2 with run anim, else random_time = TFN, etat = 1)
    }
    if etat == 12 {
        if random_time <= 0.0 { random_time = time_for_nothing; }
        etat = 1; anim_speed = 0.82 * speed;
    }
}
```

`random(1) > 0.5` is the 50/50 coin used throughout.

### 2.9 Timers, units and WDL caveats

- **Distances** are in quants (the world unit). **Angles** are in degrees, pan CCW about +Z [I].
- `waitt 1` in `LionMarche` (:284) means the AI runs **at most once per tick**, and every step is multiplied by `time` (the last *frame* duration).
  - Above 16 fps this double-counts oddly: the loop runs every ~16/fps... ticks, while each step uses one frame's `time` [?].
  - At a fixed 16 Hz with `time = 1` both readings agree. **This is the only safe port.**
- `ticks1` is a frame counter, reset to 0 every 64 frames (`mm1.wdl:893-902`). So `ticks1 % 10` has an irregular period (10, … then 4).
- **Globals as scratch.** `DistLionPlayer`, `PlayerVu`, `SpeedRotationLion`, `CurrentLionSpeed`, `VerticalSpeed`, `Angles`, `AngleNormal`, `model_pos`, `my_pos`, `Ombre_Syn` and `V/V1/V2` are **globals** reused by every lion in turn.
  - Event handlers that run during *another* entity's move (a wasp impact, for example) read the previous lion's `DistLionPlayer` (sound volume) and whatever `Ombre_Syn` was last set to (`lion.wdl:350,364,468,604`). The `ennemyResistanceOrg` comparison can therefore read the wrong shadow [R].
  - Port: use the lion's own shadow fields and its own distance.
- `CALL` of an action that waits (`Warning_Attack`, `CallFriendsToHelp`, `CreateCirclesCallFriends`) runs the callee as a parallel coroutine; the caller continues (`docs/wdl-porting.md` §2).
- `BRANCH LionDeath` from inside `AnimLion` (itself `CALL`ed by `LionMarche`) replaces only the AnimLion coroutine. `LionMarche` keeps looping, and `EnnemyBehavior` returns immediately on state 7. The respawn relies on this, since LionDeath ends by setting state 1 and `LionMarche` resumes the AI [I].

---

## 3. State machine

### 3.1 Table

Animation scenes (`mm1.wdl:14-21`): stand="stand", standB="AltStand", walk="walk", run="run", rugit="rugit", attack="attack", hit="Hit", death="death".
- `cycle` = `set_cycle` (looping, interpolating last→first). `frame` = `set_frame` (one-shot).
- "fade" = cross-fade from the current frame into frame 0 of the scene, at `prev_step += anim_speed×k×time` (§3.2).

| State | Meaning | Entered from (cite) | Per tick | Exits (cite) | Animation |
|---|---|---|---|---|---|
| **0** wait | standby, no movement except gravity/platform | init (`lion.wdl:198`); 1/2 beyond 1.1×react (:984); 4 at cycle end with 95% (`anims.wdl:162-165`); teleport forces it each tick (`mm1.wdl:4256,4272`) | §2.2 0/4 branch | → **1** if dist < react_range (:1760). → **4** with 10% at each cycle end (`anims.wdl:30-33`). → 11/6 by wasp. → 2 via friend call (:958). | stand cycle, speed 3 (×2 effective: AnimLion runs twice per tick) |
| **1** walk / patrol / wander | moves forward when `|BallDeltaPan| < 10` | 0/4 react; 8/9 sighting (:1723); 2/10 beyond attack+128 (:2075); end of roar (`anims.wdl:145`); flee over (:2149); RandomizeLion v=2; EnnemyOnBlock patrol 2/12 → 1 (:1476, :1489, :1500); respawn (:726); StartEnnemy | patrol (§2.5) / random / straight; chase test on LOS frames | → 2 on LOS ∧ random(100)>50 ∧ dist ≤ attack ∧ \|tilt\|<45 ∧ (not busy or random mode) (:2023-2065). → 0/4 beyond 1.1×react (:981). → 12 fear (:2134). → 8/13 random. → 3 never directly. | fade(walk, k=6) then walk cycle at `0.82·Speed` (scorpions ×3) |
| **2** chase | runs at `Speed·BallAccel` (BallAccel ≈ AccelAttack) toward `dist.pan` | chase start (:2053); friend call (:964); hit-reaction end (`anims.wdl:300`); swipe end (`anims.wdl:253`) | homing (§2.2); LOS every 4 (scorpions 8) frames; `GuardStuck` jitter | → **10** on touching Sammy (`lion.wdl:507-521`, `player.wdl:573-583`). → 1 if dist > attack+128 (:2072) or a patrol lion hits a wall (:1476/:1489). → 0/4 beyond 1.1×react (not during a call). → 3 stuck roar (:363). | if `Speed·BallAccel < 18 && avgfps > 30`: fade(walk, k=5) then walk cycle stepping `anim_speed·time·max(CurrentLionSpeed·0.4, 1)` (scorpions `×6`). Otherwise fade(run, k=10) then run cycle (scorpions ×7). `anim_speed = 0.3·Speed·AccelAttack`. |
| **3** roar | no steering, no forward motion; gravity/knockback only | stuck roar (:363); hit-reaction with Flag7 ∧ res ≤ 75% (`anims.wdl:280-286`, anim_speed **not** reset: it stays the hit speed) | `HandleSpeedLion` then `BallAccel` relax | → **1** at anim_step > 100 (`anims.wdl:143-148`) | fade(rugit, k=6) then rugit cycle at 4 |
| **4** alt-stand | same as 0 | 0 at cycle end 10%; 1/2 drop-out 50% (:988) | as 0 | → 0 at cycle end with 95% (`anims.wdl:162`). Otherwise as 0. | AltStand cycle, 3 (×2) |
| **6** lethal hit | knockback slide (`BounceForce1` from the hit), hop `BallVspeed=20, fric 3.5` | resistance ≤ 0 from wasp (:451-459, :586-594), lava (`mm1.wdl:4923-4933`) or death tile (`mm1.wdl:4069-4074`) | if the head vertex is inside a solid: `BallAccel = 0`. Each AnimLion tick sets `event = null`, `enable_scan = 0`, `IdEnnemy = 0`. Passable after step > 50. | → **7** (`LionDeath`) at step > 100 (`anims.wdl:184-188`) | death **frame** (one-shot) at 4/tick ⇒ 25 ticks. Pan was set to face the attacker: `ang(BallDestPan−180)` (:453). |
| **7** dying / dead | nothing (`EnnemyBehavior` returns) | `LionDeath` (:647) | blink then respawn wait (§5.3) | → 1 on respawn (:726), or the loop ends (Flag1 = 0) | none (falling check only) |
| **8** random stand | still; LOS test every tick once step > 50 | RandomizeLion v=1 | §2.2 8/9 branch | → 1 on sighting within attack range (:1709-1727). → 1/13 by re-roll. | fade(stand, 6) then stand cycle 3 (×2) |
| **9** random alt-stand | as 8, and also allowed to start a chase directly (:2023) | **unreachable** in practice (only reachable via `StartEnnemy` Skill3 or `DeathDalle` Skill5) | as 8 | as 8 | fade(AltStand, 6) then cycle 3 (×2) |
| **10** swipe | lunges at `Speed·BallAccel·1.5` | 2 touching Sammy (`lion.wdl:512`, `player.wdl:575`); 2 touching a weak door (`mm1.wdl:4511`) | homing as in 2; damage on contact at step > 60 (§4.1) | → **2** at step > 100 (`anims.wdl:238-254`), with sound hit1/2/3 at 1000 (lions) or ScorpionAttack (scorpions). → 1 if dist > attack+128. | attack **frame** at the inherited run speed `0.3·Speed·Accel` (e.g. 9%/tick, about 11 ticks). No fade. |
| **11** hit reaction | still (speed 0), hop `BallVspeed = 20·weight`, fric 3.5 | wasp hit, not lethal, res < 95%·org (:468-474, :604-610) | normal active branch (RandomizeLion may interrupt it in random mode) | at step > 100 (`anims.wdl:262-304`): random sound; **AttackRange×1.5, ReactRange×1.5 (cumulative)**. Then Flag7 ∧ res ≤ 75%·org → **3**; else res ≤ 50%·org → **12**; else → **2** | Hit **frame** at `clamp(10·res·0.3, 5, 20)` |
| **12** flee | runs at `Speed·BallAccel·1.2`; TestDist ignores the player (dist = 1) | fear (:2136); hit reaction ≤ 50% (`anims.wdl:290`); lava `EnnemyFuit` (`mm1.wdl:4944`) | fear decay; jitter (practically never) | → 1 when `0 < EnnemyAttacked ≤ V` (:2147). → 1 on a wall if patrol (:1494). → 3 stuck roar. If `EnnemyAttacked ≤ 0` the fear block is skipped and **the flee persists** until one of those. | like 2: walk if `Speed·BallAccel < 18 && fps > 30`, else run |
| **13** random run | runs at `Speed·BallAccel` in a random heading | RandomizeLion v=3 | jitter ±10° when `GuardStuck > 9` | re-roll; stuck roar → 3. **Cannot start a chase** (needs 1 or 9). | walk if `Speed·BallAccel < 15 && fps > 30` (fade k=6, rate ×max(CurSpeed·0.4,1)), else run (fade k=10) |
| 1001 / 1002 / 1004 | frozen corpse (an external script set Flag1 = 0) | death tile (`mm1.wdl:4075-4076`), DeathDalle (`mm1.wdl:3590`) | loop exits; the entity stays visible with its event and collision | none | none |

### 3.2 `AnimLion` details (`anims.wdl:2-350`)

- **Fall override (:5-20).** If `z − haut_sol > 250 && !flag8`:
  - while `prev_step ≠ 100`: `next_frame = 152`, `frame = int(frame) + prev_step·0.01`, `prev_step += anim_speed·0.2·time`;
  - then `frame = 152` (a fixed "attack step" frame).
  - This runs **before** any state animation, including death.
- **Cross-fade:**
  ```
  temp = frame;
  set_frame(scene, 0);
  next_frame = frame;
  frame = int(temp) + prev_step·0.01;
  prev_step = min(prev_step + anim_speed·k·time, 100)
  ```
  - **`prev_step` starts at the *old `anim_step`*,** not 0 (`Ombre_Syn.LionAnimPreviousStep = My.LionAnimStep` at every transition). A transition taken late in a cycle therefore fades almost instantly.
  - Transitions that don't write `prev_step` (TestDist → 0/4, 3 → 1, 10 → 2, 11 → *, Lion_Hit and EnnemyOnBlock changes, fear 1 → 12) keep the old value, normally 100, which means no fade.
  - The shadow's `prev_step` is 0 at init, so the first walk fades in.
- `anim_step` wraps at >100 → 0 for cycles. For 0/4 the wrap triggers the 0↔4 dice.
- **States 0/4/8/9 animate twice per tick** (`EnnemyBehavior` :1734/:1781 and `LionMarche` :273). The effective stand rate is 6%/tick. The 0↔4 dice also runs per wrap.
- **Sounds:**
  - Chase start: lionatt1 (lions, 50%) or scoratt1 (scorpions, 50%).
  - Stuck roar: tigre (+ lionatt1 with Flag7 and no call active).
  - Swipe end: hit1/2/3 (lions, random) or scoratt1.
  - Hit-reaction end: lionHit1 / lionHit2 / tigre (1/3 each) via `play_sound` at 100, tuned with vol `30000/dist`, freq `200·weight + random(50) − 25`.
  - Scorpion giving up a chase: scorwalk.
  - The tuned `play_sound` calls (2D sounds, distance-scaled volume) are `lion.wdl:349-353`, `:2037-2041` and `anims.wdl:268-273`. The others are `play_entsound` (3D) with the given range (1000-2000).

---

## 4. Interaction with Sammy

### 4.1 Damage (`WindowHearts`, 0..48)

There are three collision paths. Damage ≤ 0 → `my = Sammy; branch Player_Kaputt` (`lion.wdl:501-505`, `player.wdl:568-571`). Lion type 8888 only.

| Situation | Handler | Damage | Guard |
|---|---|---|---|
| Lion in **2** touches Sammy (either mover) | `lion.wdl:507-521` (lion moved, EVENT_ENTITY) / `player.wdl:573-583` (Sammy moved, EVENT_ENTITY on Sammy) | `res × 0.25`. The lion switches to **10**, `anim_step = 0`, `shadow.BaffeDonne = −999`. The lion side also emits 1 `particle_Hit1` (TailleFumee4 = 500). | once (the state changes) |
| Lion in **10**, `anim_step > 60`, `BaffeDonne == −999` (the swipe connects) | `lion.wdl:491-506` / `player.wdl:558-572` | `res × HargneEnnemy` (2). Plays hit1/2/3 at 2000 range; the lion side also emits 10 `particle_Hit1` (size 1000). `BaffeDonne = 0`. | **once per swing**, shared by both handlers |
| Sammy moves into a lion in **10** with `anim_step > 60` (lion EVENT_IMPACT) | `lion.wdl:622-636` | `res × 0.25` **on every such impact** | none [R] |

- **Examples:** nano (res 6): bump 1.5, swipe 12. Punk (res 30): bump 7.5, swipe 60, which kills from full health.
- Damage scales with the lion's **current** resistance, so wounded lions hit softer.
- After a respawn (resistance ≤ 0) hits deal ≤ 0, which means they **heal** [R; moot in shipped data].
- `EVENT_IMPACT` on Sammy (a lion moved into him) has the damage code commented out (`player.wdl:651-682`). Lion-initiated contact is handled by the lion's EVENT_ENTITY.
- Engine order between the mover's ENTITY event and the target's IMPACT event within one MOVE is unverified [?].

### 4.2 Knockback on Sammy (`Player_Bong`, `player.wdl:520-620`)

- Lions have `BounceForce (ObjetPoussant) ≠ 0` (default 2). So whenever Sammy moves into a lion, Sammy's knockback is:
  `Sammy.BounceForce1 = lion.EnnemySpeed(X,Y,Z) × Sammy.WeightForce (0.8) × lion.BounceForce` (:537-542).
  - `EnnemySpeed` is the lion's last velocity in quants/tick (§2.3). A stationary lion that is blocked has `0.1/mintime` along its pan.
  - The `IntensiteBlobBounce` (30) / normal path (:543-555) applies only to entities with BounceForce 0, never to lions.
- When a lion moves into Sammy, Sammy gets EVENT_IMPACT (:625-643) with the same `ObjetPoussant` formula.
- **Stomp** (:584-618): Sammy hits a lion while above its top (`Sammy.z > lion.z + lion.max_z`), with `BounceOnEnnemy ≠ 0` (per-level; default 0), `BlobVspeed ≤ 0` and (`PlayerEtat ∈ {4,5}` or `BlobVspeed < 0`). Then:
  - knockback ×2;
  - Sammy re-jumps with `BlobVspeed = BounceOnEnnemy × BlobBallVerticalSpeed`;
  - Blouit sound.
  - **The lion takes no damage.**
- **Lion knockback from Sammy:** `lion.BounceForce1.xy = Sammy.PlayerSpeed × lion.weight × 1.8` (`lion.wdl:395-399` / `:535-538`).

### 4.3 IDs and scans involving enemies

| Id | Who / where | Effect on enemies |
|---|---|---|
| skill1 3333 | Sammy (`player.wdl:40`) | the target of damage and LOS tests (`You.Idplayer == 3333`) |
| skill1 8888 | lions and scorpions (and wasp enemies, `bullets.wdl:1533-1611`) | damage-capable class; doors with Flag3 open for them (`mm1.wdl:3097`); `DoorScanEntities` counts them (`mm1.wdl:3010`); gravity treats them as "not floor" (`lion.wdl:1306`) |
| skill1 1111 | elephant | as 8888 for doors, lava and weak doors, but no damage and immune to wasps |
| skill1 9999 | Sammy's wasps | hit the enemy (§5) |
| 507, 77 | terrain prop / projectile ids | the gravity trace doesn't adopt them as floor |
| scan 1 | Sammy's pickup scan (`player.wdl:221-225`) | ignored (Lion_Hit returns on scans) |
| scan 2 | wasp seek cone (`bullets.wdl:390-396`) | the lion is *detected*. Wasp_Hit: if `lion.EnnemyAttacked ≤ 0` and the wasp hasn't locked yet: `lion.EnnemyAttacked += WaspPoison` (this pre-fear can trigger fleeing before impact) and the wasp homes on `shadow.muzzle − (0,0,32)` (or `lion.pos + (0,0,32)`) (`bullets.wdl:657-695`) |
| scan 25 | Sammy's door/camera scan (500 sphere, `player.wdl:231-238`); lions with Flag5 | lions with Flag5 open Flag3 doors and trigger camera zones (extro) |
| scan 27-99 | lion path scans | §2.5 |
| scan 100 | lion with Flag6 | gems explode (§6.6) |
| scan 102 | door clear-check (`mm1.wdl:2934`) | a door won't come down while a 3333/8888/1111 entity is within range (`mm1.wdl:3007-3013`) |
| scan 1000 | `RestartSammy` (`player.wdl:886-891`) | resets gems. Enemies ignore it. `RestartSammy` also sets `AttackCall = 0`. |
| skill1 = 0 | end of game (main loop exit) sets `pov.IdPlayer = 0` (`mm1.wdl:911`); teleports also zero Sammy's id while fading | LOS never succeeds (no 3333) |

**`pov_Syn`** is the target of every distance and LOS computation. It is Sammy (`player.wdl:31`), except in the extro, where `InitSetPovSyn` (`mm1.wdl:5404-5450`) retargets the camera and all lions to a lion.

### 4.4 Warning (`Warning_attack`, `player.wdl:1091-1106`)

- **Condition:** `AverageFps > 35 && video_depth > 8 && Sammy.lightrange == 0`.
- Sammy gets a dynamic light (RGB 250,100,0) with range `LightRange1` (300) that shrinks by 5/tick to 100, then turns off.
- It runs as a parallel coroutine. Port: an orange point-light pulse on Sammy lasting about 40 ticks.

### 4.5 Calling friends (`CallFriendsToHelp`, `lion.wdl:998-1070`)

- **Only caller:** the stuck roar with Flag7 (§2.8 step 2).
- **Preconditions:** `AttackCall == 0` and `AnswerFriends ≠ 0`.
- **Effect:**
  - `AnswerFriends = 0`; `AttackCall = TempoCall` (128 ticks); `FriendCall = spec + weight` (e.g. 1.65 = nano, 1.55 = rocker, 3.75 = scorpion);
  - spawn three `circl100.mdl` rings at the lion: z+50 (skin 1), then after 8 ticks z+85 (skin 3), then after 10 more ticks z+150 (skin 1);
  - each ring: flare, passable, starts at scale 0.01 and frame 1; it grows `+0.2·time` until scale 3, then `+(5.1−s)·0.5·time` until 5, while the frame advances `+4·time` up to 100; then it is removed (:1015-1070);
  - `AttackCall −= time` each tick until 0 (its own coroutine).
- **Responders:** while `AttackCall ≠ 0`, each lion's `TestDistLionPlayer` (§2.6) checks for same `FriendCall`, `128 < dist < RangeCall` (10000) and state ∈ {0,1,4,8,9}. Such a lion enters **2** (chase) and, if it was beyond its attack range, `AnswerFriends += 1`.
  - All matching lions within range have `dist` forced to 64 for the tick, so they can't give up the chase during the call.
  - **No lion reverts to idle while any call is active.**
- **Lockout:** if nobody farther than their attack range answered, `AnswerFriends` stays 0 and **no lion will ever call again**. It is never reset, not even per level [R].

### 4.6 Eye of Truth range rings (brief)

- **Pickup:** `EyeOfTruth_Hit` (`mm1.wdl:5018-5065`) sets `EyeOfTruth = 1` for `DureeEyeTruthOrg` ticks (256), then the panel shrinks. Death of Sammy clears it (`player.wdl:780`).
- **`DisplayEnnemyRange`** (`lion.wdl:1586-1653`) runs every tick from `LionMarche`. It first hides the ring, then shows it if `EyeOfTruth ≠ 0` and one of:
  - `EnnemyMustDisplayRange > 1`;
  - id 8888, `shadow.EnnemyAvuPlayer ≠ 0` and (state 1 or random mode).
- **Ring position:** `(x, y, z + CircleZ)`.
- **Ring when shown in state 1 or random mode:**
  - skin 1, or skin 2 if `dist < attack + (react − attack)/2`;
  - `frame = clamp(100·dist/attack, 60, 99)`;
  - `scale_xy = attack/1000`.
- **Ring in state 2:**
  - skin 2, or skin 3 if the lion has seen Sammy;
  - radius = attack, or `dist + 128` if beyond it;
  - frame clamp low bound 55;
  - `scale = radius/1000`.
- `DisplayRange` (:1654-1677) is called for idle states when `EnnemyMustDisplayRange == 1`. **Its result is immediately re-hidden** by `DisplayEnnemyRange` in the same tick, unless the conditions above hold. Net effect: idle lions show no ring unless they have seen Sammy and are random-mode lions [R].

---

## 5. Being hit by wasps

### 5.1 Hit (`lion.wdl:441-483` ENTITY, `:561-621` IMPACT)

Applies only when `IdEnnemy == 8888`; the elephant is immune. `poison = wasp.WaspPoison` comes from the pickup Skill3 (`bullets.wdl:131,138`; defaults by type 3,2,1,1,4,6,1 per `gameplay.md` §9).

```rust
// IMPACT only (:567-579): idle lion woken by a wasp
if etat == 0 || etat == 4 {
    if shadow.attack_range_bis != 0 { attack_range = bis; bis = 0; }
    let v = (frc(react_range) * 10000.0 + 0.5) as i32; if v != 0 { react_range = v as f32; }
}
wasp.flag1 = 0; wasp.wasptat = 1004;                 // wasp dies (it sets 1001 on itself too, bullets.wdl:701-702)
ennemy_resistance -= poison;
if ennemy_resistance <= 0.0 {
    if etat != 6 {
        pan = ang(ball_dest_pan - 180.0);            // ball_dest_pan was just set to the wasp's pan (IMPACT) → faces the wasp
        anim_step = 0.0; anim_speed = 4.0; ball_vspeed = 20.0; ball_fric_vspeed = 3.5; etat = 6;
    }
} else if etat != 11 {
    ball_vspeed = 20.0 * weight; ball_fric_vspeed = 3.5;   // hop
    ennemy_attacked += poison;                        // fear
    if ennemy_resistance < shadow.resistance_org * 0.95 {
        anim_step = 0.0; anim_speed = (10.0 * ennemy_resistance * 0.3).min(20.0).max(5.0); etat = 11;
    } else {
        bounce_force1.x *= K; bounce_force1.y *= K;   // K = 2 (ENTITY) / 2.5 (IMPACT): a stronger shove instead of a flinch
    }
}
```

- **Knockback** is the collision push computed just before (§2.8): `wasp.my_speed × lion.weight × wasp.BounceForce`. The wasp's Skill37/38 hold its last per-frame displacement (`bullets.wdl:365-368`); its BounceForce is 1-3 (`bullets.wdl:226-310`). The lion turns to the wasp's heading on IMPACT.
- **Interrupting states:** a hit during **11** subtracts resistance but neither restarts the flinch nor adds fear or a hop. A hit during **6** only subtracts.
- **Hit-reaction end** (`anims.wdl:262-304`):
  - ranges ×1.5;
  - Flag7 ∧ res ≤ 0.75·org → roar (3);
  - else res ≤ 0.5·org → flee (12) with heading `pan − 180 + random(180)` and `random_time = TFN×1.5`;
  - else → chase (2) with `random_time = TFN×2`. **Note:** this makes a wounded lion chase Sammy without needing LOS.
- **Fear** (§2.2 :2128): flee when `EnnemyAttacked > |res·weight·0.5|`. It decays by `0.05·|res·weight·0.5|` per tick, about 5% of the threshold per tick.
  - Example: a full-HP nano (threshold 1.95) flees after a poison-3 wasp is *detected* (pre-fear +3).

### 5.2 Death (`AnimLion` state 6 → `LionDeath`, `lion.wdl:645-740`)

- **State 6:** the death frame plays over 25 ticks. Each tick it nulls the event, disables scans and sets `IdEnnemy = 0`. It becomes passable at step > 50.
- **`LionDeath`:**
  - event null, state 7, `IdEnnemy = 0`, `BounceForce1 = 0`, passable;
  - blink: invisible/visible toggles with waits 4,4,4,4,3,2,2,2,2,2,2,2 (33 ticks), ending invisible;
  - shadow and ring hidden;
  - `LionNbrVie −= 1`. If ≤ 0: `Flag1 = 0` and the loop ends. **The entity is never removed**: it stays invisible and passable with no event.

### 5.3 Respawn (lives > 1; never used by shipped data)

1. Wait `int(TempoStartBall)` ticks if nonzero.
2. Wait in 16-tick steps while Sammy is within attack range. The distance uses `pov.z + 15` the first time and `pov.z` afterwards.
3. Show the shadow **and the ring** (the ring is then re-hidden each tick by `DisplayEnnemyRange`).
4. Become visible and solid at the spawn origin. State 1, `anim_step 0`, walk speed using the *old* SpeedEnnemy.
5. **`IdEnnemy = 8888` even for an elephant**; `event = Lion_Hit`; `enable_scan = 1`.
6. Face Sammy (pan and BallDestPan), then `SpeedEnnemy += 1`.
7. **Resistance is not restored** (`:691-694` is inside `IFDEF sammynewversion`, undefined; the line even has an unbalanced parenthesis). So:
   - the next wasp kills;
   - the fear threshold is 0, so any fear means flee;
   - hits on Sammy deal ≤ 0.
   - `BounceForce` is unchanged; the attack/react ranges keep any ×1.5 growth.

### 5.4 Other deaths and removals

- **Lava** (`mm1.wdl:4886-4937`), via the gravity trace sonar:
  - **Grounded condition (alias bug):** `lion.z ≤ lion.Skill18 (AnimStep) + lion.Skill30 (RandomTime2) + 41 && lion.Skill35 (BallVSpeed) == 0`. These are the *player's* field aliases applied to a lion [R bug]. The condition is effectively "world Z is low".
  - **Spec 2 (elephant; the comment wrongly says scorpion):** `res −= 0.1·time`; at ≤ 1 → `EnnemyFuit`.
  - **Others:** `EnnemyFuit` (state 1/2 → 12, heading `pan − 180`, run anim **without** AccelAttack, `random_time = TFN`) and `res −= lava.WeightForce·time`.
  - At ≤ 0 → state 6.
- **Death tile with Flag2** (`mm1.wdl:4066-4077`): same alias condition. Sets state 6 then immediately `Flag1 = 0, LionEtat = 1004`. The lion **freezes visible** (its loop ends) without the death animation.
- **DeathDalle** (`mm1.wdl:3578-3592`): if `IdEnnemy` ∈ {Skill2, Skill3, Skill4} or the dalle has Flag1, and `lion.z − dalle.z ≤ Skill6`: `Flag1 = 0, LionEtat = Skill5` (frozen).

---

## 6. Movement and collision details

### 6.1 Hull and MOVE

- **Hull:** `fat = 1, narrow = 0` (`lion.wdl:140-141`); `push = −3`.
- **Forward move:** `MOVE me, (fwd,0,0) relative, (0,0,vz) absolute` (`lion.wdl:1103`), where `fwd` is time-scaled by the script.
  - "Relative" is rotated by pan/tilt/roll, so a patrol lion with non-zero tilt (node tilt easing) moves along its tilt.
  - It glides along walls and fires BLOCK/ENTITY/IMPACT.
- **Knockback move:** absolute `BounceForce1·mintime` with events disabled (:1166-1179).
- **Platform move:** absolute `LionForce`, events disabled (:1214-1218).
- **Idle 0/4:** only `MOVE me, 0, (0,0,vz)` (:1784).
- **`enable_block`** is off while `|BallDeltaPan| > 6` (:2206-2213), so walls give no BLOCK events during fast turns.

### 6.2 Pushing between entities

- Lion ↔ lion: both use `other.EnnemySpeed × my.weight × other.BounceForce (2)`.
- Lion ↔ Sammy: §4.2.
- Lion → moving platform (`PlatForm_Hit` IMPACT, `mm1.wdl:2307-2314`): `lion.BounceForce1.xy = −lion.EnnemySpeed.xy × platform.BounceForce`, i.e. the lion bounces back.

### 6.3 Doors

- Flag3 doors open on a scan-25 event from a 8888/1111 entity (`mm1.wdl:3095-3098`). Only lions with Flag5 emit scan 25.
- A descending door waits while a lion is within range (scan 102).
- Doors have `push −2` when `DoorPassable` (`mm1.wdl:2895`), which is higher than the lion's −3, so lions collide with doors.

**Weak doors** (`DoorWeak_Hit`, `mm1.wdl:4487-4520`; the lion is the mover, the door gets IMPACT):
- `door.WeightForce ≤ 0.4`: explode immediately. Door damage = `You.WaspPoison` = the **lion's EnnemyResistance**.
- Lion in 2: the lion goes to **10** with `anim_step 0`, **`EnnemyAttacked = −999`** (an old flag kept by mistake). That makes the lion fearless: the fear block needs > 0, and wasp pre-fear only adds when ≤ 0, so it climbs back from −999 by poison amounts.
- The "state 10 and step > 60 and `EnnemyAttacked == −999`" branch then also explodes the door.

### 6.4 Teleports, platforms and death tiles (all via the gravity trace's `ACTIVATE_SONAR`)

- **Teleport with Flag2** (`mm1.wdl:4236-4285`):
  - **Condition:** `lion.z ≤ HautSolBall + Skill30 (RandomTime2, alias) + 41 && BallVSpeed == 0`, and the teleport is not busy (`skill12`).
  - **During:** `IdEnnemy = 0`, passable, `enable_scan = 0`, transparent; alpha 100 → 0 at −2/tick (50 ticks), forcing `LionEtat = 0` every tick; jump to (Skill1, Skill2, Skill3); shadow hidden; alpha 0 → 100 at +1.5/tick (~67 ticks), still forcing state 0.
  - **After:** restore id, scan flag and solidity; show the shadow.
- **Moving platform** (`mm1.wdl:2383-2412`): if the platform motor is moving and `lion.z ≤ HautSolBall + Skill30 + 41`, then `lion.LionForce.xy = platform.BounceForce1.xy` (its per-tick motion), applied by `HandleForcesOnEnnemy` and decaying via `accel(…, 0.2)`.
- **Death tiles, lava, DeathDalle:** §5.4.

**Port note:** the `Skill30` aliasing makes these thresholds noisy. A faithful-but-sane port uses "grounded (`ball_vspeed == 0`) and `z ≤ floor + hover + 41`" and flags the difference.

### 6.5 StartEnnemy triggers (`InitStartEnnemy`, `mm1.wdl:5126-5170`)

An invisible trigger scans a sphere of radius Skill2 every 8 ticks with `enable_detect`. For each detected entity whose `IdEnnemy == Skill1`:
- `LionEtat = Skill3`. If that is 1 and the entity is 8888, it also gets `anim_step = 0` and the walk speed.
- `Skill5` ≠ 0 → react range.
- `Skill6` ≠ 0 → attack range.
- Then it stops (Skill4 == 0) or re-arms after Skill4 ticks.

Shipped use, `180c_t`: `[8888, 300, 1, 0, 10000, 600|500]` → wake lions into walk with react 10000 and attack 500-600.

### 6.6 Gem crushing (Flag6)

- Every 4th frame in active states: scan id 100, sphere of radius `30·scale_z` (`lion.wdl:1803-1809`).
- Gems (`Key_Hit`, `gems.wdl:224-257`) respond:
  1. `GemExplose`, then shrink ×0.5/tick to 0.05, then hide.
  2. If gem Skill3 = 0: gone. Otherwise reappear after Skill3 ticks and grow +0.1/tick back to the stored scale.
- No shipped entity uses Flag6.

---

## 7. Minimum viable port

**Phase 1 (believable lions).**
1. **Loader.** Unpack §1.1 with f32 math and apply type defaults (§1.2). Spawn at `floor + 28·scale` (or 15·scale) and keep `hover = HautBall`. Shadow decal.
2. **Tick at 16 Hz.** Implement:
   - states 0/4 (idle, react by distance, 10%/95% stand alternation);
   - state 1 as "walk straight, turn only when bumping" (the no-path behaviour, which is what level 10004 actually does);
   - state 2 chase (LOS raycast throttled 10/4 frames, `random(100) > 50`, attack range, |tilt| < 45, homing `BallDestPan = dist.pan`, give up at attack+128, idle at 1.1×react);
   - rotation `pan += ang(dest − pan)·0.2`;
   - walk only when `|delta| < 10`.
3. **Mover.** Collide-and-slide with ground snap at the hover height and gravity −4 / −3 per tick. Wall bounce = EnnemyOnBlock (normal push of 15·…, re-steer ±25-45°). Knockback channel `BounceForce1` with linear decay ≈ 15.7/tick.
4. **Swipe.** Touching Sammy in 2 → state 10: lunge ×1.5, `res·0.25` on contact, `res·2` once when `anim_step > 60`, back to 2 at 100. Sammy knockback = `lion_vel·0.8·2`.
5. **Wasp hits.** Resistance, flinch (11) below 95%, hop, death (6 → blink 33 ticks → hide, no respawn). Use the Lion_Hit numbers.
6. **Animations.** stand / walk / run / attack / Hit / death, with the §3.1 rates. The cross-fade can be a simple 0.1-0.2 s blend.

**Phase 2.**
- Patrol path nodes (§2.5), including the "last scanned node wins" quirk and the 64-tick rescan.
- Random mode (8/13/1 dice).
- Fear and flee (12), including wasp pre-fear on detection.
- Hit-reaction branches (roar 3 and flee 12) with the ×1.5 range growth.
- Second attack/react ranges.
- Stuck accumulator and roar (`GuardStuckBalle > 10` with LOS) with the 16-tick turnaround.
- Muzzle wall probe (head vertex 109/135/132/139/209, 32 ahead).
- Sounds with distance-tuned volume.

**Phase 3.**
- Friend calls (Flag7, rings, the `AnswerFriends` lockout, the RangeCall bug).
- Eye-of-Truth rings.
- Doors (Flag5 scan 25, weak doors and the −999 fear quirk).
- Teleports, platforms, lava, death tiles (with or without the alias bugs).
- StartEnnemy triggers.
- Flag6 gem crushing, Flag8 flyers (shared with `InitWaspEnnemy`).
- Respawn (unused by shipped data).
- Extro `pov_Syn` retargeting.
- Fall-frame 152 override.
- Double-speed idle animation.

**Deliberately replicate:**
- per-level `InitLevelglobal` values (15/10, not 5/3);
- `LionAnimSpeedattack` unused (swipe at run speed);
- patrol lions without nodes;
- no LOS needed for idle → walk;
- chase-drop on walls for patrol lions.

**Safe to drop:**
- `NbrLionActifs`;
- the `GuardStuck == 8` jitter (it practically never fires);
- the ±90 BallDeltaPan in gravity (overwritten).
