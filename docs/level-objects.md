# Level objects: port gap analysis and exact specs

This document lists every entity action used by the shipped levels, says how far the Rust/Bevy port implements each one, and gives exact per-tick specs for everything missing or partial. It builds on, and does not repeat:
- `gameplay.md` (overview; §4.6 table of level objects, §6 camera, §7 gems summary);
- `enemy-ai.md` (lions; §6.4 teleports/platforms/death tiles, §6.5 StartEnnemy, §6.6 crushing);
- `wasps.md` (ammo, fired wasps, weak doors, hives §8, extro wasps §10);
- `wdl-porting.md` (engine semantics: `accel`, `MOVE`, `SONAR`, events, `ScanIdentite`).

**Conventions**
- Citations are `file.wdl:line` against `extracted/*.wdl` (CP1252; the UTF-8 copies have identical line numbers).
- Times are in **ticks** (16 Hz, `time = mintime = 1`). Distances are in quants. `random(n)` is a float in `[0, n)`.
- **[R]** read in the script, **[I]** inferred, **[D]** from level data, **[U]** uncertain engine semantics (verify against the running game).
- Level data: `scratchpad/wmb/out/*.entities.json`. Flags bit 0 = Flag1 … bit 7 = Flag8, bit 8 = editor INVISIBLE, bit 9 = editor PASSABLE.
- "Port" means `crates/sammy/src` as of this writing (`actions/mod.rs::init` and the modules next to it).

**Entity-skill aliasing matters.** Many specs below depend on the shared `Define`s in `mm1.wdl:175-420`. The ones that recur:

| Slot | Player/gem/platform name | Lion name |
|---|---|---|
| Skill1 | `IdPlayer` (Sammy 3333, gem 7345, wasp 9999, door/platform 90273) | `IdEnnemy` (8888 lion, 1111 elephant) |
| Skill2 | `SpeedBalle` (Sammy: WED walk speed, 25 typical) | `SpeedEnnemy` |
| Skill4 | `BlobVSpeedOrg` (= gem spin speed Skill4!) | |
| Skill18 | `BlobHautSol` | `LionAnimStep` |
| Skill20 | `PlayerDestPan` (= gem bounce counter Skill20!) | `BallDestPan` |
| Skill22-24 | `PlayerForceX/Y/Z` (platform carry) | `LionForceX/Y/Z` |
| Skill25-27 | `BounceForce1X/Y/Z` (knockback; platforms: per-tick motion) | `BounceBallX/Y/Z` |
| Skill35/36 | `BlobVSpeed` / `BlobFricVSpeed` | `BallVSpeed` / `BallFricVSpeed` |
| Skill37-39 | `PlayerSpeedX/Y/Z` (= gem scale copies Skill37-39!) | `EnnemySpeedX/Y/Z` |
| Skill40/41 | `HautBall` / `HautSolBall` (gems) | same |
| Skill43 | `PlayerKey` (gems: "being carried" flag) | `EnnemyResistance` |
| Skill44 | `ObjetPoussant` = `BounceForce` | |
| Skill46 | `PlayerOldKey` (boulder: gem fate) | `TimeWithoutPatrol` |

---

## 1. Gap table

### 1.1 Headline numbers

- **38 levels with entities** (36 game levels, `extro`, `menu_greenpepper`); 7267 entities; **57 distinct action names** (56 plus "no action"). Every name resolves to an `Action` in the scripts.
- Status by action name: **23 implemented**, **7 partial** (`InitKey`, `InitDoor`, `InitLion`, `InitTeleport`, `InitPlatForm`, `InitEnginePlatForm`, `InitCamera1`), **20 missing**, **7 marker-only** (three of which hide gameplay values: `InitGame4`, `InitGame5`, `InitCamera3`).
- Of the 20 missing: 7 are used in normal game levels (`SetCamera`, `InitArbre`, `InitMDL`, `InitBoulder`, `InitHive`, `InitStartEnnemy`, `OnGround`), 5 are extro-only (`StartExtro`, `InitWaspEnnemy`, `InitSetPovSyn`, `InitSetAnimPlayer`, `InitSetFinExtro`) and 8 are menu-only.

### 1.2 Every action in the shipped levels

Sorted by entity count. "Status" is the port's status for gameplay behaviour; the model is always rendered unless the action hides it.

| Action (spellings in data) | Entities | Levels | Main files | Status | What is missing / notes |
|---|---|---|---|---|---|
| `initpath` | 1892 | 22 levels: 10006, 10007, 10008, 10009, 10010, 10011, 101o_j, 102N_t, 103J_j, 104M_j, 106g_t, 108i_j, 110n_t, 111r_j, 112e_j, 174a_j07, 175c_t, 176g_t, 178d_t, 179f_t, 180c_t, extro | setpath.wmb 1826, spheran2.mdl 40, cliprang.mdl 17 | **implemented** | Lion patrol nodes (`enemy.rs`). Hidden-but-alive. |
| `initbonus3` | 967 | all 36 game levels | gemgr+17.pcx 967 | **implemented** | Diamonds (`mm1.wdl:1200-1291`). |
| `setcamera` | 741 | 21 levels: 10012, 103J_j, 107i_j, 108i_j, 109o_t, 111r_j, 112e_j, 113q_t, 114t_t, 170b_t00, 171c_t04, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, 180c_t, extro, menu_greenpepper | setcam.wmb 525, cliprang.mdl 180, spheran1.mdl 29 | **missing** | Treated as a marker (hidden, no behaviour). §2.12. |
| `initarbre` | 668 | 17 levels: 10001, 10004, 10005, 10007, 10008, 101o_j, 103J_j, 104M_j, 113q_t, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 178d_t, extro, menu_greenpepper | herbes1.pcx 518, tree1b.pcx 121, tree2.pcx 29 | **missing** | Spawned as a camera-facing billboard at the editor origin. Missing: ground snap, crossed (oriented) sprites, Flag1 facing, Flag4 random scale, Skill7 alpha, ×0.3 trunk hull. §2.13. |
| `initammo1` | 633 | 22 levels: 10005, 10006, 10007, 10009, 10010, 101o_j, 102N_t, 103J_j, 104M_j, 106g_t, 107i_j, 111r_j, 112e_j, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, 180c_t | wasp1.mdl 633 | **implemented** | wasps.md §2. |
| `initKey` / `InitKey` | 307 | 33 levels: 10002, 10003, 10004, 10005, 10006, 10007, 10008, 10009, 10010, 10012, 102N_t, 103J_j, 104M_j, 106g_t, 107i_j, 108i_j, 109o_t, 110n_t, 111r_j, 113q_t, 114t_t, 170b_t00, 171c_t04, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, 180c_t, extro | gemye+17.pcx 83, gembl+17.pcx 64, gemre+17.pcx 60 | **partial** | Missing: surprise colour (Skill5, 12 gems in 180c_t), Flag7 gravity and platform carry (44 gems: 180c_t, extro wagon), Flag5 hover height, lifetime blink/explode (Skill9, boulder gems only), crush (unused), exact eject physics, PlayerOldKey timing. §2.2-2.3. |
| `initDoor` / `initdoor` | 277 | all 36 game levels | doorexit.wmb 43, dfixgree.wmb 27, dfixmag.wmb 22 | **partial** | Missing: gem flying into Flag5 doors and returning (117 doors, 29 levels), doordark morph, Skill15 validation by walking under, bells/ambient blink (Skill8>48, 11 doors in 4 levels), DoorLight on bells, DoorScanEntities radius and lions, double flash on the exit door. §2.5. |
| `InitLion` | 181 | 33 levels: 10004, 10005, 10006, 10007, 10008, 10009, 10010, 10011, 10012, 101o_j, 102N_t, 103J_j, 104M_j, 106g_t, 107i_j, 108i_j, 109o_t, 110n_t, 111r_j, 112e_j, 170b_t00, 171c_t04, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, 180c_t, extro, menu_greenpepper | nanolion.mdl 67, rockera.mdl 39, scorpion.mdl 38 | **partial** | AI mostly ported. Missing: teleports (Flag2 pads), platform carry, DoorScanEntities, StartEnnemy, extro pov_Syn retarget, Flag5 scan 25 (extro only). §2.10. |
| `initmdl` / `initMDL` | 175 | 10 levels: 10002, 10004, 10007, 10009, 104M_j, 111r_j, 172c_j06, 178d_t, extro, menu_greenpepper | grate1.mdl 102, cactus1.mdl 37, huttop2.mdl 36 | **missing** | Every MDL loops "stand"; no skin/scale/frame/overlay; cactus1 not solid. §2.13. |
| `initTeleport` / `initteleport` | 118 | 15 levels: 10008, 101o_j, 109o_t, 110n_t, 111r_j, 112e_j, 113q_t, 114t_t, 171c_t04, 173b_t05, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t | teleport.wmb 101, telelion.wmb 17 | **partial** | Sammy only. Missing lion teleport on Flag2 pads (20 pads in 171c_t04, 173b_t05, 178d_t, 179f_t). enemy-ai.md §6.4. |
| *(none)* | 107 | 10 levels: 10007, 10008, 111r_j, 113q_t, 114t_t, 173b_t05, 176g_t, 178d_t, 179f_t, menu_greenpepper | barrier1.wmb 60, border.wmb 16, bigrock2.wmb 7 | **implemented** | No action: 81 static WMBs (solid brushes, correct); 3 editor-passable MDLs should be static frame 1, not animated. §2.13. |
| `initboost` | 83 | 11 levels: 10007, 104M_j, 107i_j, 108i_j, 172c_j06, 174a_j07, 175c_t, 176g_t, 177e_j, 180c_t, extro | boost.mdl 83 | **implemented** |  |
| `initsmallheart` | 78 | 10011, 108i_j, 111r_j, 172c_j06, 174a_j07, 177e_j | heart+11.pcx 78 | **implemented** |  |
| `initplatform` / `initPlatform` | 76 | 17 levels: 10006, 10009, 10012, 107i_j, 111r_j, 170b_t00, 171c_t04, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, 180c_t, extro | platfor1.wmb 36, platgree.wmb 10, platfor2.wmb 8 | **partial** | Missing: morph to platfor1.wmb on unlock (38 coloured platforms), GemLight on unlock, no unlock sound with the master key, carrying Flag7 gems and lions, IMPACT push-back. §2.6. |
| `initDoorweak` / `initdoorweak` / `initDoorWeak` | 75 | 9 levels: 10010, 102N_t, 111r_j, 172c_j06, 173b_t05, 175c_t, 176g_t, 177e_j, 179f_t | doorwk2.wmb 24, doorwk3.wmb 23, doorwk1.wmb 14 | **implemented** | wasps.md §6.2. |
| `initmainmenu` | 59 | menu_greenpepper | mflecha1.pcx 4, icon114.pcx 1, icon173.pcx 1 | **missing** | Menu only (port has no menu). §2.15. |
| `initpathplayer` | 45 | menu_greenpepper | setpath.wmb 45 | **missing** | Menu/intro only. §2.15. |
| `initSkyMenu` / `initskymenu` | 42 | 36 levels: 10001, 10002, 10003, 10004, 10005, 10006, 10007, 10008, 10009, 10010, 10011, 10012, 101o_j, 102N_t, 103J_j, 104M_j, 106g_t, 107i_j, 109o_t, 110n_t, 111r_j, 112e_j, 113q_t, 114t_t, 170b_t00, 171c_t04, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 178d_t, 179f_t, 180c_t, extro, menu_greenpepper | sky3.mdl 13, sky2.mdl 12, sky4.mdl 6 | **implemented** |  |
| `initEnginePlatform` | 41 | 17 levels: 10006, 10009, 10012, 107i_j, 111r_j, 170b_t00, 171c_t04, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, 180c_t, extro | small.pcx 41 | **partial** | Missing: wait for PartieEncours, 4-tick lock poll, impact rule details (see §2.6). Shift ×3 is ported. |
| `initgame3` | 40 | all 36 game levels + extro + menu | small.pcx 40 | **implemented** | Two InitGame3 entities in 109o_t and 179f_t: apply both (non-zero skills only); see §2.9. Gravite/AccGravite (-12/3) must also drive gem physics (§2.1). |
| `initExit` | 39 | all 36 game levels | tileexit.wmb 39 | **implemented** |  |
| `initcamera1` | 38 | all 36 game levels + extro + menu | setcam.wmb 37, small.pcx 1 | **partial** | Type 2 (Flag4, 11 levels) and Flag1 (camera may move in Z) not ported; port always uses type 3. §2.12. |
| `initcamera2` | 38 | all 36 game levels + extro + menu | small.pcx 38 | **implemented** | Skill1/Skill4 (type-2 frictions) ignored; see §2.12. |
| `initextraballes` | 38 | all 36 game levels + extro + menu | small.pcx 38 | **marker-only** | All skills 0 in shipped data. Nothing to port. |
| `initgame` | 38 | all 36 game levels + extro + menu | small.pcx 38 | **implemented** |  |
| `initgame2` | 38 | all 36 game levels + extro + menu | small.pcx 38 | **implemented** | Camera origin skills must apply only when non-zero (§2.12). |
| `initNextraballes` | 38 | all 36 game levels + extro + menu | small.pcx 38 | **marker-only** | All skills 0. Nothing to port. |
| `initpublic` | 38 | all 36 game levels + extro + menu | small.pcx 38 | **marker-only** | DRM. Drop. |
| `initcamera3` | 37 | all 36 game levels + extro | small.pcx 37 | **marker-only** | Values ignored. Skill5 ground clearance (64), Skill8 start-sweep friction (0.6), Flag2 doors opaque (10 levels). §2.12. |
| `initmarche` | 37 | all 36 game levels + extro | suricat1.mdl 37 | **implemented** |  |
| `initgame5` | 34 | 34 levels: 10001, 10002, 10003, 10004, 10005, 10006, 10007, 10008, 10009, 10010, 10011, 101o_j, 102N_t, 103J_j, 104M_j, 106g_t, 107i_j, 108i_j, 109o_t, 110n_t, 111r_j, 112e_j, 113q_t, 114t_t, 171c_t04, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 179f_t, 180c_t, extro | small.pcx 34 | **marker-only** | **Values ignored but gameplay-relevant**: SpeedRotationPlayer/Move (Skill1/2), SlopeAngleMin 0.9 (30 levels; port hard-codes 0.92), Accpente/ForcePente/AmortiPenteEnd/BigSlope (ice level 179f_t). §2.9. |
| `initgame4` | 32 | 32 levels: 10001, 10002, 10003, 10004, 10005, 10006, 10007, 10008, 10009, 10010, 10011, 101o_j, 102N_t, 103J_j, 104M_j, 106g_t, 107i_j, 108i_j, 109o_t, 110n_t, 111r_j, 112e_j, 113q_t, 114t_t, 172c_j06, 174a_j07, 175c_t, 176g_t, 177e_j, 179f_t, 180c_t, extro | small.pcx 32 | **marker-only** | Values ignored. Skill3 clip range (render), Skill5/6 IntensiteBlobBounce 15 / AmortiBouncePlayer 7 in 179f_t (§2.8, §2.9). |
| `inittitle` | 31 | 14 levels: 10001, 10002, 10003, 10004, 10005, 10006, 10007, 10008, 10009, 10010, 10011, 10012, extro, menu_greenpepper | tut12_4.pcx 3, titre.mdl 2, tut01e5b.pcx 1 | **implemented** | Triggers on Sammy only; original triggers on any scan-25 scanner (extro credits). |
| `initbonusVie` | 27 | 13 levels: 10011, 10012, 104M_j, 108i_j, 170b_t00, 172c_j06, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, extro | lifea.mdl 23, coeur.wmb 4 | **implemented** |  |
| `initTiledeath` | 27 | 10 levels: 10012, 102N_t, 113q_t, 114t_t, 170b_t00, 175c_t, 176g_t, 178d_t, 179f_t, 180c_t | tiledeat.wmb 27 | **implemented** | No shipped tile has Flag2 (lion kill) or Flag3. |
| `initEyeOfTruth` | 22 | 108i_j, 111r_j, 174a_j07, 175c_t, 178d_t, 180c_t | eyetruth.pcx 22 | **implemented** |  |
| `initDecor` | 20 | 20 levels: 10001, 10002, 10003, 10004, 10005, 10006, 10007, 10008, 10009, 10010, 10011, 101o_j, 102N_t, 103J_j, 104M_j, 106g_t, 107i_j, 110n_t, 113q_t, 114t_t | solmenu5.mdl 17, solmenu4.mdl 2, greenhi1.mdl 1 | **implemented** |  |
| `initwaspennemy` | 19 | extro | wasp1.mdl 18, win.mdl 1 | **missing** | Extro only. wasps.md §10, §2.14. |
| `initboulder` | 12 | 112e_j, 174a_j07, 176g_t, 178d_t, extro | small.pcx 6, boulder5.wmb 4, boulder3.wmb 1 | **missing** | Rendered static. Gem emitters (5 levels). §2.4. |
| `initgame7` | 11 | 11 levels: 107i_j, 111r_j, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, 180c_t, extro | small.pcx 11 | **implemented** |  |
| `initgame6` | 8 | 8 levels: 10007, 10008, 10009, 10010, 10011, 107i_j, 177e_j, 180c_t | small.pcx 8 | **implemented** |  |
| `initrestart` | 7 | 10009, 108i_j, 172c_j06, 174a_j07, 177e_j | restart.wmb 7 | **implemented** |  |
| `initsetpovSyn` | 7 | extro | spherang.mdl 7 | **missing** | Extro only; currently treated as a marker. §2.14. |
| `OnGround` | 5 | 101o_j | arbre1.pcx 5 | **missing** | Ground snap and alpha 80 missing. §2.13. |
| `initStartEnnemy` | 4 | 180c_t | cliprang.mdl 4 | **missing** | Visible cliprang.mdl helper, no behaviour. §2.13. |
| `inithive` | 3 | 176g_t, 177e_j | hivegree.wmb 2, hiveblue.wmb 1 | **missing** | Rendered static. wasps.md §8. |
| `initsetanimplayer` | 3 | extro | spherang.mdl 3 | **missing** | Extro only; currently a marker. §2.14. |
| `initmainTitle` | 2 | menu_greenpepper | titjump.wmb 1, titjum2.wmb 1 | **missing** | Menu only. §2.15. |
| `initcamera4` | 1 | 177e_j | small.pcx 1 | **marker-only** | Only value is 105 = default. Nothing to port. |
| `initlionskills1` | 1 | 177e_j | small.pcx 1 | **implemented** |  |
| `initPLayerPatrol` | 1 | menu_greenpepper | suricat1.mdl 1 | **missing** | Menu/intro only. §2.15. |
| `initsetfinextro` | 1 | extro | spherang.mdl 1 | **missing** | Extro only; currently a marker. Ends the game. §2.14. |
| `PosBonusLevels` | 1 | menu_greenpepper | setcam.wmb 1 | **missing** | Menu camera marker. §2.15. |
| `PosJumpLevels` | 1 | menu_greenpepper | setcam.wmb 1 | **missing** | Menu camera marker. §2.15. |
| `PosthinkLevels` | 1 | menu_greenpepper | setcam.wmb 1 | **missing** | Menu camera marker. §2.15. |
| `PosTutorialLevels` | 1 | menu_greenpepper | setcam.wmb 1 | **missing** | Menu camera marker. §2.15. |
| `StartExtro` | 1 | extro | wasp1.mdl 1 | **missing** | Extro only. §2.14. |

### 1.3 Script actions with no shipped entity

These exist in the scripts but no level uses them. Port them only for completeness (specs below are brief or by reference):
`InitTrampoline` (§2.7), `InitBodyPlatForm` (§2.6), `InitLava` (ported anyway), `InitDeathDalle`, `InitRotationObjet`, `InitRemoveObjet`, `InitBonus5`, `InitMdlBonus`, `Initplanche1`, `InitDynamicLight` (§2.13), `InitBounceObject` (`mm1.wdl:3730-3770`, same push rule as `PlatForm_Hit` IMPACT), `InitBonusLife` (alias of `InitBonusVie`), `InitGame8`, `InitLionSkills2`, `Initprotection`, `InitInvisible`, `InitRangeMdl`, `InitSammyExtro` (dead, §2.14), `InitExtro` (called by code, not by an entity).

### 1.4 Which levels need which feature

Computed from the entity data [D]. Use this with the priority list in §3.

| Feature | Levels |
|---|---|
| Door Flag5 "gem flies into the door" (117 doors) | 29: 10003 10004 10005 10006 10007 10009 10010 10012 102N_t 104M_j 106g_t 107i_j 108i_j 109o_t 110n_t 111r_j 112e_j 113q_t 114t_t 170b_t00 171c_t04 172c_j06 173b_t05 174a_j07 175c_t 176g_t 178d_t 179f_t 180c_t |
| Locked platform → `platfor1.wmb` morph (38 platforms) | 15: 10006 10009 10012 111r_j 170b_t00 171c_t04 172c_j06 173b_t05 174a_j07 175c_t 176g_t 177e_j 178d_t 179f_t extro |
| Camera type 2 (`InitCamera1` Flag4) | 11: 10001-10007 102N_t 104M_j 106g_t 108i_j |
| `SetCamera` zones (741) | 21: 10012 103J_j 107i_j 108i_j 109o_t 111r_j 112e_j 113q_t 114t_t 170b_t00 171c_t04 173b_t05 174a_j07 175c_t 176g_t 177e_j 178d_t 179f_t 180c_t extro menu |
| `InitGame5` slope/rotation values | 34 levels set them; ice level 179f_t is the one that changes feel most |
| `InitArbre` decoration (668) | 17: 10001 10004 10005 10007 10008 101o_j 103J_j 104M_j 113q_t 172c_j06 173b_t05 174a_j07 175c_t 176g_t 178d_t extro menu |
| `InitMDL` (cactus collision, cages) | 10: 10002 10004 10007 10009 104M_j 111r_j 172c_j06 178d_t extro menu |
| `InitBoulder` gem emitters | 5: 112e_j 174a_j07 176g_t 178d_t extro |
| Teleport pads that also take lions (Flag2) | 4: 171c_t04 173b_t05 178d_t 179f_t |
| Door bells (Skill8 > 48) | 4: 170b_t00 171c_t04 173b_t05 180c_t |
| Lions and platforms in the same level (lion carry) | 17: 10006 10009 10012 107i_j 111r_j 170b_t00 171c_t04 172c_j06 173b_t05 174a_j07 175c_t 176g_t 177e_j 178d_t 179f_t 180c_t extro |
| Key gems with Flag7 (gravity, carried by platforms) | 180c_t (9), extro (35, the wagon) |
| Surprise key gems (Skill5) | 180c_t (12) |
| `InitHive` | 176g_t 177e_j |
| `InitStartEnnemy` | 180c_t |
| `OnGround` | 101o_j |
| Extro controller pieces | extro |

Not needed by shipped data: lion Flag6 crushing, door Flag3/Flag4 openers, death tiles with Flag2/Flag3, `InitLava`, `InitTrampoline`, `SetCamera` Skill8 surveillance and Flag2 lock, camera type 1, gem lifetimes on WED gems (WED only sets Skill1-8; Skill9 is runtime-only).

---

## 2. Specs

### 2.1 Shared gem machinery (`gems.wdl`)

**Globals and their real per-level values**

| Name | Default | Where set | Shipped value |
|---|---|---|---|
| `HeightGem` | 25 | `skills.wdl:697` | 25 |
| `Gravite` | -2 | `InitLvl.wdl:24`; `InitGame3` Skill5 (`mm1.wdl:1860`) | **-12** in 38 of 40 `InitGame3` entities [D] |
| `AccGravite` | 1.5 | `InitLvl.wdl:25`; `InitGame3` Skill6 (`mm1.wdl:1861`) | **3** [D] |
| `GemVerticalSpeed` / `GemVerticalFriction` | 10 / 2.5 | `InitLvl.wdl:63-64` | never overridden |
| `SpeedGemKey` | 12 | `InitLvl.wdl:65` | 12 |
| `GemSonar` | 0 | `InitLvl.wdl:112`; `InitGame5` Skill3 (`mm1.wdl:1919`) | 0 everywhere [D] |
| `NbrSonsCligno1` | 0 | `InitLvl.wdl:17`; `InitGame7` Skill2 | 0 everywhere [D] |
| `RangeScanDoor` | 500 | `mm1.wdl:165` | |

The port's `JumpParams` already reads `InitGame3` Skill5/6 for Sammy. **Gem code must use the same two values** (`Gravite`, `AccGravite`), not the -2/1.5 defaults: they drive the Flag7 fall speed, the launched-gem arcs and the boulder gems.

**Engine helpers** (see `wdl-porting.md` §2, §5.1):
- `accel(v, a, 0)` ≡ `v += a·time`. `accel(vec, src, 1)` ≡ `vec = src·time` (used as "vertical displacement this tick = BlobVSpeed").
- `MOVE me, rel, abs`: `rel` is rotated by the gem's pan (tilt 0). Gems are sprites; `my.push = -4` lets Sammy walk through them [I, `gems.wdl:715` comment].
- `sonar me, 2000` (`gems.wdl:59,65,122,793`): down-trace from the origin; `target.z` = floor z; fires the floor entity's `EVENT_SONAR` with `you` = the gem [U A5]. `LaunchNewKey` uses an explicit `trace` with `IGNORE_ME|IGNORE_PASSENTS|IGNORE_PASSABLE|ACTIVATE_SONAR` (`:1378-1382`).
- `content(pos) == CONTENT_SOLID`: point inside level geometry.

**Shared helpers**

```rust
fn gem_explose(gem) {                       // gems.wdl:364-394
    play_entsound(gem, "broken4", 2000);
    let n = average_fps * 0.3;              // :368  particle count is frame-rate dependent; ≈15 at 50 fps [U pick 10]
    if (1..=6).contains(&gem.color) { emit(n, gem.pos, particle_gem_explose(gem.color)); } // no particles for 255
}
fn gem_light(ent, color) {                  // :454-477 runs as a sub-task on `ent`
    if video_depth == 8 || average_fps < 16 || ent.light_range != 0 { return; }
    ent.light_range = 300.0;
    ent.light_rgb = match color { 1=>(250,0,0), 2=>(250,250,0), 3=>(250,100,0), 4=>(0,0,250),
                                  5=>(250,0,250), 6=>(0,250,0), 7=>(150,150,150), _=>(0,0,0) };
    while ent.light_range > 100.0 { ent.light_range -= 15.0 * time; wait_frame(); } // per FRAME (`wait 1`)
    ent.light_range = 0.0;
}
fn gem_catch(gem) {                         // :395-452 sub-task, colours 1-6 only
    emit(5, gem.pos, particle_gem_catch(gem.color));
    if average_fps < 40 { return; }
    wait_ticks(2); emit(10, ...); wait_ticks(4); emit(10, ...);
}
fn init_new_key(gem) {                      // :193-205  "becomes collectable"
    gem.skill37_39 = gem.scale;             // saved scale (aliases PlayerSpeedX..Z)
    gem.id = 7345;                          // Skill1
    gem.invisible = false; gem.facing = true; alpha(gem, 50);
    gem.enable_scan = true; gem.event = Key_Hit;
    gem.frame = random(17.0) + 1.0;
}
```

### 2.2 `InitKey` extras (`gems.wdl:40-205, 212-263`)

**Editor parameters actually present in shipped data** (307 gems) [D]:
- Skill6 colour: 2 ×82, 4 ×65, 1 ×59, 3 ×53, 5 ×21, 6 ×17, 255 ×5, **0 ×5** (the 5 colour-0 gems are surprise gems).
- Skill5 surprise: 12 gems, all in 180c_t (values 1, 2, 5, 6).
- Skill1-4, Skill7, Skill8 are 0 everywhere. **Skill3 (crush delay) and Skill4 (spin) are never set; Skill9 (lifetime) cannot be set from WED** (only Skill1-8 are stored), so lifetimes only exist on runtime gems (§2.3, §2.4).
- Flags: none ×163, **Flag6 ×100** (keep WED height), **Flag5+Flag7 ×35** (extro wagon), **Flag7 ×9** (180c_t).

```rust
// ---------- InitKey (spawn, and again on every respawn via GemWaitForNewStart) ----------
fn init_key(gem) {                                              // :40
    gem.ambient = 100;
    if gem.skill5 != 0 {                                        // surprise gem :43-49
        alpha(gem, 60);
        wait_ticks((random(8.0) as i32 + 1) as f32);           // 1..8 ticks, visible but NOT collectable
        gem.color = min(gem.color + (random(gem.skill5) as i32) + 1, 6);   // Skill6 is overwritten
        // QUIRK [R]: on every respawn this runs again on the already-raised Skill6, so the colour
        // only ever climbs towards 6 (green) with each death. Replicate.
    } else {
        alpha(gem, 50);
    }
    gem.push = -4;
    wait_ticks(1);                                              // :55
    gem.flag3 = true;                                           // level gem: respawns on reset
    if gem.flag5 {                                              // :57-61
        let floor = sonar(gem, 2000.0);
        gem.haut_ball = gem.z - floor.z + HEIGHT_GEM;           // Skill40: hover height = WED height above floor + 25
    }
    if !gem.flag6 {                                             // :63-67
        let floor = sonar(gem, 2000.0);
        gem.z = floor.z + HEIGHT_GEM;
    }
    init_new_key(gem);                                          // :68 collectable now
    gem.flag1 = true; gem.passable = true;                      // :70-71
    branch gem_wait_for_player(gem);
}
```

Note: the sprite file is never re-chosen from Skill6. A surprise gem keeps its WED sprite (gemye/gemre/...) whatever colour it rolls; so do the extro wagon gems (all Skill6 = 3, eight different sprites) [R, `gems.wdl:40-73`]. The HUD and doors use Skill6.

```rust
// ---------- GemWaitForPlayer: idle loop of every collectable gem ----------
fn gem_wait_for_player(gem) {                                   // :75-175
    gem.skill31 = 3.0;                                          // frames per tick
    while gem.flag1 && score < score_goal {                     // :77  all gems stop when the goal is reached
        if gem.skill9 > 0.0 {                                   // lifetime (runtime gems only)
            // Blink by toggling scale once per animation half-cycle. Only works for gems created at
            // scale 1.5 (runtime gems): the tests are exact float compares [R].
            if gem.frame > 9.0 && gem.scale_x == 1.5 {          // :82-90
                if nbr_sons_cligno1 < 5 { play_entsound(gem, "cligno1_son", 150); }
                gem.scale_xy = 0.75;
            }
            if gem.frame < 9.0 && gem.scale_x == 0.75 {         // :91-100
                gem.skill31 += 0.1 * time;                      // animation (and blink) speeds up
                if nbr_sons_cligno1 < 5 { play_entsound(gem, "cligno1_son", 150); }
                gem.scale_xy = 1.5;
            }
            gem.skill9 -= time;                                 // :101
            if gem.skill9 <= 0.0 {                              // :102-108
                nbr_sons_cligno1 -= 1;
                gem_explose(gem);
                gem.flag1 = false;                              // loop ends after this tick
            }
        }
        let v = gem.frame + gem.skill31 * time;                 // :110-112
        gem.frame = if v >= 18.0 { 1.0 } else { v };
        if gem.skill4 != 0.0 { gem.pan += gem.skill4 * time; }  // spin; invisible for a facing sprite [I]
        if gem.flag7 {                                          // :118-151  gravity + platform carry
            gem.skill43 = 0.0;                                  // "no carry this tick"
            let floor = sonar(gem, 2000.0);                     // a moving platform's EVENT_SONAR may set skill43/PlayerForce (§2.6)
            gem.haut_sol_ball = floor.z;
            if gem.skill43 != 0.0 {
                let f = gem.player_force;                       // Skill22-24 = the platform's motion this tick
                move_abs(gem, f);                               // :129 collides [U]
                gem.player_force = accel_vec(f, ZERO, 0.2);     // :131 decays (friction 0.2) — overwritten next carry tick anyway
            }
            let rest = if gem.flag5 { gem.haut_ball } else { HEIGHT_GEM };
            if gem.z > gem.haut_sol_ball + rest { gem.z += GRAVITE * time; }   // falls at 12/tick (shipped Gravite -12)
            else { gem.z = gem.haut_sol_ball + rest; }
        }
        wait_ticks(1);
    }
    gem.invisible = true; gem.passable = true;                  // :155-174
    if gem.flag3 {                                              // level or persistent gem
        gem.enable_scan = true; gem.event = GemWaitForNewStart; // dormant until the next reset scan
    } else {                                                    // runtime gem
        gem.event = None;
        while gem.light_range != 0.0 { wait_ticks(2); }
        wait_ticks(16);
        remove(gem);
    }
}

// ---------- reset scan (id 1000, radius 50000, from RestartSammy player.wdl:886-891) ----------
fn gem_wait_for_new_start(gem) {                                // :178-191 (event of dormant Flag3 gems)
    if scan_identite != 1000 { return; }
    if key_player1 == 255 { remove(gem); return; }              // master key held: level gems are gone for good
    gem.scale = gem.skill37_39;
    branch init_key(gem);                                       // full re-init in place (surprise re-roll, snaps)
}
fn key_hit(gem) {                                               // :212-263 (event of collectable gems)
    if scan_identite == 1000 && (!gem.flag3 || key_player1 == 255) {   // :213-223
        gem.passable = true; gem.invisible = true; gem.enable_scan = false;
        gem.flag1 = false;                                      // GemWaitForPlayer ends; runtime gems get removed
        return;                                                 // (a Flag3 gem with key 255 goes dormant, removed at the next reset)
    }
    if scan_identite == 100 {                                   // crushed by a Flag6 lion (enemy-ai.md §6.6; unused in data)
        gem_explose(gem); gem.event = None;
        while gem.scale_x > 0.05 { gem.scale_xy *= 0.5; wait_ticks(1); }
        gem.invisible = true;
        if gem.skill3 == 0.0 { gem.flag1 = false; return; }     // all shipped gems: gone (dormant if Flag3)
        gem.skill30 = gem.skill3;
        while gem.skill30 > 0.0 { gem.skill30 -= time; wait_ticks(1); }
        gem.invisible = false;
        while gem.scale_x < gem.skill37 { gem.scale_xy += 0.1 * time; wait_ticks(1); }
        gem.scale = gem.skill37_39; gem.event = Key_Hit;
        return;
    }
    if scan_identite == 1 { pick_up(gem, you /* Sammy */); }    // §2.3
}
```

Port deltas for `InitKey` (`actions/pickups.rs`):
1. Surprise colour (Skill5) and its respawn re-roll/climb.
2. Flag7: per-tick gravity at `Gravite` and platform carry (needs §2.6). Flag5 hover height. Without these the extro wagon gems stay at the start and 180c_t's Flag7 gems float.
3. Respawn goes through `init_key` again (surprise re-roll; Flag7 gems re-snap at the spot where they were picked up, which is where `LevelKey.home` already puts them).
4. Lifetime blink/explode belongs to runtime gems (§2.3/§2.4); the port has no Skill9.
5. Crush (scan 100): unused in data; skip.

### 2.3 Picking up, mixing and the thrown-back key (`Key_Hit` GemOk, `EjectOldKey`, `LaunchKey`, `LaunchYouKey`)

```rust
fn pick_up(gem, sammy) {                                        // gems.wdl:265-350
    next_gem_wait += 1;                                         // :267 (harmless here)
    if sammy.player_old_key != 0 { return; }                    // :269  a key is "in flight" (see below); gem stays collectable
    gem.event = None;
    gem.entity1 = sammy;
    gem_light(gem, gem.color); gem_catch(gem);                  // sub-tasks
    play_entsound(gem, "ColoredGem_Son", 2000);
    if sammy.player_key == 0 {
        sammy.player_key = gem.color; sammy.player_old_key = 0;
        scale_down_gem(gem);                                    // sub-task; caller continues
    } else {
        sammy.player_old_key = sammy.player_key;
        sammy.player_key = gem.color;
        scale_down_gem(gem);
        match (sammy.player_key, sammy.player_old_key) {        // :297-326 mixable primaries
            (1,2)|(2,1) => { sammy.player_key = 3; anim_mix_keys(gem, 1, 2); sammy.player_old_key = 0; }
            (1,4)|(4,1) => { sammy.player_key = 5; anim_mix_keys(gem, 1, 4); sammy.player_old_key = 0; }
            (4,2)|(2,4) => { sammy.player_key = 6; anim_mix_keys(gem, 2, 4); sammy.player_old_key = 0; }
            _ => eject_old_key(gem, sammy),                     // :328-334 (player_key != 0 always here)
        }
    }
    key_player1 = sammy.player_key;                             // :336-347 HUD
    gem_current_life = gem.skill9;                              // inherited by the NEXT thrown-back key [U ordering, below]
    gem_current_delay = gem.skill3;
    gem_current_state = gem.flag3;                              // unused
}
fn scale_down_gem(gem) {                                        // :351-363
    gem.enable_scan = false; gem.event = None;
    while gem.scale_x > 0.05 { gem.scale_xy *= 0.8; wait_ticks(1); }
    gem.flag1 = false; gem.invisible = true;                    // GemWaitForPlayer ends: Flag3 → dormant, else removed
    wait_ticks(2);
}
fn eject_old_key(gem, sammy) {                                  // :649-690
    let pos = sammy.pos + vec3(0, 0, 50);
    create(sprite_for(sammy.player_old_key), pos, LaunchKey);   // colours 1-6 and 255 (gemmu)
}
fn launch_key(new) { new.entity1 = creator_gem.entity1 /* Sammy */; you = Sammy; branch launch_you_key(new); } // :693-701
```

`AnimMixKeys`/`AnimGemMix1` (`:494-613`) are summarised in `gameplay.md` §7 (radius 0→80→1, 70°/tick, +6/tick radius, Z +5/tick around `pov_Syn`, then 70 `Particle_Explose<colour>` and `EndGemMixing_Son`, removed 16 ticks later). They are visual only.

**`PlayerOldKey` semantics (port bug).** Pickup is refused only while `Sammy.PlayerOldKey != 0`. It is non-zero:
- between `pick_up` and the moment the thrown-back key's `LaunchYouKey` starts (it zeroes it at `:720`, before its first wait) — effectively **0-1 tick**;
- for the whole flight of a gem towards a Flag5 door (999, §2.5);
- permanently after the score goal (999, `mm1.wdl:1450`).

The port sets `key_in_flight` until the thrown-back key's 5th bounce, which blocks all other pickups for about a second. It should only block for the door flight (and the goal case, already handled by `goal_reached`). The thrown-back key itself is not collectable until §below says so.

```rust
// ---------- LaunchYouKey: the thrown-back old key ----------
fn launch_you_key(gem, sammy) {                                 // :702-952
    gem.entity1 = sammy; gem.invisible = true;
    gem.skill25_27 = gem.pos;                                   // last good position (restored if stuck in a wall)
    gem.push = -4;
    gem.skill28 = gem.z + 32.0;                                 // ceiling: creation z (= Sammy.z+50) + 32
    gem.color = sammy.player_old_key;                           // Skill6
    gem.skill3 = gem_current_delay;
    gem.skill9 = gem_current_life;                              // [U] if `create` runs the action immediately (inside pick_up),
                                                                // this is the lifetime of the PREVIOUS pickup, i.e. of this key; if
                                                                // deferred, it is the new gem's. Recommend: lifetime of this key.
    sammy.player_old_key = 0;                                   // :720 pickup unblocked
    alpha(gem, 50); gem.ambient = 100; gem.facing = true;
    gem.event = TestGemNear; gem.enable_scan = true; gem.enable_stuck = true;
    gem.skill29 = 0.0;
    gem.scale = 1.5;  gem.fat = false; gem.narrow = false;      // point hull
    gem.id = 7345;
    gem.skill20 = 0;                                            // bounce counter
    gem.blob_vspeed_org = GEM_VERTICAL_SPEED;                   // 10
    gem.blob_vspeed = GEM_VERTICAL_SPEED;
    gem.blob_fric_vspeed = GEM_VERTICAL_FRICTION;               // 2.5
    gem.pan = sammy.pan;                                        // thrown FORWARD (port throws it backwards at 12)
    gem.speed_balle = sammy.speed_balle * 2.0;                  // Sammy's Skill2 (WED walk speed, 25 typical) ×2 = 50, NOT his current speed

    // Wait while another gem is being launched within 78 quants (scan id 101; only launching/flying gems answer).
    loop {                                                      // :746-768
        next_gem_wait = 0;
        scan_sphere(gem.pos, radius: pov_syn.blob_zone_scan * 0.75, id: 101);   // 104·0.75 = 78
        if next_gem_wait != 0 && gem.skill29 < 16.0 {
            move_rel(gem, vec3(0.1, 0, 0)); wait_ticks(2); gem.skill29 += time; continue;
        }
        break;
    }
    if gem.skill29 >= 16.0 {                                    // gave up waiting: scatter
        gem.pan = random(360.0);
        gem.blob_vspeed_org = GEM_VERTICAL_SPEED * 2.0;         // 20 (only affects bounces/particles)
        gem.blob_vspeed = GEM_VERTICAL_SPEED;
        gem.blob_fric_vspeed = GEM_VERTICAL_FRICTION;
        gem.speed_balle = 2.0;
    }
    gem.passable = false; gem.invisible = false; gem.enable_stuck = false;
    gem.event = Gem_Hit; gem.enable_block = true; gem.enable_entity = true;
    gem.skill29 = 0.0;
    gem.flag1 = true;
    while gem.flag1 {                                           // :784-934, once per FRAME (`wait 1`)
        gem.push = -4;
        gem.frame = wrap17(gem.frame + 3.0 * mintime);
        let mut dz = 0.0;
        let floor = sonar(gem, 2000.0); gem.blob_haut_sol = floor.z;
        if gem.blob_vspeed == 0.0 && gem.z - floor.z > HEIGHT_GEM + 1.0 {        // walked off a ledge
            gem.blob_vspeed = GRAVITE; gem.blob_fric_vspeed = ACC_GRAVITE;       // -12 / 3
        }
        if gem.z > gem.skill28 {                                // never above launch height + 32
            gem.blob_vspeed = GRAVITE; gem.blob_fric_vspeed = ACC_GRAVITE;
        }
        if gem.blob_vspeed != 0.0 {
            gem.blob_vspeed -= gem.blob_fric_vspeed * time;     // accel V, -fric, 0
            let mut vz = gem.blob_vspeed;
            if gem.z < floor.z + HEIGHT_GEM {                   // touched down = a bounce
                gem.skill20 += 1;
                if gem.skill20 == 5 { init_new_key(gem); }      // collectable from the 5th bounce on
                if gem.speed_balle > 2.0 {
                    if gem.blob_vspeed_org > 7.0 {              // bounce sounds while bounces are still high
                        play_entsound(gem, ["ding6","ding5","ding3","ding7"][min(gem.skill20,4)-1], 1000);
                    }
                    gem.speed_balle -= 5.0;
                    gem.blob_vspeed_org *= 0.95;
                    gem.blob_vspeed = gem.blob_vspeed_org - gem.blob_fric_vspeed * time;
                    vz = gem.blob_vspeed;
                    dz = vz * time;
                } else {
                    dz = (floor.z + HEIGHT_GEM) - gem.z;        // settle
                    gem.blob_vspeed = 0.0;
                }
            } else {
                dz = vz * time;
            }
        }
        if ticks1 % 16 == 0 { gem.skill25_27 = gem.pos; }
        move_gem(gem, rel: vec3(mintime * gem.speed_balle, 0, 0), abs: vec3(0, 0, dz));   // fires Gem_Hit on walls/entities
        if my_speed.z == 0.0 {                                  // vertical part blocked (or zero)
            if gem.z > floor.z + HEIGHT_GEM {                   // stuck in the air
                gem.skill29 += 1.0;
                if gem.skill29 > 24.0 { gem.flag1 = false; }
                play_entsound(gem, "ding3", 500);
                gem.speed_balle = 30.0;
                gem.blob_vspeed_org = GEM_VERTICAL_SPEED; gem.blob_vspeed = GEM_VERTICAL_SPEED;
                gem.pan = random(360.0); gem.z += 8.0;
            } else {
                gem.z = floor.z + HEIGHT_GEM; gem.flag1 = false;   // resting: done
            }
        }
        gem.speed_balle -= 0.5 * mintime;
        if gem.speed_balle < 0.1 {
            gem.speed_balle = 0.1;
            if gem.blob_vspeed == 0.0 { gem.flag1 = false; }
        }
        if gem.blob_vspeed_org > 10.0 { particle_gem_keys(gem); }   // only after the "scatter" case (20)
        if content(gem.pos) == SOLID {                          // inside a wall: go back to last good position
            gem.flag1 = false; gem.pos = gem.skill25_27;
            wait_ticks(4); break;
        }
        wait_frame();
    }
    gem.flag1 = true;
    if gem.skill20 < 5 { init_new_key(gem); }                   // collectable once it rests, whatever the bounce count
    gem.passable = true;
    if gem_sonar != 0 { gem.flag7 = true; }                     // never in shipped data
    branch gem_wait_for_player(gem);                            // Flag3 = 0: removed after pickup, removed by the reset scan
}
fn test_gem_near(gem) {                                         // :954-964 (event while waiting to launch)
    if event_type == EVENT_STUCK { gem.z += 32.0 + random(16.0); next_gem_wait += 1; return; }
    if scan_identite == 101 { next_gem_wait += 1; }
}
fn gem_hit(gem) {                                               // :965-981 (event while flying)
    if event_type == EVENT_SCAN { next_gem_wait += 1; return; } // tells a launching gem to wait
    gem.pan = to_angle(bounce).pan;                             // reflect off walls/entities (Sammy included)
    gem.speed_balle = max(gem.speed_balle - 8.0, 0.1);
}
```

Notes:
- Once the 5th bounce calls `init_new_key`, the event becomes `Key_Hit`: walls no longer reflect the gem (MOVE glides), and Sammy can catch it in flight.
- Typical result: thrown forward at 50 qu/tick, losing 0.5/tick and 5 per bounce, 3 more units of `BlobVSpeed` per tick lost to `AccGravite` only after hitting the +32 ceiling. It settles after ~6 bounces, roughly 150-250 quants ahead [I]. The port's version (backwards, 12 qu/tick, reflect ×0.6, gravity 4) is a different trajectory.
- If a lifetime was inherited (`Skill9 > 0`), the scale is 1.5, so the blink logic of §2.2 works and the key explodes when the time runs out.

### 2.4 Boulders and gems launched by objects (`InitBoulder`, `BoulderGem_Hit`, `EmitGems`, `EjectNewKey`, `LaunchNewKey`)

**Editor parameters** (`gems.wdl:1550-1566`):

| Field | Meaning |
|---|---|
| Skill1 | gem fate → `PlayerOldKey`: 0 decorative (explodes when it stops, never collectable); 1-999 collectable with that **lifetime in ticks**; 1000 stays; 1001 stays and respawns (Flag3) |
| Skill2 | `BounceForce` applied to non-matching impacters |
| Skill4 | launch vertical speed (`PlayerSpeedZ`) |
| Skill5 | `IdPlayer` of the entity whose IMPACT emits a gem (9999 = Sammy's wasps) |
| Skill6 | colour; 999 = random 1-6 per gem |
| Skill7 | auto mode: horizontal speed base; impact mode: factor on the impacter's speed |
| Skill8 | auto-emission period in ticks (0 = none) |
| Flag2 | no emission on impact (no IMPACT event at all) |
| Flag3/4/5/6 | add 20/45/90/180° to the random spread `Skill11` |

**Shipped boulders** (12) [D]:

| Level | File | Skills `[1..8]` | Flags | Behaviour |
|---|---|---|---|---|
| 112e_j | boulder3.wmb | `[60,0,0,20,0,999,10,48]` | F2 F3 F4 F5 + PASSABLE | auto every 48: random colour, spread 155°, lifetime 60 |
| 174a_j07 | boulder4.wmb, pan 270 | `[4,0,0,30,0,1,10,64]` | F2 F3 F4 F5 + PASSABLE | auto every 64: **red**, lifetime **4** ("red-gem fall": catch them while they bounce, they burst 4 ticks after coming to rest) |
| 176g_t | boulder5.wmb | `[96,0,0,30,9999,1,0.15,0]` | F3 | shoot it with a wasp → red gem, lifetime 96 |
| 178d_t | boulder5.wmb | `[480,0,0,30,9999,1,0.2,0]` | F3 | shoot it → red gem, lifetime 480 |
| extro ×2 | boulder5.wmb | `[96,0,0,30,9999,1,0.15,0]` | F3 | shoot it → red gem |
| extro ×6 | small.pcx (invisible emitters, PASSABLE) | `[16|64|80,0,0,20|30,0,999,20|25|30,48|64]` | F2 F3 F4 + PASSABLE | rain random-colour gems over the wagon track |

```rust
fn init_boulder(b) {                                            // gems.wdl:1568-1618
    b.albedo = 20; b.ambient = 70;
    if !b.flag2 { b.enable_impact = true; b.event = BoulderGem_Hit; }
    b.skill11 = 20.0*f3 + 45.0*f4 + 90.0*f5 + 180.0*f6;        // spread (added to WED Skill11, which is 0)
    b.skill12 = b.skill11 * 0.5;
    b.bounce_force = b.skill2;                                  // Skill44
    b.player_old_key = b.skill1;                                // Skill46: fate
    b.player_speed_z = b.skill4;                                // Skill39
    if b.skill8 == 0.0 { return; }
    wait_ticks(48);
    loop {                                                      // while Skill8 > 0
        b.skill10 = b.skill8;
        while b.skill10 > 0.0 { b.skill10 -= mintime; wait_ticks(1); }
        b.player_dest_pan = ang(b.pan + random(b.skill11) - b.skill12);    // ±spread/2 around the boulder's pan
        b.player_speed_x = b.skill7 + random(3.0);
        b.player_speed_y = 0.0;
        b.player_speed_z = b.skill4 + random(4.0);
        emit_gems(b, spawn: b.pos + vec3(0, 0, 150));
    }
}
fn boulder_gem_hit(b) {                                         // :1620-1640  EVENT_IMPACT: `you` moved into the boulder
    if you.id_player != b.skill5 {                              // anything else just gets pushed back
        you.bounce_force1.xy = -you.ennemy_speed.xy * b.bounce_force;   // shipped BounceForce 0 → zeroes the impacter's knockback
        return;
    }
    b.player_dest_pan = ang(you.pan - 180.0 + random(b.skill11) - b.skill12);   // back towards the shooter
    b.player_speed_x = you.player_speed_x * b.skill7;           // wasp: PlayerSpeedX = its last My_Speed.x (bullets.wdl:379)
    b.player_speed_y = you.player_speed_y * b.skill7;
    // player_speed_z keeps Skill4 (impact mode never randomises it)
    emit_gems(b, spawn: you.pos);
}
fn emit_gems(b, spawn) {                                        // :1641-1657
    b.player_key = if b.skill6 == 999.0 { random(6.0) as i32 + 1 } else { b.skill6 };
    gem_light(b, b.player_key);
    b.player_old_key = b.skill1;
    eject_new_key(b, spawn);                                    // :1272-1313 → create(sprite_for(b.player_key), spawn, LaunchNewKey)
}                                                               // (no sprite for 255: EjectNewKey only handles 1-6)
```

`EjectNewKey` is also used by `ResetSammy` (`player.wdl:1068-1087`) when Sammy dies holding a key below the goal: spawn at `Sammy.pos + (0,0,30)`, `PlayerOldKey = 0` (**decorative**: explodes when it stops, never collectable), `PlayerSpeedZ = 40`, colour = his key, then his key is cleared. The level gem itself comes back through the reset scan. The port's death eject creates a collectable key instead.

```rust
fn launch_new_key(gem, src /* boulder or Sammy */) {            // gems.wdl:1317-1548
    gem.entity1 = src; gem.passable = false;
    gem.player_key = src.player_key; gem.color = src.player_key;
    gem.player_old_key = src.player_old_key;                    // fate
    gem.push = -4;
    gem.scale = 1.5;
    if gem.player_old_key > 0 && gem.player_old_key < 1000 { gem.flag7 = true; alpha(gem, 30); }  // lifetime gems use gravity
    else { alpha(gem, 50); }
    gem.ambient = 100; gem.facing = true;
    gem.event = Gem_Hit; gem.enable_block = true; gem.enable_entity = true;
    gem.skill20 = 0;
    gem.blob_vspeed_org = src.player_speed_z; gem.blob_vspeed = src.player_speed_z;
    gem.blob_fric_vspeed = ACC_GRAVITE;                         // 3 in shipped levels
    gem.pan = src.player_dest_pan;
    gem.speed_balle = max(src.player_speed_x.abs() + src.player_speed_y.abs() + 5.0, 25.0);   // at least 25
    gem.flag1 = true;
    gem.skill9 = gem.player_old_key;                            // lifetime
    if gem.player_old_key == 1000 { gem.skill9 = 0; gem.flag7 = true; }
    if gem.player_old_key == 1001 { gem.skill9 = 0; gem.flag3 = true; gem.flag7 = true; }
    gem.id = 7345;
    gem.skill25_27 = gem.pos;
    while gem.flag1 {                                           // once per FRAME
        gem.push = -4;
        gem.frame = wrap17(gem.frame + 3.0 * mintime);
        let mut dz = 0.0;
        let floor = trace_down(gem, 2000.0, IGNORE_ME|IGNORE_PASSENTS|IGNORE_PASSABLE|ACTIVATE_SONAR);
        gem.blob_haut_sol = floor.z;
        if gem.blob_vspeed == 0.0 && gem.z - floor.z > HEIGHT_GEM + 1.0 {
            gem.blob_vspeed = GRAVITE; gem.blob_fric_vspeed = ACC_GRAVITE;
        }
        if gem.blob_vspeed != 0.0 {
            gem.blob_vspeed -= gem.blob_fric_vspeed * time;
            let vz = gem.blob_vspeed;
            if gem.z < floor.z + HEIGHT_GEM {
                gem.skill20 += 1;
                if gem.skill20 == 5 && gem.player_old_key > 0 {
                    if gem.skill9 < 1000.0 { nbr_sons_cligno1 += 1; }   // QUIRK: also for 1000/1001 (Skill9 = 0); never undone on pickup
                    init_new_key(gem);                          // collectable
                }
                if gem.speed_balle > 0.5 {                      // ALWAYS true (speed_balle ≥ 1 below): it never "settles" here
                    if gem.blob_vspeed_org > 5.0 { play bounce ding6/5/3/7 as in §2.3 }
                    gem.blob_vspeed_org *= 0.9;
                    gem.blob_vspeed = gem.blob_vspeed_org - gem.blob_fric_vspeed * time;
                    dz = gem.blob_vspeed * time;
                } else {
                    dz = (floor.z + HEIGHT_GEM) - gem.z; gem.blob_vspeed = 0.0;
                }
            } else {
                dz = vz * time;
            }
        }
        move_gem(gem, rel: vec3(mintime * gem.speed_balle, 0, 0), abs: vec3(0, 0, dz));
        gem.speed_balle -= 0.05 * mintime;
        if my_speed.x.abs() + my_speed.y.abs() < 1.0 {          // blocked horizontally
            gem.blob_vspeed_org -= 0.5 * mintime;
            if gem.blob_vspeed_org.abs() < ACC_GRAVITE.abs() { gem.flag1 = false; }
        } else {
            gem.blob_vspeed_org -= 0.02 * mintime;
        }
        if gem.speed_balle < 1.0 {
            gem.speed_balle = 1.0;
            if gem.blob_vspeed_org.abs() < ACC_GRAVITE.abs() { gem.flag1 = false; }
        }
        if ticks1 % 16 == 0 { gem.skill25_27 = gem.pos; }
        if content(gem.pos) == SOLID { gem.flag1 = false; gem.pos = gem.skill25_27; }
        if gem.blob_vspeed_org > 2.0 { particle_gem_keys(gem); }
        wait_frame();
    }
    gem.flag1 = true;
    if gem.skill20 < 5 && gem.player_old_key > 0 { init_new_key(gem); }
    gem.passable = true;
    if gem.player_old_key == 0 {                                // decorative (Sammy's death gem)
        gem.flag1 = false; gem.event = None;
        while gem.scale_x > 0.05 { gem.scale_xy *= 0.9; wait_ticks(1); }
        gem_explose(gem);
        gem.passable = true; gem.invisible = true; wait_ticks(16); remove(gem);
        return;
    }
    if !gem.invisible { gem_wait_for_player(gem); }             // (invisible = it was already caught in flight)
}
```

Notes:
- **When does it stop?** Only when blocked against a wall with `|BlobVSpeedOrg| < AccGravite` (3), or when `SpeedBalle` has decayed to 1 (≈ 480 ticks at -0.05/tick, faster after `Gem_Hit`'s -8 per wall hit) with `|BlobVSpeedOrg| < 3`. `BlobVSpeedOrg` loses ×0.9 per bounce and 0.02/tick (0.5/tick while blocked). Boulder gems therefore keep hopping and sliding for a long time [R]; that is the intended "gem rain".
- It is collectable from the 5th bounce (or when it stops). Lifetime gems (1-999) have Flag7, so after stopping they keep falling with gravity and ride platforms; their `Skill9` only counts down in `GemWaitForPlayer`, i.e. **after they stop** (or after being thrown back by Sammy).
- When collected, a lifetime gem's remaining `Skill9` is inherited by the key Sammy later throws back (§2.3).

### 2.5 Doors: full open/close machine, gem flight, bells (`mm1.wdl:2855-3283`, `gems.wdl:987-1233`)

**Shipped door classes** [D] (277 doors):

| Prefab | Flags | Skill5 | Count | Behaviour |
|---|---|---|---|---|
| `dfix<colour>` | F5 F7 | > 0 | 101 | permanent once the gem reaches it; otherwise closes again |
| `dfix<colour>` | F5 F7 | 0 | 7 | permanent; the gem can never be called back |
| `dfixall` | F5 F6 | 0 | 6 (113q_t) | opens without a key; validated by walking under it |
| `done<colour>`, `doormage` (108i_j) | F7 | 0 | 56 | "burn": opens once, stays up, ambient -20 |
| `door<colour>` | F7 | > 0 | 64 | timed; 11 with Skill8 = 172 (bells), 3 with 32, 3 with 40, 1 with 600 (bells) |
| `doorexit` | F7 (41) / none (2) | 0 | 43 | exit door (Skill6 = 7): burn door |

All have Skill7 = 25 (Sammy's door scan) and Skill4 = 40 (50 on two). Skill2 (rise) 150-450, raised to 300 when ≤ 250. Skill3 = 0 → `RangeScanDoor - 150` = 350.

**Port status** (`actions/door.rs`): open rule, rise, hold, lower, burn doors and the "wrong key" flash are there. Flag5 doors are treated as "open forever and take the key" the moment they open. Missing: everything below that involves `Skill14`/`Skill15` (gem flight, recall, validation, `doordark`), bells, `DoorScanEntities` radius and lions, and the exit-door double flash.

```rust
// Fields: skill9 = state (0 down, 1 rising, 3 up, 4 up+Sammy still near, 5 must lower),
// skill10 = z0, skill11 = hold counter, skill13 = top z, skill14 = "gem created/in flight",
// skill15 = "gem has validated the door", skill16 = bell timer, skill17 = entities under door.
fn init_door(d) {                                               // mm1.wdl:2886-3006
    d.skill1 = 90273;                                           // id: camera may see through doors (DoorInvisible)
    if d.skill2 <= 250.0 { d.skill2 = 300.0; }
    wait_ticks(2);
    if door_passable == 1 { d.push = -2; }
    if d.skill8 == 0.0 { d.skill8 = 4.0; }
    if d.skill3 == 0.0 { d.skill3 = RANGE_SCAN_DOOR - 150.0; }  // 350
    d.event = movePortail; d.enable_scan = true;
    d.skill10 = d.z; d.skill13 = d.z + d.skill2; d.flag1 = true; d.skill9 = 0;
    while d.flag1 {                                             // :2920-2982 "closer" loop
        if d.skill9 == 5 {
            if d.skill5 != 0.0 && d.skill14 == 0 {              // wait until nobody is under the door
                d.enable_scan = false; d.event = DoorScanEntities; d.enable_detect = true;
                d.skill17 = 0; scan_sphere(d.pos, radius: d.skill2 + 100.0, id: 102); wait_ticks(2);
                while d.skill17 != 0 {
                    d.skill17 = 0; scan_sphere(d.pos, radius: 250.0, id: 102); wait_ticks(2);
                }
                d.event = movePortail; d.enable_detect = false; wait_ticks(1);
            }
            if d.skill5 > 0.0 {
                play_entsound(d, "DoorClose_son", 1000);
                while d.z > d.skill10 + 1.0 { d.z -= d.skill5 * time; wait_ticks(1); }
            }
            if d.skill5 < 0.0 { while d.z < d.skill10 + 1.0 { d.z -= d.skill5 * time; wait_ticks(1); } }  // unused
            if d.skill5 != 0.0 { d.z = d.skill10; d.skill14 = 0; }
            d.enable_scan = true; d.skill9 = 0;
        }
        wait_ticks(1);
    }
    if d.flag5 && d.skill15 == 1 {                              // validated permanent door
        d.z = d.skill13;
        morph(d, "doordark.wmb");                               // new model AND collision hull [U same shape]
        d.event = None; d.enable_scan = false; d.flag1 = false;
        return;
    }
    if !d.flag5 && d.skill5 == 0.0 {                            // burn door
        d.z = d.skill13; d.ambient = -20.0; d.event = None; d.enable_scan = false;
    }
}
fn door_scan_entities(d) {                                      // :3007-3014
    if event_type != EVENT_DETECT { return; }
    if you.id == 3333 || you.id == 8888 || you.id == 1111 { d.skill17 += 1; }   // Sammy AND lions block closing
}
fn move_portail(d) {                                            // :3015-3112, event: Sammy's per-tick scan id 25, radius 500
    if event_type == EVENT_SCAN && scan_identite != d.skill7 { return; }
    if !d.flag2 && you.id == 3333 {
        let key = you.player_key;
        let ok = key == 255 || d.flag6
            || if d.flag7 { key == d.skill6 }
               else { key & d.skill6 != 0 && (key == d.skill6 || matches!(key, 1|2|4)) };
        if ok { branch ouvre_portail(d); }
        // bad key or no key:
        if d.skill14 == 1 { ouvre_portail(d); }                 // :3060 keeps the distance logic running during a gem flight
        if d.skill6 == 7 && score < score_goal {                // exit door
            warn_score_goal();                                  // blinks the score panel
            if key == 0 { d.enable_scan = false; d.ambient = -10; wait_ticks(3); d.ambient = 100; d.enable_scan = true; }
        }
        if key != 0 && key != 255 {                             // wrong key: flash (exit door with a key: flashes twice, 6 ticks)
            d.enable_scan = false; d.ambient = -10; wait_ticks(3); d.ambient = 100; d.enable_scan = true;
        }
        return;
    }
    if d.flag3 && (you.id == 8888 || you.id == 1111) { branch ouvre_portail(d); }
    if d.flag4 && you.id == 9999 { branch ouvre_portail(d); }
    // Flag5 → id 78, Flag6 → id 77: legacy balls, unused
}
fn ouvre_portail(d) {                                           // :3114-3260
    let dist = (d.pos.xy - you.pos.xy).length();                // horizontal only
    if dist > d.skill3 - 4.0 {                                  // too far (346..500)
        if d.skill5 != 0.0 { d.skill14 = 0; }                   // RECALL the flying gem
        return;
    }
    if dist < 128.0 && d.skill14 == 1 { d.skill15 = 1; return; }   // Sammy walks under: speeds the gem up (validation)
    if d.skill9 == 3 || d.skill9 == 4 { d.skill9 = 4; return; }    // already up: "Sammy still here"
    if d.flag5 && d.skill14 == 0 {                              // first opening of a permanent door
        d.skill14 = 1; d.player_key = you.player_key; d.entity1 = you; d.skill15 = 0;
        create_gem_key_anim(d);                                 // gems.wdl:987, spawns at Sammy.pos + (0,0,40)
    }
    if d.skill9 == 0 || d.skill9 == 5 {                         // down (or about to lower): rise
        d.skill9 = 1; d.enable_scan = false;
        play_entsound(d, "DoorOpen_son", 1000);
        while d.z < d.skill13 { d.z += d.skill4 * time; wait_ticks(1); }   // (negative Skill4 lowers instead; unused)
        d.z = d.skill13; d.skill9 = 3; d.skill11 = d.skill8;
        if d.skill5 == 0.0 && !d.flag5 { d.flag1 = false; return; }        // burn door: closer loop ends
        d.enable_scan = true; wait_ticks(1);
    }
    loop {                                                      // :3190 hold
        if d.flag5 && d.skill15 == 1 { d.flag1 = false; return; }          // validated → closer loop ends → doordark
        d.skill16 = 0.0;
        while d.skill11 > 0.0 {
            d.skill11 -= time;
            if d.skill8 > 48.0 {                                // bells
                let ringing = d.skill11 < 48.0 && d.skill11 > 4.0;
                if ringing || d.skill11 > 48.0 {
                    if d.skill16 <= 0.0 {
                        door_light(d, V /* QUIRK: the global V, not the door colour; see below */);
                        if ringing { play_entsound(d, "Bell1", 1500); }
                        d.ambient = if d.ambient == 100.0 { 0.0 } else { 100.0 };   // blink every 12 ticks for the WHOLE hold
                        d.skill16 = 12.0;
                    }
                    d.skill16 -= time;
                }
            }
            wait_ticks(1);
        }
        if d.skill8 > 4.0 && d.skill9 != 4 { play_entsound(d, "Bell2", 2000); }   // only doors with a WED Skill8 > 4
        if d.skill9 == 4 || d.skill14 == 1 { d.skill11 = 4.0; d.skill9 = 3; continue; }   // Sammy still near / gem flying
        d.skill9 = 5; return;                                   // → closer loop lowers it
    }
}
fn door_light(d, v) {                                           // :3261-3283, like gem_light but range 600, -50/frame
    if video_depth == 8 || average_fps < 16 || d.light_range != 0 { return; }
    d.light_range = 600.0; d.light_rgb = color_table(v);        // v outside 1..6 → black (no visible light)
    while d.light_range > 100.0 { d.light_range -= 50.0 * time; wait_frame(); }
    d.light_range = 0.0;
}
```

Bell/blink notes:
- For a door with Skill8 = 172 the hold lasts 172 ticks + 4 per "still near" refresh. The ambient toggles 100↔0 every 12 ticks from the start of the hold; `Bell1` rings with each toggle during the last 48 ticks (`4 < Skill11 < 48`); `Bell2` at the end. The ambient is **not** restored afterwards, so the door may stay dark (ambient 0) [R].
- `DoorLight` is passed `V1 = Skill1` but reads `V` (`:3206-3207` vs `:3267-3272`): the colour is whatever the global `V` holds at that moment [R bug]. Port: use the door's colour (`Skill6`) and note the deviation, or skip the light.
- `Bell2` also rings for Skill8 32/40 doors (no blink).
- The hold of a normal door is 4 ticks, refreshed by +4 while Sammy (scan every tick) stays within `Skill3 - 4` horizontally.

**The gem that flies into a permanent door** (`CreateGemKeyAnim :987-1038`, `AnimGemKey :1043-1233`):

```rust
fn create_gem_key_anim(door) {
    let sammy = door.entity1;
    let sprite = sprite_for(sammy.player_key);                  // 1-6 or 255 (gemmu). Key 0 (Flag6 door, no key): NO gem is created
    create(sprite, sammy.pos + vec3(0, 0, 40), AnimGemKey);     // `you` = door
}
fn anim_gem_key(gem, door) {
    let sammy = door.entity1;
    sammy.player_old_key = 999;                                 // blocks pickups during the flight
    gem.color = sammy.player_key; gem.entity1 = sammy; gem.entity2 = door;
    if score < score_goal { sammy.player_key = 0; hud_hide_key(); }   // key leaves Sammy now (kept if master key)
    alpha(gem, 50); gem.passable = true; gem.facing = true; gem.ambient = 100; gem.scale = 1.0;
    let target = |d| d.pos + vec3(0, 0, 100);                   // door's CURRENT position: it is rising meanwhile
    let mut dist = (target(door) - gem.pos).length();
    gem.skill20 = 80.0;                                         // phase 1: up to 80 ticks at 12 qu/tick
    while dist > 20.0 && door.skill14 != 0 && gem.skill20 > 0.0 && door.skill15 == 0 {
        let a = to_angle(target(door) - gem.pos); gem.pan = a.pan; gem.tilt = a.tilt; dist = a.len;
        move_rel(gem, vec3(SPEED_GEM_KEY * mintime, 0, 0));     // 12/tick, along pan+tilt [U collides]
        particle_gem_keys(gem);
        gem.frame = gem.frame - 3.0 * time; if gem.frame < 1.0 { gem.frame = 18.0; }   // plays backwards
        gem.skill20 -= time; wait_ticks(1);
    }
    gem.skill20 = 16.0;                                         // phase 2 runs in ALL cases (also after a recall):
    while dist > 16.0 && gem.skill20 > 0.0 {                    //   exponential approach, 70 % of the gap per tick
        let a = to_angle(target(door) - gem.pos); gem.pan = a.pan; gem.tilt = a.tilt; dist = a.len;
        move_rel(gem, vec3(dist * 0.7 * mintime, 0, 0));
        particle_gem_keys(gem); gem.frame = wrap_back(gem.frame); gem.skill20 -= time; wait_ticks(1);
    }
    if dist <= 20.0 && door.skill14 != 0 {                      // arrived and not recalled: VALIDATE
        gem_explose(gem);
        door.skill15 = 1; door.flag1 = false;                   // door: closer loop ends → doordark, stays up for good
        sammy.player_old_key = 0;
        gem.passable = true; gem.invisible = true; gem.event = None; wait_ticks(16); remove(gem);
        return;
    }
    // recalled (Sammy walked 346-500 away while Skill5 != 0) or timed out: fly back to Sammy
    gem.skill20 = 80.0;
    loop {
        let a = to_angle(sammy.pos + vec3(0, 0, 100) - gem.pos); gem.pan = a.pan; gem.tilt = a.tilt;
        if !(a.len > 32.0 && gem.skill20 > 0.0) { break; }
        if a.len > 128.0 { move_rel(gem, vec3(40.0 * mintime, 0, 30.0 * mintime)); }   // fast, climbing in its local frame
        else { move_rel(gem, vec3(a.len * 0.9 * mintime, 0, 0)); }
        particle_gem_keys(gem); gem.frame = wrap_back(gem.frame); gem.skill20 -= time; wait_ticks(1);
    }
    sammy.player_old_key = 0;
    sammy.player_key = gem.color;                               // key given back (QUIRK: overrides a master key won meanwhile)
    hud_show_key();
    while gem.scale_x > 0.05 { gem.scale_xy *= 0.5; wait_ticks(1); }
    gem.passable = true; gem.invisible = true; gem.event = None; wait_ticks(16); remove(gem);
}
```

How the door and the gem interact, in play:
1. Sammy with the right key comes within 346 (horizontal) of a `dfix` door. The door rises (40/tick, ~8 ticks) and a gem leaves Sammy's head towards a point 100 above the door's origin; Sammy's HUD key empties.
2. While the gem flies, `Skill14 == 1` keeps the door up (the hold loop refreshes every 4 ticks), and Sammy's scan keeps calling `ouvre_portail` through the bad-key path (his key is now 0).
3. If Sammy comes within 128 horizontally, `Skill15 = 1` ends phase 1 early and the gem snaps in (phase 2).
4. On arrival: the gem bursts, the door morphs to `doordark.wmb` and stays up; Sammy's `PlayerOldKey` is cleared.
5. If Sammy walks out to 346-500 (and the door has Skill5 ≠ 0), `Skill14 = 0`: the gem still finishes phase 2 towards the door, then, not validated, flies back and returns the key; the door's hold ends (`Skill9 = 5`) and it closes once nobody is within `Skill2 + 100` / 250. Beyond 500 there is no scan event, so the gem completes and validates.
6. Doors with Skill5 = 0 never recall the gem.
7. `dfixall` (Flag6, no key needed) with key 0 creates no gem; it validates only when Sammy passes under it (< 128), otherwise it stays up while he is near and then holds open (Skill5 = 0 never lowers) [R].
8. With the master key (score goal reached), the gem is `gemmu` and Sammy keeps 255.
9. Death during a flight: `ResetSammy` ejects nothing (key already 0); the gem still completes or returns its key to the respawned Sammy [I].

### 2.6 Moving platforms: unlock morph, riders, gems and lions (`mm1.wdl:2238-2629`)

Already ported: engine motion, impact turn/reflect/pause, the platform following its engine, Sammy carried, the key test, Shift ×3. Exact remaining behaviour:

```rust
fn init_engine_platform(e) {                                    // :2459-2591
    if e.skill1 == 0.0 { e.skill1 = 8465.0; }
    wait_ticks(2);
    e.enable_impact = true; e.enable_block = true; e.enable_entity = true; e.push = -3;
    e.event = moteurBonus_Hit;
    scan_sphere(e.pos, radius: 50000.0, id: e.skill1); wait_ticks(1);
    scan_sphere(e.pos, radius: 10000.0, id: e.skill1);          // links platforms whose Skill1 == id (SetLinkMoteur)
    while partie_en_cours != 1 { wait_frame(); }                // no motion during the level-start camera sweep
    e.flag2 = true;                                             // "moving": riders/gems/lions are carried only while set
    if e.skill6 != 0.0 { while !e.flag5 { wait_ticks(4); } }    // locked: poll every 4 ticks (≤ 4 ticks start delay)
    e.flag1 = true;
    loop {
        while e.flag2 {
            let mut v = e.skill2 * mintime;
            if boost_held && window_boost < WIDTH_BAR_BOOST && id_platform == e.skill1 { v *= 3.0; }  // Shift/Joy3
            move_rel(e, vec3(v, 0, 0));
            e.bounce_force1 = my_speed;                         // this tick's motion (read by platforms)
            wait_ticks(1);
        }
        for each linked platform p { p.bounce_force1 = ZERO; }
        if e.skill3 > 0.0 { wait_ticks(e.skill3); }
        e.flag2 = true;
    }
}
```

Note `e.flag2` is **true while locked** (set before the lock wait), so a locked platform still "carries" in the sense of the riders' tests, but its motion is zero.

```rust
fn real_init_platform(p) {                                      // :2250-2272, after the engine's scan matched Skill1
    p.push = 1; p.objet_poussant = 1.0;                         // Skill44 (also BounceForce, overwritten just below)
    set_link_moteur(p, engine);                                 // offset Skill40-42 = p.pos - engine.pos
    p.enable_scan = false; p.enable_sonar = true; p.enable_impact = true; p.event = PlatForm_Hit;
    p.bounce_force = if p.skill2 == 0.0 { 1.5 } else { p.skill2 };   // all shipped: 1.5
    p.skill1 = 90273;                                           // camera-transparent like doors
    loop {                                                      // LoopPlatForm :2273-2304
        if engine.flag2 && !p.flag3 {                           // follow, unless Sammy's sonar already moved it this tick
            p.bounce_force1 = (engine.pos + p.offset) - p.pos; p.pos += p.bounce_force1;
        }
        if ticks1 % 2 == 0 { p.flag3 = false; }
        wait_ticks(1);
    }
}
fn platform_hit(p) {                                            // :2305-2416
    if event_type == EVENT_IMPACT {                             // something moved into the platform's side
        you.bounce_force1.xy = -you.ennemy_speed.xy * p.bounce_force;   // ×1.5 push back (Sammy, lions, gems)
        return;
    }
    if event_type == EVENT_SONAR && you.id == 3333 {            // Sammy's floor trace hit this platform
        if you.z <= you.blob_haut_sol + you.blob_haut + 41.0 && you.blob_vspeed == 0.0 {   // grounded on it
            p.flag3 = true;
            let e = p.entity2;                                  // engine
            if e.skill6 != 0.0 && !e.flag5 {                    // locked
                let key = pov_syn.player_key;
                let ok = if !e.flag7 { key & e.skill6 != 0 && (key == e.skill6 || matches!(key, 1|2|4)) }
                         else { key == e.skill6 };
                if ok { unlock(e, p, sound: true); }
                if key == 255 { unlock(e, p, sound: false); }   // master key: NO platform1_son [R :2350-2356]
            }
            if e.flag2 {
                id_platform = e.skill1;                         // enables Shift ×3 for this engine
                p.bounce_force1 = (e.pos + p.offset) - p.pos; p.pos += p.bounce_force1;
                you.player_force = p.bounce_force1;             // Sammy carried (HandleSpeedPlayer applies it)
            }
        }
        return;
    }
    if event_type == EVENT_SONAR && p.entity2.flag2 {           // other sonar users, platform "moving"
        if you.id == 7345 {                                     // gem with Flag7 (or launched gem)
            let rest = if you.flag5 { you.haut_ball + 4.0 } else { HEIGHT_GEM + 4.0 };
            if you.z <= you.haut_sol_ball + rest {
                you.skill43 = 1.0; you.player_force = p.bounce_force1;   // carried this tick (§2.2)
            }
        }
        if you.id == 8888 && you.z <= you.haut_sol_ball + you.blob_haut + 41.0 {   // alias noise: enemy-ai.md §6.4
            you.lion_force.xy = p.bounce_force1.xy;
        }
    }
}
fn unlock(e, p, sound) {
    e.flag5 = true;                                             // engine starts (within 4 ticks)
    morph(p, "platfor1.wmb");                                   // ONLY the platform Sammy stands on; new model + hull
    gem_light(p, e.skill6);                                     // coloured light 300 → 100 at -15/frame
    if sound { play_entsound(p, "platform1_son", 1000); }
}
```

- **Morph targets** [D]: 38 locked platforms use coloured prefabs (`platgree` 10, `platfred` 7, `platfor2` 7, `platfmag` 6, `platfor3` 4, `platfora` 3, `platfyel` 1); 36 `platfor1` platforms are unlocked already. Unlocking swaps the coloured prefab for the neutral `platfor1.wmb` (visual cue that it is "open"). Port: replace mesh + collision brush with `platfor1.wmb` placed at the platform's origin [U: compare hull extents; the prefabs look the same shape].
- **Gems on platforms**: only gems with Flag7 (WED) or lifetime/1000/1001 runtime gems run the sonar each tick, and only they get carried. Extro: the 35 wagon gems (Flag5+Flag7) ride engine 7788 with Sammy; 180c_t: 9 Flag7 gems.
- **Lions on platforms**: `LionForce = platform motion` while the lion's floor trace hits the platform; applied and decayed by the lion's `HandleForcesOnEnnemy` (`enemy-ai.md` §6.1, §6.4).
- **InitBodyPlatForm** (0 shipped uses): links to the engine but has **no follow loop**, so bodies never move [R, the engine's body-moving loop is commented out at `:2528-2550`].
- **Engine events** (`moteurBonus_Hit`): the engine has `enable_impact` and `enable_entity`, so an entity walking into the engine sprite, or the engine moving into an entity, also triggers the turn/pause [R]. Whether the tiny `small.pcx` engine sprite actually collides is [U]; the port only reacts to level geometry.

### 2.7 `InitTrampoline` (`mm1.wdl:3506-3572`) — no shipped entity

```rust
fn init_trampoline(t) { t.enable_scan = true; t.event = RealInitTrampoline; }
fn real_init_trampoline(t) {                                    // first scan received decides
    if event_type != EVENT_SCAN { return; }
    if you.skill1 == t.skill1 { set_link_moteur(t, you); return; }   // linked to an engine (but, like bodies, never moved)
    wait_ticks(8); t.enable_scan = false; t.enable_sonar = true; t.event = Trampoline_Hit;
}
fn trampoline_hit(t) {                                          // EVENT_SONAR from an entity standing on it
    if !(t.flag1 || you.id == t.skill2 || you.id == t.skill3) { return; }
    if you.blob_vspeed == 0.0 {
        you.blob_vspeed = t.skill5 * (you.blob_vspeed_org as i32) as f32;   // launch
        you.blob_fric_vspeed = t.skill6 * you.blob_fric_vspeed_org;
    }
    if t.flag2 {                                                // slow/accelerate tile
        if you.id == 3333 || you.id >= 55551 { you.blob_ralentit = t.skill4; } else { you.ball_accel = t.skill4; }
    }
    if t.flag3 {                                                // spin what stands on it
        you.pan += t.skill7 * time;
        if you.id != 77 && you.id != 78 { you.tilt += t.skill8 * time; }
    }
}
```

Sammy's `BlobVSpeedOrg` (Skill4) and `BlobFricVSpeedOrg` (Skill8) come from his WED skills [I]. Skip unless a mod level needs it.

### 2.8 `Player_Bong`: Sammy bumping non-lion entities (`player.wdl:520-697`)

Sammy has `enable_entity` and `enable_impact` but **not** `enable_block` (`player.wdl:48-53`): walls never produce a knockback, only entities do.

```rust
fn player_bong(sammy, you, event) {
    if event == EVENT_STUCK { play_sound("beep1", 10); }
    if event == EVENT_ENTITY {                                   // Sammy's MOVE hit `you`
        sammy.pente = ZERO;                                     // cancel slope slide
        if you.id == 7345 { return; }                           // gems: nothing
        if you.objet_poussant != 0.0 {                          // Skill44 ≠ 0: lions (frc(Skill2)·100 or LionForce), platforms 1.5,
                                                                // hives 10 (bullets.wdl:1207), hive wasps 0.1 (:1015), ammo pickups 1-3 (:226-310)
            sammy.bounce_force1 = you.ennemy_speed * sammy.weight_force * you.bounce_force;   // Skill37-39 × 0.8 × Skill44
        } else {                                                // doors, weak doors, decor models, trees, boulders (Skill44 = Skill2 = 0), ...
            let mut n = normal;                                 // contact normal of the MOVE [U A5]
            if n.x + n.y == 0.0 { n = bounce.normalize(); }
            sammy.bounce_force1 = vec3(n.x, n.y, 0.0) * INTENSITE_BLOB_BOUNCE;   // 30 (InitLvl.wdl:80); 15 in 179f_t (InitGame4 Skill5)
        }
        if you.id == 8888 { lion contact rules: enemy-ai.md §4.1-4.2; return; }
        // ActionParticleBlob_Syn is ActionNulle for TypeJeu 25
        return;
    }
    if event == EVENT_IMPACT {                                  // `you` moved into Sammy
        if you.id == 7345 { return; }
        if you.objet_poussant != 0.0 { sammy.bounce_force1 = you.ennemy_speed * sammy.weight_force * you.bounce_force; }
        else { sammy.bounce_force1 = rotate(bounce, pan: 180) * INTENSITE_BLOB_BOUNCE; }   // includes z
        if average_fps > 35 && sammy.light_range == 0 {         // orange-ish flash (150,200,10), LightRange1 (300) → 100 at -10/tick
            flash_light(sammy, (150, 200, 10), LIGHT_RANGE1, 10.0);
        }
    }
}
```

Consumption (`HandleSpeedPlayer`, `player.wdl:1415-1470`): each tick, `MOVE abs = BounceForce1·time` with events off, then each component is decreased towards 0 by `time·AmortiBouncePlayer·80/(AverageFps+1)` (AmortiBouncePlayer 10, `InitLvl.wdl:82`; 7 in 179f_t). While `|BounceForce1.x|+|.y| > 3` in a jump, air control is cut to 0 (`:1197-1200`).

Port: `slide_move` reports one touched entity (`PlayerTouched`), consumed only by lions. Add: for any solid non-lion, non-gem entity hit by Sammy's own move, set `knock = contact_normal.xy · 30` (15 in 179f_t); for an entity with `ObjetPoussant` ≠ 0 other than a lion (platform 1.5, hive 10) set `knock = its EnnemySpeed (Skill37-39) · 0.8 · ObjetPoussant`. Platforms and hives never write Skill37-39, so this is **zero**: bumping them cancels any knockback. Which entities are solid for Sammy is listed in §2.13 (trees with the ×0.3 hull, cactus, boulders, hives, doors, platforms, weak doors).

### 2.9 Config entities whose values the port ignores

| Entity | Skill/flag → global | Shipped values [D] | Port |
|---|---|---|---|
| `InitGame5` (`mm1.wdl:1912-1924`) | Skill1 `SpeedRotationPlayer`, Skill2 `SpeedRotationPlayerMove` | 10 (18 levels) / 9 or 10 (default 7, `InitLvl.wdl:110-111`) | hard-coded |
| | Skill4 `SlopeAngleMin` | **0.9** in 30 levels, 0.98 in 179f_t (default 0.92) | hard-coded 0.92 |
| | Skill5 `AccPente`, Skill6 `ForcePente`, Skill7 `AmortiPenteEnd`, Skill8 `BigSlope` | 1 / 200 in 108i_j, 109o_t, 110n_t; 0.2 / 20 in 113q_t, 114t_t; 0.2 / 100 in 171c_t04, 173b_t05; **2.5 / 150 / 0.3 / 3 in 179f_t (ice)**; others default (1.5 / 100 / 0.3 / 10) | hard-coded defaults |
| | Skill3 `GemSonar` | 0 | n/a |
| `InitGame4` (`mm1.wdl:1891-1910`) | Skill5 `IntensiteBlobBounce`, Skill6 `AmortiBouncePlayer` | 15 / 7 in 179f_t only (defaults 30 / 10) | §2.8 |
| | Skill3 `Clip_RangeGame` (×1.5 at runtime), Skill1/2 mip, Skill8 `TitlesOn` | render only; Skill8 0 everywhere | n/a |
| `InitGame3` | applied twice in 109o_t (`[20,0,…]` then the usual) and 179f_t (`[0,…,15000]` + usual) | each instance only sets its non-zero skills, so the union applies; for 109o_t Skill1 (jump speed 20 vs 30) depends on spawn order [U: entity order in the WMB]. | first-wins/last-wins by spawn order; check |
| `InitCamera3` | Skill5 ground clearance, Skill8 start-sweep friction, Flag1/Flag2 doors solid/opaque to the camera | 64 (27 levels), 0.6 (24); Flag2 in 10 levels | §2.12 |
| `InitCamera1` Flag1 / Flag3 / Flag4 | camera may move in Z / type 1 / type 2 | §2.12 | §2.12 |
| `InitGame7` Skill5 `GraviteEnnemy`, Skill8 `PlayerMoveMode` | -1 in one level; 0 | check `apply_game7` |

The slope set is gameplay-relevant (179f_t "slides in Sammy's land" is built around it); wire `InitGame5` into the player's slope constants.

### 2.10 Lions on level objects (what the port still needs)

From `enemy-ai.md` §5.4, §6.2-6.6 (specs there); status in the port (`actions/enemy.rs` header says these are not done):

| Interaction | Needed by shipped data? | Port needs |
|---|---|---|
| Teleport pads with Flag2 (`mm1.wdl:4236-4285`) | **yes**: 20 pads in 171c_t04, 173b_t05, 178d_t, 179f_t | grounded lion on the pad and pad not busy (shared `skill12` lock with Sammy) → id 0, passable, alpha 100→0 at -2/tick forcing state 0, jump to (Skill1-3), alpha 0→100 at +1.5/tick, restore |
| Moving platforms | yes (17 levels have both) | `lion_force.xy = platform motion` while standing on a moving platform; lion→platform side IMPACT push-back ×1.5 |
| Doors closing on lions (`DoorScanEntities`) | yes (any level with doors and lions) | count lions in the 102 clear-scan |
| Doors opened by lions (Flag3) | no | — |
| Lava (`Lava_Hit`) | no (`InitLava` unused) | — |
| Death tiles with Flag2 | no | — |
| `DeathDalle` | no | — |
| `InitStartEnnemy` | 180c_t | §2.13 |
| Scan 25 from Flag5 lions (titles, camera zones) | extro only | §2.14 |
| Crushing gems (Flag6) | no | — |
| `pov_Syn` retargeting | extro only | §2.14 |
| Weak doors | ported (`wasps.md` §6.2) | — |

### 2.11 Hives (`InitHive`, `bullets.wdl:1197-1321`)

Specified in `wasps.md` §8 (hit points, `EmitWasp`, `LaunchNewWasp`). Shipped: `hivegree.wmb` ×2 and `hiveblue.wmb` ×1 in 176g_t and 177e_j. Port: not dispatched (static brush). They are ammo sources in those two levels, so they matter for finishing them only if the level's other ammo is insufficient [I].

### 2.12 Camera zones and camera types 1/2

Scope: `SetCamera`/`SetCameraHit`/`CameraSurveillance` (`camera.wdl:958-1233`), camera types 1 and 2 (`HandleCamera :13-136`, `HandleCamera1 :278-307`, `CameraHandle4 :312-317`, `SetPosCamera :1236-1270`, `SetCameraType :933-951`), and how the camera type is picked at level start, respawn and teleport. The type-3 follow camera itself is in `gameplay.md` §6.2 and is not repeated here. Pseudo-code assumes a fixed 16 Hz tick (`time = mintime = 1`).

#### Corrections to `gameplay.md` §6

- **§6.1 `InitCamera2`:** the frictions do not map to "types 1/2/3" in order. Skill1 sets `Camera1Friction` and Skill4 sets `Camera1VertFriction`, and these are read by `HandleCamera`, which is **type 2** (`camera.wdl:54-55`). Skill2 sets `Camera2Friction` and Skill5 sets `Camera2VertFriction`, which are read by `HandleCamera1`, which is **type 1** (`camera.wdl:288-289`). Skill3 and Skill6 are type 3 (`mm1.wdl:2804-2821`).
- **§6.1 `InitCamera1` flags:** the flags are tested in the order Flag2, Flag3, Flag4, and **each later one overrides** the earlier ones (`mm1.wdl:2775-2800`). With no flag set, the type is 2.
- **§6.4 friction cap:** the 0.25 cap on `Camera3Friction`/`Camera3VertFriction` is applied **after** the interpolation loop, for 4 ticks only. It is not applied during the transition (`camera.wdl:1142-1157`).
- **§6.4 "no change" test:** it compares distance, height and PlusTilt. It does **not** compare the back angle (`:1019`).
- **§6.4 surveillance and Flag2:** none of the 741 shipped zones uses either (see the data below), so both are dead in the shipped levels.

#### Shipped data (`*.entities.json`)

**Camera type at level start** (`InitCamera1` flags; Flag1 means `CameraCanMoveInZ`):

| Type | `InitCamera1` flags | Levels |
|---|---|---|
| **2** (`HandleCamera`) | Flag1+Flag4 | 10001, 10002, 10003, 10004, 10005, 10006, 10007, 102N_t, 104M_j, 106g_t |
| **2**, fixed Z | Flag4 only | 108i_j |
| **3** (`SuricateCameraHandle`) | Flag1+Flag2 | 10008, 10009, 10010, 10011, 10012, 101o_j, 103J_j, 107i_j, 109o_t, 110n_t, 111r_j, 112e_j, 113q_t, 114t_t, 170b_t00, 171c_t04, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 177e_j, 178d_t, 179f_t, 180c_t, extro |
| 3, fixed Z | Flag2 only | menu_greenpepper |
| **1** (`HandleCamera1`) | Flag3 | **none** |

All 11 type-2 levels set `InitCamera2` Skill1 = 10 (108i_j: 1) and Skill4 = 0.1 (108i_j: 0, so the default 0.1 applies). The horizontal factor is therefore `min(10·1, 1) = 1`, which **snaps** horizontally every tick at 16 Hz.

**`SetCamera` zones:** 741 entities in 21 levels.

| Level | Zones | Level | Zones | Level | Zones |
|---|---|---|---|---|---|
| 178d_t | 122 | 176g_t | 103 | 179f_t | 85 |
| 175c_t | 78 | 173b_t05 | 48 | 103J_j | 45 |
| 111r_j | 38 | 107i_j | 36 | extro | 34 |
| 174a_j07 | 27 | 113q_t | 24 | 177e_j | 23 |
| 180c_t | 23 | 109o_t | 20 | 170b_t00 | 11 |
| 171c_t04 | 7 | menu | 7 | 10012 | 6 |
| 108i_j | 2 | 112e_j | 1 | 114t_t | 1 |

- **Models:** 525 `setcam.wmb`, 180 `cliprang.mdl`, 29 `spheran1.mdl`, 7 `small.pcx`. All are morphed to `small.pcx` and made invisible, so the model does not matter.
- **Skill1 (type):** **3 in 740 zones**. Exactly one zone has Skill1=2: 170b_t00, skills `(2,400,700,180,250,50,200,0)`. That zone switches the level to type 2 until another zone switches it back.
- **Skill2-5:** all non-zero except a few Skill5=0 or negative (extro, menu). Skill4 is 180 or 181 in normal levels. Extro and menu use real cinematic angles (-181..350).
- **Skill6 (steps):** 5..300.
- **Skill7 (trigger distance):** 50..500. Values above 500 can never trigger, because the scan radius is 500.
- **Skill8: always 0**, so `CameraSurveillance` never runs in shipped data.
- **Flags:** Flag2 is never set. Flag3 is set on 2 zones (10012, 170b_t00) and is not read by any camera code.
- **108i_j** is a type-2 level with two Skill1=3 zones (Skill7=300), so walking into them turns its camera into type 3 until the next respawn.
- **Overlap:** Skill7 spheres overlap often. Counting other zones whose Skill7 sphere contains a given zone's origin, the mean is 2.2 in 174a_j07, 2.0 in 113q_t, 1.6 in 103J_j, 1.5 in 173b_t05 and 1.3 in 176g_t. This matters for the same-tick race described below.
- **Other id-25 scanners:** lions and wasp-enemies with Flag5 also scan with id 25 (`lion.wdl:1810-1818`, `bullets.wdl:1735-1742`). They therefore also trigger zones, with `you` = the lion. Only extro has such entities (4 of them).

#### `SetCamera` (init, `camera.wdl:958-978`)

```rust
temp = skill1;                               // :959 (unused)
morph("small.pcx"); invisible = true; passable = true; enable_scan = true;   // :960-964
event = SetCameraHit;                        // :965
// Surveillance point: the zone's own position if Skill2==0, else (Skill2,Skill3,Skill4).
skill20_22 = if skill2 == 0 { pos } else { (skill2, skill3, skill4) };      // :966-977
```

Skill meanings (`:980-1000`):
- Skill1: camera type.
- Skill2: distance (`PersoDistCam`).
- Skill3: height (`HautCamPerso`).
- Skill4: back angle (`PersoAngArriere`).
- Skill5: `Camera_PlusTilt`.
- Skill6: number of steps (ticks).
- Skill7: trigger distance.
- Skill8: if non-zero, surveillance mode.
- Flag2: keep `CameraBouge = 1` afterwards (locks out all later zones).

**Port:** `setcamera` is currently in `is_marker` (hidden, no behaviour). The zone must stay invisible and passable, but needs a component with skills 1-8, Flag2, the surveillance point and an "armed" bool (the WDL `event` field).

#### `SetCameraHit` (`camera.wdl:1002-1173`). One async task per zone.

**Trigger.** `SetCameraHit` is triggered from Sammy's per-tick door scan (`player.wdl:231-238`): a 360×360 sphere of radius `RangeScanDoor = 500` (`mm1.wdl:165`), with `ScanIdentite = 25`. The scan runs in the `Marche` loop just **before** `Call CameraHandle_Syn` (`player.wdl:241`), so a zone hit is evaluated before that tick's camera update.

```rust
// --- synchronous part, inside the scan event (you = scanner) ---
if scan_identite != 25 { return; }                                  // :1015
if camera_bouge != 0 { return; }                                    // :1016  one transition at a time
if camera_type == skill1 {                                          // :1017
    if dist == skill2 && haut == skill3 && skill5 == plus_tilt { return; }   // :1019-1023 exact float compare; back angle NOT compared
    if skill8 != 0 && survey_pos.x == skill20 && survey_pos.y == skill21 { return; } // :1024-1030 (dead)
}
// Line of sight zone -> scanner; ignores me, you, passable, sprites, models (doors and other map entities block).
if trace(pos, you.pos, IGNORE_ME|IGNORE_YOU|IGNORE_PASSABLE|IGNORE_SPRITES|IGNORE_MODELS) != 0 { return; } // :1031-1035  [U] trace returns 0 when clear
camera_bouge = 0;                                                   // :1037 (already 0 here; only matters in the race below)
wait_ticks(1);                                                      // :1038
// --- next tick ---
if (pos - you.pos).length() > skill7 { return; }                    // :1040-1047 3D distance (To_Angle .z) [U] `you` still the scanner after waitt
armed = false;                                                      // :1050 event = null
skill6 = max(skill6, 1);                                            // :1053
camera_bouge = 1;                                                   // :1054
if skill8 != 0 { return camera_surveillance(); }                    // :1056-1059 (dead in data)
camera_mode = 1;                                                    // :1061
if camera_type != 3 && skill1 == 3 { camera_syn.pos = camera.pos; } // :1063-1070
camera_type = skill1;
set_camera_type();                                                  // :1072-1073, see below
let k = 1.0 / skill6;                                               // :1074 Skill10
let mut n = skill6 + 1;                                             // :1075 Skill9: one step MORE than Skill6
let d_dist = skill2 - dist;                                         // :1076 Skill13
let d_haut = skill3 - haut;                                         // :1077-1085 Skill14 (same for every type)
let d_rel  = if camera_type == 3 { -ang_rel_cam_perso.pan } else { 0 };   // :1080 Skill17 (AngRelCamPerso is always 0 in practice: mouse cam dead, Cameramouse never set)
let d_back = ang(skill4 - back);                                    // :1087 Skill15 shortest way, ang() -> -180..180 [U range]
let d_tilt = skill5 - plus_tilt;                                    // :1088 Skill16
while n > 0 && camera_bouge == 1 {                                  // :1094
    dist += d_dist * k;                                             // :1096
    haut += d_haut * k;                                             // :1097-1111
    if camera_type == 3 { ang_rel_cam_perso.pan = ang(ang_rel_cam_perso.pan + d_rel * k); }
    back = ang(back + d_back * k);                                  // :1108-1109
    plus_tilt += d_tilt * k;                                        // :1110
    if camera_type == 1 { type1_recompute_delta_and_aim(); }        // :1112-1126, see type 1
    n -= 1;
    wait_ticks(1);                                                  // :1136
}
// Steps 1..skill6 reach the target and step skill6+1 overshoots by one step; the snap below corrects it.
// The loop also exits early if someone sets camera_bouge = 0 (respawn, teleport, wasp-6, extro). The snap still happens.
if !flag2 { camera_bouge = 0; }                                     // :1138-1141
camera3_vert_friction = min(camera3_vert_friction, 0.25);           // :1142-1145
camera3_friction      = min(camera3_friction, 0.25);                // :1146-1149
camera_mode = 1;                                                    // :1150
dist = skill2; haut = skill3; back = skill4; plus_tilt = skill5;    // :1151-1154 raw values (back may be 350, -30...)
wait_ticks(4);                                                      // :1155
camera3_vert_friction = camera3_vert_friction_org;                  // :1156
camera3_friction      = camera3_friction_org;                       // :1157
if camera_type == 1 { type1_recompute_delta_and_aim(); }            // :1159-1171
armed = true;                                                       // :1172 event = SetCameraHit
```

**Effects of `camera_bouge == 1` on the type-3 handle during the transition**, which the port must also implement:
- In `CamFollowMe`, with `CameraState == 1`, `camera.pos` snaps to `Camera_Syn` every tick. Both proxies become passable and `CameraHitObject` is reset (`camera.wdl:509-521`).
- `LookAtTarget` **snaps** the tilt (no 0.4 ease), skips the ≤20° clamp and skips the 3.5×-distance emergency switch (`:837-858`).
- The emergency-camera reset after more than 4 ticks without line of sight is suppressed (`:647`).

**`SetCameraType`** (`camera.wdl:933-951`):

| Type | Handle | Also sets |
|---|---|---|
| 1 | `HandleCamera1` | nothing |
| 2 | `HandleCamera` | `CameraState = 1` |
| 3 | `SuricateCameraHandle` | `CameraState = 1`, `CameraMode = 1` |

A zone trigger in a type-3 level therefore also forces an emergency (state 2) camera back to state 1.

**Same-tick race [R, consequence I].** The `camera_bouge != 0` guard is checked before the `waitt 1`, but `camera_bouge = 1` is set only after it. Every zone the scan hits in the same tick and passes the checks therefore starts a transition. Each one computes its deltas from the values *as modified by the previous ones*, and all the loops run in parallel. With two identical zones the camera parameters overshoot to almost twice the delta, then snap back at the end. Which zone's final snap wins depends on scheduler order [U].

Recommended port: among the zones passing the synchronous checks in one tick, keep **only the first in scan or entity order**. Note this as a deliberate deviation, and replicate the race only if it is visibly needed (it can happen in 174a_j07, 113q_t, 103J_j, 173b_t05, 176g_t).

**Interaction with respawn** (`player.wdl:962-990`). The order is:
1. Respawn sets `HautCamPerso/PersoDistCam/PlusTilt/PlusTiltMin = *Org` (`PersoAngArriere` is **not** reset).
2. It sets `camera_bouge = 0`, waits one tick, sets `camera_bouge = 1`, and restores `CameraType = CameraTypeStart` (zones that changed the type are undone).
3. An interrupted zone task then runs its post-loop snap and **overwrites the Org values with the zone's** [I, from task ordering].

The teleport has the same `camera_bouge` 0→1 pattern (`mm1.wdl:4174-4231`).

#### `CameraSurveillance` (`camera.wdl:1175-1233`). Not used by shipped data; spec for completeness.

```rust
camera_mode = 2;                                                    // :1177
survey_pos = skill20_22;                                            // :1178-1180
let a = to_angle(survey_pos - camera.pos);                          // :1182-1185 (a.z = length)
let step = a.z / skill6;                                            // :1186
let mut n = max(skill6 - 1, 1);                                     // :1188
camera_syn.pos = camera.pos; camera_syn.pan = a.pan; camera_syn.tilt = a.tilt;   // :1189-1193
while n > 0 && camera_bouge == 1 {                                  // :1194
    camera_syn.passable = true;
    move_rel(camera_syn, (step, 0, 0));                             // :1196-1200 forward along its pan/tilt
    camera.pos = camera_syn.pos; n -= 1; wait_ticks(1);             // :1201-1205
}
let mut tempo = 32.0;                                               // :1211
while (camera.pos - survey_pos).length() > 2.0 && tempo > 0.0 {     // :1212
    camera.pos += (survey_pos - camera.pos) * 0.1 * mintime;        // :1214-1216
    tempo -= time; wait_ticks(1);
}
if camera_bouge == 1 { camera_syn.pos = skill20_22; camera.pos = skill20_22; } // :1222-1230
camera_bouge = 0; armed = true;                                     // :1231-1232 (Flag2 and the friction cap are not applied here)
```

While `CameraMode == 2`, the type-3 handle only runs `LookAtTarget` (`camera.wdl:434-438`): the camera stays put and turns to Sammy. The next normal zone (`:1061`/`:1150`) or the wasp-6 return (`:415-423`) sets mode 1 again. `HandleCamera` (type 2) ignores `CameraMode`.

#### Level-start camera (`InitCamera1`, `InitGame2`, `StartintroGame`)

**Defaults** (`InitLvl.wdl:150-187`, every level):
- `CameraHandle_Syn = null`, `CameraType = 1`, `CameraState = 1`, `CameraMode = 1`.
- Frictions: `Camera1Friction = Camera2Friction = 1`, `Camera1VertFriction = 0.1`, `Camera2VertFriction = 0.05` (`skills.wdl:641-654`).
- `CameraPosOrigine = (832,-316,1550)`, pan 90, tilt -65 (`InitLvl.wdl:50-54`).

**`InitGame2`** (`mm1.wdl:1835-1846`): each of Skill2/3/4 (origin x/y/z) and Skill5/6 (pan/tilt) overrides the default only when non-zero. A tilt of 90 becomes -90. The port does `apply_init_game2`, but takes the skills unconditionally, so a zero skill wrongly replaces the default.

**`InitCamera1`** (`mm1.wdl:2719-2800`):

```rust
create("small.pcx", pos, CreateCameraSyn);    // :2721 Camera_Syn (invisible; after 8 ticks: fat hull, enable_entity, event camera_hit, camera.wdl:243-258)
                                              //       + Camera2_Syn (invisible, passable, camera.wdl:259-268)
wait_ticks(4);                                // :2723 everything below runs 4 ticks after spawn
// skills 1-8 as in gameplay.md §6.1 (Skill8 always applied)
camera_can_move_in_z = flag1;                 // :2767-2773
camera_type = 2; camera_type_start = 2;       // :2775-2776 default; handle stays null
if flag2 { handle = SuricateCameraHandle; camera_type = 3; camera_type_start = 3; }   // :2778-2784
if flag3 { camera_type = 1; camera_type_start = 1; handle = HandleCamera1; }          // :2785-2791
if flag4 { camera_type = 2; camera_type_start = 2; handle = CameraHandle4; }          // :2792-2799
```

The handle chosen here does not matter. `StartintroGame` overwrites it through `SetCameraType` (`mm1.wdl:758-759`). `CameraHandle4` (`camera.wdl:312-317`) is just `my = Pov_Syn; HandleMouseCam (dead, Cameramouse==0); HandleCamera`, which is identical to type 2.

**`AfterLoadInitLevel`** (`InitLvl.wdl:198-228`):
- `camera.pos/pan/tilt = CameraPosOrigine/CameraAngleOrigine`, and `camera.arc`/`camera.ambient` are set.
- If `Camera_Syn` already exists, it is moved to `CameraPosOrigine` [U whether it exists yet; if not, it stays at the `InitCamera1` entity's origin].
- If the handle is still null, it becomes `CameraHandle4`.

**`StartintroGame`** (`mm1.wdl:744-829`):

```rust
// Re-applies the origin every Tempo_LoadLevel until InitCamera1 has run (≤16 tries).   :744-756
camera_type = camera_type_start; set_camera_type();                 // :758-759
if camera_type == 2 { set_pos_camera(); }                           // :760-763  no sweep, see below
if camera_type == 3 {                                               // :766-815  the "level-start sweep"
    camera_handle();                                                // :768
    camera_bouge = 0; wait_ticks(1); camera_bouge = 1;              // :770-772  -> type-3 snap/no-clamp rules above
    adjust_fps_cam3 = 0.1;                                          // :775
    clip_range = clip_range_start (20000);                          // :776
    if camera3_friction_start == 0 { camera3_friction_start = camera3_friction_org; }   // :777-778
    camera3_friction = camera3_vert_friction = camera3_friction_start;                  // :779-780  BOTH axes use InitCamera3.Skill8 (default 0.5; shipped 0.6/0.2/0)
    let mut tempo = 128.0;                                          // :774
    loop {
        let v = camera_delta.x.abs() + camera_delta.y.abs() + camera_delta.z.abs();     // :773/:810  L1 norm of the last per-tick STEP, not the gap
        if !(v > 4.0 && tempo > 0.0 && !key_enter && !key_esc && !joy_2) { break; }    // :781
        camera.fog = max(camera.fog - 10 * time, fog_level_intensity);                   // :783-787
        tempo -= time; wait_ticks(1); camera_handle();                                  // :789-809
    }
    camera3_friction = camera3_friction_org; camera3_vert_friction = camera3_vert_friction_org;   // :810-811
}
clip_range = clip_range_game * 1.5; camera_bouge = 0;               // :815-819
camera.fog = fog_level_intensity; partie_en_cours = 1;              // :824-829  -> Marche loop starts, calls the handle every tick
```

**The sweep exists only for type 3.** Type-2 levels (10001-10007, 102N_t, 104M_j, 106g_t, 108i_j) start playing immediately.

Type-3 sweep step size: `v1 = v2 = min(Skill8 · 0.1, 1)`, which is 0.06/tick with the usual Skill8 = 0.6. The sweep therefore ends once the per-tick step falls to ≤ 4 (L1), roughly when the remaining gap is under ~67 quants, or after 128 ticks.

`set_camera_type` with type 1 never calls `SetPosCamera`, so a type-1 start would leave `BlobCamDelta = 0` (camera inside Sammy). This is moot, because no level uses Flag3.

#### `SetPosCamera` (`camera.wdl:1236-1270`). Used at type-2 start, respawn and teleport.

```rust
let a = if camera_type == 1 { ang(back_org + 90) } else { ang(pov.pan - back_org) };   // :1237-1246
camera.xy = pov.xy + dist_org * (cos(a), sin(a));
camera.z  = pov.z + haut_org;                                       // :1248
if !camera_can_move_in_z { camera_syn.z = camera.z; }               // :1250-1253  ONLY z of the proxy is set
pov.blob_cam_delta = camera.pos - pov.pos;                          // :1256-1258 (Skill32-34)
ang_cam_perso = pov.pan;                                            // :1260
let d = to_angle((pov.x, pov.y, pov.z + plus_tilt) - camera.pos);  // :1261-1264
camera.pan = d.pan; camera.tilt = d.tilt;                           // :1265-1266
```

`Camera_Syn.xy` is not moved, and `HandleCamera` copies `camera = Camera_Syn` on its next tick. For Flag1 levels the visible start is therefore the proxy coming from `CameraPosOrigine`: horizontally it snaps in one tick (`Camera1Friction` = 10 → factor 1), and vertically it eases slowly (see type 2).

#### Type 2: `HandleCamera` (`camera.wdl:13-136`). Called once per tick with `my` = Sammy (`player.wdl:241`).

```rust
if camera_state <= 0 { return; }                                    // :14
camera.diameter = 0;                                                // :16  [U] engine camera clip hull
ang_cam_perso += ang(me.pan - ang_cam_perso + ang_rel_cam_perso) * adjust_ang_rot_cam1;  // :18 (4/AverageFps, mm1.wdl:907; same yaw follow as type 3)
camera_target_z += (me.z + plus_tilt - camera_target_z) / 4.0;      // :19 per frame
let goal = match camera_state {
    1 => {                                                          // :21-39
        let a = ang(ang_cam_perso - back);                          // NOTE minus (type 3 uses +back, camera.wdl:453)
        let z = if camera_can_move_in_z { syn.z + (me.z + haut - syn.z) * 0.25 } else { syn.z };  // :28-35
        (me.x + dist * cos(a), me.y + dist * sin(a), z)
    }
    2 => syn.pos,                                                   // :40-45 unreachable: SetCameraType sets state 1 for type 2
    3 => /* :46-51, /24 and /10 easing */ unreachable!(),           // CameraState is never set to 3 anywhere
};
let v1 = min(camera1_friction * mintime, 1.0);                      // :54  shipped: 10 (or 1) -> 1.0 = horizontal snap
let v2 = min(camera1_vert_friction * mintime, 1.0);                 // :55  shipped: 0.1
camera_delta = ((goal.x - syn.x) * v1, (goal.y - syn.y) * v1, (goal.z - syn.z) * v2);   // :56-59
// Effective vertical rate with Flag1: 0.25 * 0.1 = 2.5 % of the gap per tick (time constant ≈ 40 ticks).
// Without Flag1 (108i_j): no vertical follow at all; height = start z + HautCamPersoOrg (from SetPosCamera).
syn.passable = true;
let r = trace(pov.pos, syn.pos, IGNORE_ME|IGNORE_YOU|IGNORE_PASSABLE|IGNORE_SPRITES|IGNORE_MODELS);   // :74-77
syn.passable = false;
if camera_time_secur > 12.0 || r < camera_angle.z * 0.8 {           // :81-85 camera_angle.z = last tick's camera->Sammy distance
    camera_time_secur = 0.0; syn.passable = true;                   // [U r==0 when clear] -> clear view or near obstacle: proxy flies through walls;
}                                                                   //  only an obstacle in the last 20 % next to the camera makes it collide
move_abs(syn, camera_delta);                                        // :87 glide, fat hull [U]
if camera_time_secur > 0.0 { camera_time_secur -= 0.1 * time; }     // :89-92 (only type 3 increments it)
camera.pos = syn.pos;                                               // :94-96
let (r, floor) = sonar(syn, 1000);                                  // :97  [U] r = height above floor; r == 0 if none
if r < 80.0 {                                                       // :98-102 ground clearance
    syn.z += (floor.z + 80.0 - syn.z) * 0.1 * mintime; camera.z = syn.z;
}
camera_angle = to_angle((me.x - syn.x, me.y - syn.y, camera_target_z - syn.z));   // :106-109
camera.pan = camera_angle.pan;                                      // :114 snap
camera.tilt += ang(camera_angle.tilt - camera.tilt) * 0.25 * mintime;   // :113 ease
inv_dir_camera = (ang(camera.pan - 180), -camera.tilt);             // :116-117
```

Differences from type 3, for the port:
- no `Camera2_Syn` emergency camera or zoom-in;
- no 20° tilt clamp;
- doors always block the line-of-sight trace (`DoorInvisible` is not consulted);
- no `CameraSolPlus` (ground clearance is the sonar rule above);
- no `AdjustFpsCam3`.

#### Type 1: `HandleCamera1` (`camera.wdl:278-307`). Unused by shipped data (only reachable through the debug `C` key, `ChangeCamera :881-929`).

```rust
if camera_state == 0 { return; }
let ideal = pov.pos + pov.blob_cam_delta;                           // :281-283 fixed world offset, no rotation
let (v1, v2) = if camera_bouge == 0 { (min(camera2_friction * mintime, 1.0), min(camera2_vert_friction * mintime, 1.0)) }   // :286-290
               else { (0.5, 0.5) };                                 // :300-305
camera.pos += (ideal - camera.pos) * (v1, v1, v2);                  // no proxy, no collision, orientation untouched
```

Its orientation and `BlobCamDelta` change only in a zone transition (`camera.wdl:1112-1126`, `:1159-1171`):
- `delta = (dist·cos(ang(back+90)), dist·sin(ang(back+90)), haut)`;
- the camera then aims at `pov + (0,0,plus_tilt)`.

Recommendation: do not port type 1 (and do not port `ChangeCamera`).

#### Port status (`crates/sammy/src/camera.rs`, `flow.rs`)

**Present:**
- A single simplified type-3 follow camera, used for **every** level. It covers yaw follow, ideal position with `+back`, a merged emergency zoom, the proxy friction ease, and look-at.
- The `InitCamera1` Skill1/4/7/8, `InitCamera2` Skill3/6 (and the arc bug), and `InitGame2` origin hookups.
- The level-start sweep from `CameraPosOrigine`, exiting on convergence, 128 ticks or Enter/Esc.

**Missing or wrong:**
1. **Type 2 for the 11 Flag4 levels:**
   - instant horizontal follow;
   - 2.5 %/tick vertical follow (none in 108i_j);
   - `ang - back` convention;
   - tilt ease 0.25;
   - `Camera_TargetZ` eased by 1/4;
   - sonar 80-quant floor clearance;
   - pass-through-wall rule;
   - `SetPosCamera` at start, respawn and teleport;
   - **no level-start sweep** (the port sweeps every level).
2. **Camera zones** (741 entities, 21 levels): the trigger checks, the 1-tick confirm, the Skill6+1-step linear tween of dist/height/back/PlusTilt, the snap, the 4-tick 0.25 friction cap, re-arming, the type switch (3↔2 in 170b_t00, 2→3 in 108i_j), and the `camera_bouge` effects on type 3 (snap camera to proxy, snap tilt, no clamp/emergency).
3. **Respawn** must restore `CameraType = CameraTypeStart` and the Org values (`player.wdl:962-976`).
4. **Sweep parameters:**
   - The port uses a hard-coded 0.5 horizontal factor and `fric_v` vertical. The original uses `InitCamera3.Skill8` (0.6 in most levels; 0 falls back to `Camera3FrictionOrg`) on **both** axes, times 0.1.
   - The port's exit test is "gap ≤ 4". The original's is "L1 step ≤ 4".
   - The port does not read `InitCamera3` at all (`Skill5` `CameraDistSol`, `Skill8` start friction, `Flag1`/`Flag2` door handling).
5. `CameraParams` lacks `can_move_in_z` (`InitCamera1` Flag1), the type, the min/max limits and the type-2 frictions (`InitCamera2` Skill1/Skill4).
6. `InitGame2` origin skills should be applied only when non-zero.

### 2.13 Decoration and miscellaneous actions (`mm1.wdl`)

All line numbers are `mm1.wdl` unless noted. Compile define `422` is active (`sammypolish.wdl`), so every `IfDef 422` branch below is the live one and every `IfNdef 422` branch (`Sonar my,2000`) is dead. None of these actions has a per-tick loop except `InitRotationObjet` and `InitStartEnnemy`. They are one-shot initialisers that run at spawn.

**Shipped usage (all levels, `*.entities.json`):**

| Action (as spelled in data) | Count | Levels | File(s) |
|---|---|---|---|
| `InitArbre` | 668 | 17 levels (10001, 10004, 10005, 10007, 10008, 101o_j, 103J_j, 104M_j, 113q_t, 172c_j06, 173b_t05, 174a_j07, 175c_t, 176g_t, 178d_t, extro, ...) | herbes1.pcx ×518, tree1b.pcx ×121, tree2.pcx ×29 |
| `initMDL` / `initmdl` | 175 | 10002, 10004, 10007, 10009, 104M_j, 111r_j, 172c_j06, 178d_t, extro, menu_greenpepper | grate1.mdl ×102 (178d_t only), cactus1.mdl ×37, huttop2.mdl ×36 |
| `OnGround` | 5 | 101o_j | arbre1.pcx |
| `initStartEnnemy` | 4 | 180c_t | cliprang.mdl |
| *(no action)* | 107 | 10007, 10008, 111r_j, 113q_t, 114t_t, 173b_t05, 176g_t, 178d_t, 179f_t, menu_greenpepper | barrier1.wmb ×60 (173b_t05), border.wmb ×16 (113q_t), bigrock1-6.wmb ×26, border7.wmb ×1, pyra176.wmb ×1 (176g_t), greenhi1.mdl ×2 (178d_t, 179f_t; editor-passable), solmenu5.mdl ×1 (menu; editor-passable) |
| `Initplanche1`, `InitDynamicLight`, `InitRotationObjet`, `InitRemoveObjet`, `InitBonus5`, `InitMdlBonus`, `InitDeathDalle` | **0** | none | n/a |

Every action name in the shipped data resolves to an `Action` in the scripts (case-insensitively). The one oddity is that **`InitMdl` is defined twice**: `mm1.wdl:2215` (`frame=Skill1; skin=Skill2`) and `mm1.wdl:4817` (the full decoration initialiser). Both definitions sit outside any `IfDef`. WDL identifiers are case-insensitive [U], so which definition the A4 compiler binds is [U]. The data only makes sense with **`:4817`**:
- grate1 has `Skill4=0.5` (scale_z) and scale 2.05, which matches the special case at `:4861` ("horreur pour tester les grilles dans level 178").
- cactus1/huttop2 have `Skill6=1`, `Skill7=22|10|21` (the random frame range).

Port `:4817` and ignore `:2215`.

---

#### `InitArbre` (`:1652-1724`; header comments `:1636-1651`)

Editor parameters (header):
- `Skill2` skin.
- `Skill3` number of skins.
- `Skill4` frame.
- `Skill5` number of frames.
- `Skill6` height offset (×scale_z).
- `Skill7` alpha.
- `Flag1` facing.
- `Flag2` oriented.
- `Flag3` nofilter.
- `Flag4` random X/Y scale.
- `Flag5` random skin.
- `Flag6` random frame. `Flag6 == 0` also gives the thin trunk hull. The header comment says "Flag7==0: small trunk", but the code tests Flag6.

```rust
// once, at spawn
fat = 0; narrow = 0;                                   // :1655-1656
// ground probe (IfDef 422 branch, :1661-1667)
trace_mode = IGNORE_ME | IGNORE_YOU | IGNORE_MODELS | IGNORE_SPRITES | IGNORE_PASSENTS; // no ACTIVATE_SONAR
let hit = trace(origin, origin - vec3(0,0,2000));     // straight down 2000 quants from the editor origin
// `target` = hit point. If nothing within 2000: `target` keeps whatever the last trace left [U].
// Port: keep the editor z on a miss.
let half_h = (max_z - min_z) / 2.0;                    // sprite bbox height / 2 [U: = scaled pixel height / 2]
z = target.z + half_h - 4.0;                           // :1668  (sprite bottom sunk 4 quants)
if skill7 != 0 { ent_alphaset(0, skill7); }            // :1670-1673 (all herbes1 + 2 tree2: 100)
if skill6 != 0 { z = target.z + skill6 * scale_z; }    // :1674-1677 (unused: Skill6 = 0 everywhere)
if flag1 { facing = 1; z = target.z + half_h; }        // :1678-1682  screen-parallel sprite, no -4
if flag2 { oriented = 1; z = target.z + half_h; }      // :1684-1688 (unused)
if flag3 { nofilter = 1; }                             // :1689-1692 (unused)
if flag4 {                                             // :1693-1699
    scale_x *= 1.0 + (random(1.0) - 0.5);              // independent factors in [0.5, 1.5)
    scale_y *= 1.0 + (random(1.0) - 0.5);
    z = target.z + half_h - 4.0;                       // half_h is NOT recomputed after rescale [U]
}
if flag5 { skin = floor(random(skill3)) + 1; }         // :1700-1703 (unused)
if flag6 { frame = floor(random(skill5)) + 2; }        // :1704-1707 (+2: off-by-one, frames 2..skill5+1; unused)
if skill2 != 0 { skin = skill2; }                      // :1709-1712 (unused)
if skill4 != 0 { frame = skill4; }                     // :1713-1716 (unused)
if !flag6 {                                            // :1717-1723  -> ALL shipped InitArbre
    min_x *= 0.3; max_x *= 0.3;
    min_y *= 0.3; max_y *= 0.3;                        // collision box shrunk to 30 % in X/Y; Z untouched
}
// no loop; entity stays non-passable (editor PASSABLE never set on InitArbre)
```

**Shipped data:**
- Skills are all 0 except `Skill7=100` on herbes1 ×518 and tree2 ×2.
- Flags:
  - herbes1: none.
  - tree1b: none ×48, Flag4 ×12, Flag1 ×4, Flag1+Flag4 ×57 (all 57 in 178d_t).
  - tree2: none ×24, Flag1 ×5.
- Flag2/3/5/6 are never set, so **every InitArbre gets the ×0.3 hull**.
- Editor `ambient`: herbes1/tree2 100, tree1b 50. InitArbre leaves ambient alone (`:1654` commented), so the editor value stands.
- Sprite sizes: herbes1 103×122, tree1b 151×156, tree2 256×252 px. Shipped scales are 1-4.

**Crossed sprites:** 580 of the 607 InitArbre with non-zero pan sit in **perpendicular pairs at the same origin**:
- herbes1 at pan 1/90 (259 + 259);
- tree1b/tree2 at pan 5/95 and 4/96.

They form an "X" billboard. In A4 a sprite with non-zero angles is drawn **oriented** (fixed in the world); a sprite with all angles 0 rotates to face the camera [U, standard A4 sprite rule]. Pan-0 sprites (tree1b Flag1+Flag4 in 178d_t, 4 tree2) are camera-facing; with Flag1 (`facing`) they are screen-parallel.

**Collision** [U, engine]:
- A4 `MOVE` collides with non-passable sprites by their bounding box. Sprite bbox ≈ x,y: ±width/2·scale, z: ±height/2·scale [U].
- After `:1717-1723` the X/Y half-extent becomes `0.3·width/2·scale` (e.g. tree2 at scale 3: ±115 quants; herbes1 at 1.5: ±23 quants).
- Sammy's and the lions' ground/gravity traces use `IGNORE_SPRITES` (`player.wdl:64,1217,1578`; `lion.wdl:1234,1291`), so sprites are never floors. Only the horizontal `MOVE` blocks.
- Whether the A4 4.22 `MOVE` really blocks on sprite boxes, including the grass tufts, is [U]. Verify in the original game before enabling it for herbes1.

**Port status:**
- `level.rs:268-310` spawns every PCX as a `Billboard` (Y-axis camera-facing) at the editor origin.
- Missing:
  - ground snap;
  - oriented (crossed) rendering for pan≠0;
  - `facing` for Flag1 (`ScreenAligned` exists but is only used by titles);
  - Flag4 random scale;
  - `Skill7` alpha;
  - collision.
- Today the crossed pairs render as two identical billboards on top of each other.

---

#### `OnGround` (`:3911-3933`)

```rust
trace_mode = IGNORE_ME | IGNORE_YOU | IGNORE_MODELS | IGNORE_SPRITES | IGNORE_PASSENTS | ACTIVATE_SONAR; // :3918
trace(origin, origin - vec3(0,0,2000));                // :3919-3922
z = target.z + (max_z - min_z) * 0.5 - 2.0;            // :3924  (sprite bottom 2 below ground)
if flag2 { nofilter = 1; }                             // :3925-3928
if skill1 != 0 { ent_alphaset(0, skill1); }            // :3929-3932
```

- `ACTIVATE_SONAR` fires the hit entity's sonar event with `you = this sprite` [U]. It is harmless in 101o_j; the port can ignore it.
- **Shipped:** 5 arbre1.pcx (92×128) in 101o_j, scale 4, angles 0.
  - `Skill1=80` (alpha 80).
  - `Skill2=20`, unused by the code.
  - Flag1, which OnGround does not read.
  - Editor PASSABLE: no collision.
- **Port:** spawned as a Billboard at the editor origin. Missing: ground snap and alpha 80 (`ent_alphaset` semantics [U]; `wdl-porting.md` §1 treats it as alpha %).

---

#### `InitMDL` (`:4817-4868`; header `:4806-4816`) — the live definition

```rust
if flag1 { overlay = 1; }            // :4818-4820  palette-index-0 / black texels transparent (colour key)
albedo = 0;                          // :4821
if flag2 { flare = 1; }              // :4822
if flag3 { nofilter = 1; }           // :4823
if flag5 { transparent = 1; }        // :4824-4827
if skill1 != 0 { skin = skill1; }    // :4829
if skill2 != 0 { scale_x = skill2; } // :4830
if skill3 != 0 { scale_y = skill3; } // :4831
if skill4 != 0 { scale_z = skill4; } // :4832
if skill5 != 0 {                     // :4833-4837
    sonar(me, 1000);                 // A4 SONAR: trace down 1000 from origin, sets target [U]
    z = target.z + skill5;
}
if skill6 != 0 {                     // :4838-4849
    frame = if skill7 != 0 { random(skill7 - skill6) + skill6 }   // float in [skill6, skill7)
            else { skill6 };
    // The header comment says "add to skill6", but the code makes skill7 an upper bound.
    // Fractional frames interpolate between frames N and N+1 [U]; the port may floor.
}
if flag4 { min_x -= 16; max_x += 16; waitt(1); }  // :4850-4859 (widen X hull only; unused)
if scale_x == 2.05 && skill4 == 0.5 && averagefps < 8 { invisible = 1; } // :4861-4867 perf hack for 178d_t grates — drop
// no loop; no animation (frame stays fixed)
```

**Shipped data:**

| Model | n | Skills | Flags | Effect |
|---|---|---|---|---|
| cactus1.mdl (21 frames [D, header]) | 37 | `S6=1, S7=22` | none (4 have SHADOW) | random frame in [1,22); **solid** (not editor-passable); scale 2.75-6 |
| huttop2.mdl (2 skins) | 32 | `S6=1, S7=10` | Flag1 + PASSABLE | random frame [1,10), overlay |
| huttop2.mdl (extro) | 4 | `S1=2, S6=1, S7=21` | Flag1 + PASSABLE | skin 2, frame [1,21), overlay |
| grate1.mdl | 102 | `S4=0.5` | Flag1 + PASSABLE | scale_z 0.5 (flattened grid), overlay; 178d_t "big wooden cage" grilles |

- Cactus collision is the model bounding box [U: A4 v4 tests static models against their bbox]. The script does not touch `fat`/`narrow`.
- Ground snap is not used (`Skill5 = 0` everywhere).

**Port status** (`level.rs:244-256`):
- Every MDL gets a `ModelAnimator` that loops scene "stand" at 2 %/tick, uses skin 1 (or `ActionInit.skin`), and has no collider. Only the player and enemies get a `BoxCollider` (`actions/enemy.rs:1375-1395`).
- Missing:
  - a fixed random frame with no animation;
  - `Skill1` skin (extro huttop2 skin 2);
  - `Skill2-4` scale overrides (grate1 scale_z 0.5);
  - `overlay` colour-key transparency (grates, hut roofs);
  - a solid AABB collider for non-passable cactus1.

---

#### `Initplanche1` (`:1725-1733`), unused

`fat = narrow = 0; passable = 1; Sonar my,1000; z = target.z; frame = random(2) + 1`. No shipped entity uses it. The weak-door planks use `Launchplank` (see `wasps.md` §6.2).

#### `InitDynamicLight` (`:3285-3291`), unused

`passable = 1; light rgb = (Skill2, Skill3, Skill4); lightrange = Skill5`. No shipped entity uses it.

---

#### No-action entities (107)

- **WMB** (81: barrier1, border, border7, bigrock1-6, pyra176):
  - All flags are 0 and all skills are 0. They have no script, so they are **static, solid** map entities. A4 collides with a map entity's BSP hulls unless it is PASSABLE [U, but consistent with the doors/platforms behaviour].
  - **Port:** already correct. `level.rs:261` makes a WMB a solid `Brush` unless editor PASSABLE.
- **MDL** (3: greenhi1 ×2, solmenu5 ×1):
  - All are editor-PASSABLE; pure scenery.
  - **Port:** renders them but also loops "stand" at 2 %/tick. It should show frame 1, static [I], since no action means no animation.

---

#### `InitStartEnnemy` / `StartEnnemy` (`:5126-5170`) — refines `enemy-ai.md` §6.5

```rust
// init
invisible = 1; passable = 1; morph("small.pcx");            // :5133-5135  (hide the cliprang.mdl helper)
flag1 = 1; enable_detect = 1; event = StartEnnemy;          // :5136-5138
while flag1 {                                               // :5139-5144
    scan(pos, angle, ScanSphere{pan:360, tilt:360, range: skill2}); // skills.wdl:170; sphere radius Skill2
    waitt(8);                                               // first scan on the spawn tick, then every 8 ticks
}
// EVENT_DETECT, you = each scanned entity that has enable_scan (lions: lion.wdl:33,731,778) [U]
fn StartEnnemy() {                                          // :5146-5169
    if you.IdEnnemy != skill1 { return; }
    you.LionEtat = skill3;
    if you.LionEtat == 1 && you.IdEnnemy == 8888 {
        you.lionAnimStep = 0;
        you.LionAnimSpeed = LionAnimSpeedWalk * you.SpeedEnnemy;
    }
    if skill5 != 0 { you.LionReactRange  = skill5; }
    if skill6 != 0 { you.LionAttackRange = skill6; }
    enable_detect = 0; event = null;                        // one-shot: the rest of this scan's hits are ignored [U]
    if skill4 == 0 { flag1 = 0; return; }                   // loop above exits after its current waitt 8
    waitt(skill4); enable_detect = 1; event = StartEnnemy;  // re-arm
}
```

**Refinements to `enemy-ai.md` §6.5:**
- The event disarms itself on the first match. With `Skill4 = 0` (all shipped), each trigger therefore wakes **exactly one** 8888 entity, the first detected within its sphere, and then dies.
- Detection is a full 3D sphere of radius `Skill2`.

**Shipped (180c_t "mosaic") — 4 triggers, all `Skill1=8888, Skill2=300, Skill3=1, Skill4=0, Skill5=10000`:**

| Trigger at z=1792 | `Skill6` | Lion directly above |
|---|---|---|
| (-6, 1784) | 600 | rockerA (-3, 1806, z≈3632), Δxy≈22 |
| (1602, -16) | 600 | rockerA (1597, -2, z≈3664) |
| (2, -1632) | 500 | nanolion (13, -1578, z≈3640) |
| (-1670, -32) | 500 | nanolion (-1683, 6, z≈3648) |

- Sammy starts at (-1725, -1730, 1737).
- The lions start ~1850 quants above their trigger. Each is out of the 300 radius until it comes down to it [I: they drop from an upper area].
- On waking: walk state 1, react range 10000 (always aware of Sammy), attack range 500/600.

**Port:**
- `initstartennemy` is **not** in `is_marker` and has no `init` arm. The cliprang.mdl helper is therefore rendered visibly (scale 0.3, animated) with no behaviour.
- Needed:
  - `hidden_but_alive`;
  - a sphere test every 8 ticks against `Enemy` with `id == 8888`;
  - a one-shot application of state 1, anim-step reset, and the react/attack overrides.

---

#### Unused interactive objects (0 shipped uses; brief)

- **`InitRotationObjet` / `RotateobjetHit`** (`:3619-3678`):
  - Init: passable, `enable_scan`. Stores the origin in S9-11. Flag2 enables a trigger with `trigger_range = Skill8·scale_x`. `S12 = int(S7)` (hover amplitude), `S13 = frc(S7)` (damping), `S14 = 1`, `S15 = S11 + S12`.
  - Loop every tick:
    - Flag3: `S18 += (1 - S18)·0.1·time`, then `pan/tilt/roll += S2/S3/S4 · S18 · time`.
    - Flag4: `z += (S15 - z)·S13·time`. Within ±8 of S15: `S14 = -S14; S15 = S11 + S12·S14`.
  - Trigger event from an entity with `skill1` ∈ {5555, 3333, 55551, 55552}: `if S18 < S5 { S18 += S6 }`.
  - Scan id 21: `flag1 = 0`, which ends the loop and removes the entity.
- **`InitRemoveObjet` / `RemoveObjet`** (`:3680-3694`): `enable_scan`; on a scan whose `ScanIdentite == Skill2`, `remove me`.
- **`InitBonus5` / `Bonus5_Hit`** (`:2046-2135`) with **`InitMdlBonus` / `MdlBonusScanne` / `AnimMdlBonus`** (`:2142-2213`):
  - InitBonus5 sets `skill1 = 2222`, invisible, `enable_impact`, then scans a 360° sphere of radius `Skill3` with `ScanIdentite = 2222`.
  - Each InitMdlBonus model hit by that scan:
    - sets `frame = its Skill1`, `bonus.entity1 = model`;
    - disables its scan;
    - becomes passable.
  - On impact, unless already busy (`flag1`):
    - `AnimMdlBonus` plays the linked model's frames from Skill2 up to Skill3, then back down, at `Skill4·time` per tick. One run at a time (`model.flag1`).
    - `Points = 100` is passed to `ActionPointsBonus_Syn`, which is `ActionPointsBonus1`, a legacy Ploing scorer (`:1774-1781`).
    - Blue light (80,30,240): `LightRange2`, then −10/tick down to 50, then off.
    - Ding3 at volume 50.
    - Popup `plu100+2.pcx` at z+100 (16 ticks, 2 frames).
    - It waits for the model animation to finish, then re-arms.
  - InitBonus5 is invisible but not passable, so it is an invisible solid block [U].
- **`InitDeathDalle` / `DeathDalle_Hit`** (`:3574-3593`):
  - `enable_sonar`.
  - On sonar from an entity whose `IdEnnemy` ∈ {S2, S3, S4}, or any entity if Flag1, and `you.z - my.z ≤ Skill6`: `you.flag1 = 0` (stops its loop) and `you.LionEtat = Skill5`. Per the header comment, 1001 means vanish in smoke and 1002 means explode.
  - Already summarised in `enemy-ai.md` §5.4.

None of these needs porting for the shipped levels.

### 2.14 Extro level

`extro.wmb` (slot 49). Line numbers are against `extracted/*.wdl`. Timers are ticks (16 Hz, `time = 1`). JSON flags bit0 = Flag1.

#### Corrections to `gameplay.md` §1.7

1. **`SammyExtro` / `InitSammyExtro` (`Extro.wdl:163-402`) are dead code.** No entity uses `InitSammyExtro`, and nothing calls it (grep over all scripts). The extro Sammy is a normal `InitMarche` entity (`suricat1.MDL`, scale 1.85, skills `[0,30,0,0,100,0,0,0]`, flags 28). It runs the regular `Marche` controller (`player.wdl:1-130`). Sammy is kept passive by riding a moving platform and by `InitSetAnimPlayer` triggers that set `PlayerEtat` 3 or 2 (see below). Jump, fire and walking stay possible until the first trigger fires.
2. The extro **has its own main loop** (`InitExtro`, `Extro.wdl:2-153`). It replaces the `DebutPartie` loop, so:
   - there is **no lose check**;
   - there is no win/lose screen, `ValideLevel` or best-score save;
   - the loop exits only on `PartieGagnee != 0`.
3. Credit titles are triggered by **any id-25 scanner**: Sammy's door scan, or wasps and lions with Flag5. They are not tied to Sammy only (`Intro2.wdl:547-556`). The port's `title.rs` uses Sammy's distance only.

#### Flow (`InitExtro`, `Extro.wdl:2-153`)

The entry points are:
- the win progression (`mm1.wdl:1102-1106`), when both `NumberLastJumpLevel` and `NumberLastThinkLevel` are completed and the level just won is one of them;
- the cheat `RunExtro` (`mm1.wdl:3947-3953`, E key binding `mm1.wdl:111`, needs Shift+X held), which sets `PartieGagnee=1`, waits 8 ticks and branches `InitExtro`.

```rust
fn init_extro() {                                   // Extro.wdl:2
    reset_panels();                                 // :4
    typejeu = 25; protection1 = 0;                  // :7-8
    pan_black.visible = true;                       // :9
    camera.fog = 100; camera.ambient = -100; sun_light = -100; fog_color = 4;   // :11-14 (black)
    extro_vue = 1;                                  // :15 (saved later with BestSco0; unlocks Bonus menu)
    wait_ticks(4);                                  // :16
    level_en_cours = 49;                            // :17  (InitMarche reads Player1RestartX[49], normally 0)
    init_level_global();                            // :18
    load_level("Extro.WMB"); wait_ticks(TEMPO_LOAD_LEVEL);   // :19-21
    after_load_init_level();                        // :23  (camera init, same as normal levels)
    wait_ticks(8); pan_black.visible = false;       // :25-26
    if camera_type == 3 {                           // :28. Extro InitCamera1 flags 3 → type 3
        // Level-start sweep, identical in shape to DebutPartie's:
        camera_handle(); camerabouge = 0; wait_ticks(1); camerabouge = 1;          // :30-34
        let mut v = |cameradelta|_1;  tempo = 128.0;  adjust_fps_cam3 = 0.1;     // :35-37
        clip_range = clip_range_start;
        if camera3_friction_start == 0 { camera3_friction_start = camera3_friction_org; }
        camera3_friction = camera3_vert_friction = camera3_friction_start;      // :38-42
        while v > 4.0 && tempo > 0.0 && !key_enter {                            // :43
            camera.fog = max(camera.fog - 10.0*time, fog_level_intensity);      // :45-49
            tempo -= time; update_average_fps(window = 32);                     // :50-59
            wait_ticks(1); camera_handle(); v = |cameradelta|_1;                // :61-63
        }
        camera3_friction = camera3_friction_org; camera3_vert_friction = camera3_vert_friction_org; // :65-66
    }
    clip_range = clip_range_game;                   // :69
    inv_dir_camera = (camera.pan - 180, -camera.tilt); camerabouge = 0;          // :71-73
    camera.fog = fog_level_intensity; fog_color = fog_level_number;             // :78-79
    sun_light = 100; sun_angle = 90;                // :80-81
    mip_flat/shaded = level values;                 // :83-84
    partie_encours = 1; partie_gagnee = 0;          // :86-87  ← releases InitMarche / platform engines / lions
    nbr_nextraball = 0; floor_range = 128;          // :89-90
    ticks1 = 0; son_finis = 0;                      // :92-93
    adjust_fps_cam3 = if avg_fps < 12 {1.0} else if avg_fps < 20 {0.8} else {0.5}; // :97-105
    ticks1 = 0; balles_joueur1 = 1;                 // :111-112
    on_esc = InitExtroOptions;                      // :113 → options.wdl:80 → InitGameOptions
    level_en_cours = 20;                            // :114 ("Play" resumes level 20, never the extro)
    menu_options_game = 2;                          // :116 (options overlay without the Restart button, options.wdl:127-135)
    while partie_gagnee == 0 {                      // :121, once per FRAME (`wait 1`, :145)
        ticks1 += 1; ticks2 += time;
        update_average_fps(window = 64);  // when the window rolls over, ticks1 = 0 (:129-136)
        min_time = min(time, 1.0); adjust_ang_rot_cam1 = 4.0 / average_fps;      // :137-138
        my = pov_syn; camera_handle();              // :142-143  camera follows pov_Syn (retargeted by InitSetPovSyn)
    }
    fog_color = 1; camera.fog = 100;                // :147-148
    partie_encours = 0; wait_ticks(4);              // :149-150 (stops every entity loop)
    branch MainMenu;                                // :152 (no win screen; port: next_slot(49) currently returns 1)
}
```

- There is no `PlayCurrentmusique` call, so the previous music keeps playing [I].
- `PanBlack` is still visible (skin 1) from `SetFinExtro` when the menu loads [I].

#### `StartExtro` (`Extro.wdl:155-159`) and `InitWaspExtro` (`bullets.wdl:1849-1937`)

```rust
fn start_extro() {            // entity: wasp1.MDL at (2143,360,1184), skills [6,12,50,12.07,1.13,10000,0,10], flags 156 (F3 F4 F5 F8)
    wait_ticks(2);
    pov_syn = me;             // :157 → overrides InitMarche's pov_Syn=Sammy (player.wdl:31); camera opens on this wasp
    branch InitWaspExtro;     // :158
}
```

`InitWaspExtro` is `InitWaspEnnemy` with three differences: no `waitt`/`PartieEncours` gate, `LionEtat = 1` (walking) instead of 0, and a forced species. The wasp-side summary is in `wasps.md` §10. Details missing there:

- `IdEnnemy = 6` (`:1853`) **before** `initSkinScaleWaspHive2`. So: `IdWaspSpec = 6`, `Skill7 = 6` → skin 6, scale 1, `WaspPoison (Skill43) = 5`, and `Skill3 (LionAttackRange) = 5` (`bullets.wdl:1411-1478`, lines `+66/+67`). The function also sets `passable = 1` (`+69`), and `Skill6 = 1` if it was 0 (WED 10000 here).
- `IdEnnemySpec (= IdWaspSpec, both Skill47; mm1.wdl:420-421) = 106` (`:1855`), then **overwritten to 101** (`:1878`). Resistance = 18 (frc(6)=0 → default, `:1872-1882`), `WeightForce` 0.3.
- Patrol (Flag3): `LionIdPath = 12`, `LionRangePath = int(frc(12.07f32)*10000+0.5) = 700` (`:1860-1864`). Path 12 = 2 `setpath.wmb` + 9 `spherang.MDL` nodes.
- Trails use `IdWaspSpec == 101` (`bullets.wdl:1740-1750`): `particle_WaspRed` every v1 ticks and `particle_WaspYellow` every v2 ticks. v1/v2 are 2/4 when `AverageFps > 40`, 3/5 when `> 24`, else 4/8 (`:1728-1738`).
- Flag5 → scan id 25, radius `RangeScanDoor` (500), every tick (`bullets.wdl:1718-1726`). This drives `SetCamera` zones and credit titles as the wasp flies.

#### `InitWaspEnnemy` in the extro (`bullets.wdl:1502-1676`, loop `WaspEnnemyFly :1680-1846`)

Facts the port needs that `wasps.md` §10 doesn't spell out:

- **Field clobbering** by `initSkinScaleWaspHive2` (`bullets.wdl:1411-1478`):
  - `Skill3 := WaspPoison`, so the effective `LionAttackRange` is 1..5. The WED 50 is lost, and the "0 → 500" default at `:1508` never fires.
  - `Skill7 := type` for types 2-7.
  - Skin = `Skill7`. For type 0/1 `Skill7` is not assigned, so skin = WED Skill7 = 0 [U: A4 skin 0 = first skin].
  - Scale: types 3, 4 and 7 → 0.8; types 2, 5 and 6 → 1. Type 0/1 keeps its WED scale.
  - `passable = 1`, so these wasps never block Sammy.
- After init, `LionEtat = 0` (standby, `:1656`) after waiting for `PartieEncours == 1` (`:1648-1651`). They wake when `dist(pov_Syn) < LionReactRange` (`enemy-ai.md` §2.2, standby branch). With Skill6 = 10000 (20000 for the "Win" model) this is immediate [I].
- **All distance and LOS tests use `pov_Syn`** (`lion.wdl:948-950`), which the extro retargets. Flag4 is set on every extro wasp, so they never attack.
- Ambient buzz: `random(100) > 99.5` per tick → v = int(random(6))+1. v 1, 2 or 3 → `WaspKaputt_son`, `WaspKaputt2_son` or `WaspKaputt3_son` at volume 100. v ≥ 4 → `TakeWasp_son` at volume 200 (`:1789-1808`).
- **"You Win" model** (`Win.MDL`, skills `[99,9,0,0,1.25,20000,0,16]`, flags 136 = F4+F8):
  - Init leaves `IdEnnemy = 99`, because no type branch matches. That gives spec 99, `WeightForce` 0, resistance 0, and no trail.
  - Each tick (`:1810-1827`): `set_cycle(LionStand_str, PlayerAnimStep)`; `PlayerAnimStep += 6*time`, wrapping to 0 above 100. A `Particle_Win` burst comes from vertex `int(random(172))`. `random(100) > 99.5` → `Glisse_son` at 100.
  - It is not 8888, so `InitSetPovSyn` triggers and the player's wasps ignore it.
  - Being in state 1 with Flag3=0 and Skill4=0, it would creep forward at `SpeedEnnemy = 9` when facing (`enemy-ai.md` §2.3) [I. It sits at (14434,-4641,1648), pan 180].
- **Unpacking traps** (f32): Skill2 `29.5` → `BounceForce = int(0.5*100+0.5) = 50` (wasps at x 5395/5103/4623/4315/4755/4259). Integer Skill2 → `LionForce`. Skill5 `1.13` → `LionAccelAttack = frc(1.13f32)*10 = 1.2999999`, `LionNbrVie = 1`.

| Group | Count | Type (Skill1) | Speed (Skill2) | Path (Skill4 → id/range) | Flags | Notes |
|---|---|---|---|---|---|---|
| swarm at (4259..6079, −5496..−4724, z≈3244-3588) | 13 | 3 or 4 | 28-30 | 14.1 → 14 / 1000 | 140 (F3 F4 F8); one has 156 (+F5) | path 14 = 1 cliprang + 19 spheran2 nodes |
| near start (1803..2655, 16..336, z 832) | 5 | 6, 1, 2, 2, 2 | 15 | 13.1 → 13 / 1000 | 140 | path 13 = 9 cliprang nodes |
| `StartExtro` | 1 | (forced 6 → spec 101) | 12 | 12.07 → 12 / 700 | 156 | camera opens here |
| `Win.MDL` | 1 | 99 | 9 | — | 136 | "You Win" sparkle |

Path 18 (1 cliprang + 21 spheran2 nodes) is not referenced by any extro enemy [D].

#### `InitSetPovSyn` / `SetPovSyn` (`mm1.wdl:5395-5486`)

Fields:
- Skill1: id to match against `you.IdEnnemy`. This is Skill1 of the target, so Sammy matches 3333 (`IdPlayer` is the same slot, `mm1.wdl:203/312`).
- Skill2: scan radius.
- Skill3: required `IdEnnemySpec` (0 = any).
- Skill4: re-arm delay. 0 = one-shot; 999 = stay armed with no disarm.
- Skill5: number of uses.
- Skill6: forced `CameraBouge` while active.
- Skill9: saved `CameraBouge`.

```rust
fn init_set_pov_syn() {                           // :5404
    invisible = true; passable = true; morph("small.pcx"); flag1 = true;
    enable_detect = true; event = SetPovSyn;
    while flag1 { scan_sphere(pos, radius: skill2, id: <current ScanIdentite, normally 0>); wait_ticks(2); } // :5411-5416
    remove(me);
}
fn set_pov_syn(you) {                             // :5419, runs once per detected entity [U: A4 EVENT_DETECT order]
    if you.id_ennemy != skill1 { return; }
    if skill3 != 0 && you.id_ennemy_spec != skill3 { return; }
    pov_syn = you;                                                    // :5429 (camera AND all lion/wasp distance tests retarget)
    let off = rotate((persodistcam, 0, 0), pan: PersoAngArriere, tilt: 0, roll: 0); // :5431-5437. World-fixed yaw, NOT you.pan
    let p = (you.x + off.x, you.y + off.y, you.z + HautCamPerso + CameraSolPlus);   // :5439-5441
    camera_syn.pos = camera2_syn.pos = camera.pos = p;                // hard cut (:5446-5455)
    camera_target_z = you.z + Camera_PlusTilt;
    (camera.pan, camera.tilt) = to_angle((you.x, you.y, camera_target_z) - camera.pos); // :5457-5462
    if skill6 != 0 { skill9 = camerabouge; camerabouge = skill6; }    // :5463-5467
    if skill4 == 999 { return; }                                      // stays armed
    enable_detect = false; event = None;
    if skill4 == 0 { flag1 = false; return; }                         // one-shot → loop ends → removed
    wait_ticks(skill4); if skill9 != 0 { camerabouge = skill9; }
    skill5 -= 1; if skill5 == 0 { flag1 = false; return; }
    enable_detect = true; event = SetPovSyn;
}
```

Extro instances (all one-shot, Skill4 = 0, Skill6 = 0):

| Pos | Skill1 | Radius |
|---|---|---|
| (11752,−5147,2216) | 8888 | 1000 |
| (20287,−1423,264) | 8888 | 700 |
| (2620,−5244,40), (7696,−5576,40), (2704,−1964,40) | 8888 | 300 |
| (−4700,−4664,716), (472,−4728,588) | 3333 (Sammy) | 500 |

- The 3333 trigger at x −4700 lies on Sammy's wagon route. Sammy starts at x −5856, so it fires after about (5856−500−4700)/8 ≈ 82 ticks [I] and pulls the camera back from the `StartExtro` wasp to Sammy.
- The 8888 triggers hand the camera to whichever lion or wasp passes first [I: emergent, depends on patrol timing].
- Port: `pov_Syn` must be a resource (`PovTarget(Entity)`), read by the camera and by lion/wasp `test_dist_lion_player`/LOS instead of "the player".

#### `InitSetAnimPlayer` / `SetAnimPlayer` (`mm1.wdl:5488-5529`)

Fields:
- Skill1: `IdPlayer` to match.
- Skill2: radius.
- Skill3: `PlayerAnimEtat`.
- Skill4: re-arm delay (0 = one-shot).
- Skill5: anim speed, **also used as the use counter** (bug).
- Skill6: anim step.
- Skill7: `PlayerEtat`.

```rust
fn init_set_anim_player() { /* same as init_set_pov_syn, event SetAnimPlayer, scan every 2 ticks (:5497-5511) */ }
fn set_anim_player(you) {                          // :5512
    if you.id_player != skill1 { return; }
    enable_detect = false; event = None;
    you.player_anim_etat = skill3; you.player_anim_speed = skill5;
    you.player_anim_step = skill6; you.player_etat = skill7;           // :5517-5520
    if skill4 == 0 { flag1 = false; return; }
    wait_ticks(skill4); skill5 -= 1; if skill5 == 0 { flag1 = false; return; }
    enable_detect = true; event = SetPovSyn;       // :5527 BUG: re-arms with the wrong handler (moot: Skill4 = 0 in data)
}
```

Extro instances, all Skill1 = 3333, radius 250, Skill4 = 0, Skill6 = 0:

| Pos | Skill3 anim | Skill5 speed | Skill7 PlayerEtat | Effect |
|---|---|---|---|---|
| (−5388,−4648,572) | 11 `EtatAnimLoopjambes` | 3 | 3 | legs-loop cycle (`anims.wdl:477-483`, wraps at 100) |
| (2396,−4700,572) | 12 `EtatAnimSalut` | 2 | 2 | wave once, then loop-legs at speed 2 (`anims.wdl:485-496`) |
| (5448,−4652,572) | 12 `EtatAnimSalut` | 3 | 3 | same |

Effects of the forced `PlayerEtat`:
- **3** ("Kaputt" state):
  - `HandleSpeedPlayer` only applies input rotation and walking for `PlayerEtat` 1 or 4 (`player.wdl:1119-1170`), so Sammy can no longer steer or walk.
  - Jump and fire need `PlayerEtat == 1` (`player.wdl:281, 396`).
  - `TeleportFor1Life`/`Player_Kaputt` return immediately while it is 3 (`player.wdl:701, 775`), so **Sammy can't die**.
  - Gravity and the platform carry still apply [I].
- **2** ("firing"): also blocks input. Nothing resets it to 1 except the tail of `PlayerTire` (`player.wdl:380-382`), which isn't running [I].

`AnimPlayer` must support anim states 11 and 12 (`Playerloopjambes_str`, `PlayerSalut_str`; `PlayerAnimSpeedLoopjambes = 2`, `PlayerAnimSpeedSalut = 3`, `mm1.wdl:52-53`).

#### `InitSetFinExtro` / `SetFinExtro` (`mm1.wdl:5531-5562`)

```rust
fn init_set_fin_extro() { /* as above, event SetFinExtro, scan every 4 ticks (:5538-5543) */ }
fn set_fin_extro(you) {                            // :5546
    if you.id_player != skill1 { return; }         // 3333
    enable_detect = false; event = None;
    fog_color = 1; camera.fog = 1.0;
    while camera.fog < 100.0 { camera.fog += 6.0 * time; wait_ticks(1); }   // 17 ticks
    pan_black.visible = true; pan_black.skin = 1;
    partie_gagnee = 1;                             // → InitExtro loop exits → MainMenu
}
```

- Instance: (8560,−4652,572), radius 250.
- From Sammy's start at x −5856 on a wagon moving at 8/tick, it is reached after about (8560−250+5856)/8 ≈ 1771 ticks ≈ 111 s [I]. That assumes the engine is never blocked [U: geometry], and is ×3 while Shift-boosting with boost charge (`mm1.wdl` engine loop; boost starts empty).

#### The ride: platform engine 7788

- Engine `InitEnginePlatform` at (−4951,−3276,−2079), angles 0, skills `[7788, 8, 0, 0, 1, 0, 0, 0]`, flags 0:
  - linked by scan radius 50000/10000;
  - unlocked (Skill6 = 0), so it starts as soon as `PartieEncours = 1`;
  - moves along +X at 8/tick;
  - on impact: pause 0 ticks, tilt +1° (Skill5).
- Bodies with Skill1 7788: `platfor1.wmb` at (−5912,−4656,740) and `wagongem.wmb` at (−6168,−4656,740), pan 90.
- **37 `InitKey` gems sit in the wagon** at (−6260..−6060, −4728..−4592, z 772-820):
  - flags 80 = **Flag5 + Flag7**: gravity/sonar, carried by platforms via `PlayerForce`;
  - Skill6 = 3 (orange) for all of them, whatever their sprite file (gemgr/gemre/gembl/gemma/gemve/gemye/gemmu/gemor). The sprite is not re-chosen from Skill6 (`gems.wdl:38-63`, `InitNewKey :193-204`);
  - without platform-carried gems (`gems.wdl:105-130`) they stay behind at the start.
- A second engine (7899) at (−1652,808,−858), skills `[7899,10,40,0,180,1,0,0]`, flags 64 (F7):
  - red-locked (Skill6 = 1);
  - **no bodies have id 7899**, so it does nothing visible [D].

#### Other extro entities (behaviour specced elsewhere)

| Action | Count | Data | Note |
|---|---|---|---|
| `InitGame` | 1 | `[25,−64,−524,0,3000,0,2220,0]` F1 | goal 2220 (no diamonds in the extro, so never reached; key gems therefore never vanish) |
| `InitGame2` | 1 | `[1,3200,400,1000,90,−45,0,0]` | 1 life; start camera (3200,400,1000) pan 90 tilt −45 |
| `InitGame3/4/5/7`, `InitCamera1/2/3`, `Initpublic`, `Init(N)ExtraBalles` | 1 each | InitCamera1 `[500,−50,1500,200,32,1000,200,16]` flags 3 (type 3); InitGame4 `[0.5,0.5,20000,…]`, Skill8 = 0, so `TitlesOn` stays 1 (`mm1.wdl:1904`) | markers |
| `InitLion` | 9 | rocker `[13,10,600,28.12,1.25,10000,0,16]` F3+F5; hippie `[12,12,2500,29.1,…]` F3+F5; punk `[14,12,3000,17.08,…]` ×1.5 F3+F5; 4 nano + 2 rasta `[10|10,8|6,60,0,1.25,10000,0,4]` F4 | rocker/hippie/punk patrol paths 28/29/17 **without Flag4, so they can attack** (but Sammy is immune once `PlayerEtat = 3`). Their Flag5 drives camera zones and titles. The nano and rasta lions (F4, no path) never attack [I: walk straight once awake]. |
| `SetCamera` | 34 | all Skill1 = 3 (type 3), Skill8 = 0 | zones triggered by scan 25 from Sammy and from Flag5 lions/wasps |
| `InitTitle` | 6 | credits `agameby/jerome/Jorg/remi/theend.pcx` `[n,215,0,±50,8,16.3..256.3,8,95]` flags 16; `titre.MDL` flags 32 | trigger on scan 25 from **any** scanner (`Intro2.wdl:547`), so titles show as the camera wasp or lions fly by |
| `InitBoulder` | 8 | 2× `boulder5.wmb` `[96,0,0,30,9999,1,0.15,0]` F3; 6× invisible `small.pcx` emitters `[16/64/80,0,0,20/30,0,999,20-30,48/64]` flags 526 (F2 F3 F4 + bit 9) | the emitters hang above the wagon track (x −1199..−3423, z 1029-1781) and rain random-colour (999) short-lived gems (Skill1 < 1000 = lifetime). See the boulder spec. |
| `InitKey` (others) | 2 | gemye `[…,2]` scale 1.5 at (9580,−852,1072); gemmu `[…,255]` scale 1.75 at (9208,3212,216) | off-route; no doors in the extro |
| `InitBoost` | 1 | `[0,50,128,…]` at (3357,−4738,236) | about 500 below the track [I unreachable] |
| `InitBonusVie` | 1 | `[0,1,300,…]` F3 at (12220,−4416,−690) | decorative in practice |
| `InitMDL` | 15 | 11× cactus1 `[0,0,0,0,0,1,22,0]`; 4× huttop2 `[2,…,1,21,0]` flags 513 | decoration spec |
| `InitArbre` | 46 | herbes1.pcx `[…,100(Skill7),…]` | decoration spec |
| `InitSkyMenu` | 2 | sky5 / solmenu5 | implemented |
| `InitPath` | 121 | ids 12, 13, 14, 17, 18, 28, 29 | implemented |

#### Port checklist (extro)

**Status (port):** items 1-7 done in `actions/extro.rs` (Pov, Scanners, triggers, fade), `enemy::init_wasp_ennemy` (Flag8 flight, passable, Win model) and `flow.rs` (`EXTRO_SLOT`: no lose check; the end loads slot 1 until the menu exists). Not done: wasp trails, `Particle_Win` sparkle, ambient buzz sounds.

1. Add `PovTarget` and use it in the camera and in enemy distance/LOS.
2. Implement `InitSetPovSyn` (hard camera cut), `InitSetAnimPlayer` (forced anim 11/12 and `PlayerEtat` 2/3 lock and immunity) and `InitSetFinExtro` (fog fade → menu).
3. `StartExtro` → `InitWaspExtro`.
4. `InitWaspEnnemy`: lion AI with the wasp skin, flying, `passable`, the "You Win" variant, and the field clobbering above.
5. Make platforms carry Flag7 gems (the wagon).
6. Let Flag5 enemies' scan 25 trigger `SetCamera` zones and `InitTitle`.
7. Give the extro its own main loop: no lose check, no win screen. On end, go to the menu (or the title flow); do not advance to slot 1.

### 2.15 Menu-level actions

These are used only in `menu_greenpepper.wmb`, which is both the 3D main menu and the intro scene. None of them is gameplay-relevant. The port has **no menu**: `flow.rs:64-74` chains levels and goes 49 → 1, and there is no `menu` module. The menu itself is specified in `gameplay.md` §1.2-1.3.

| Action | Count | Source | What it does |
|---|---|---|---|
| `InitMainMenu` | 59 | `interfac.wdl:23-199` | Menu items and level icons (Skill1 = menu code, table in `gameplay.md` §1.3). Flag1 invisible; Flag3 alpha 80; Flag4 flare. Skill7 = hover amplitude (int) / damping (frc). Hidden: Skill1 = 50 (Bonus) unless `BonusActif == 2` or (`== 1 && ExtroVue`); Skill1 = 1 (Play) while `NeverPlayed == 0`. Skill1 ≥ 100 → level icon state from `LevelsInfo[Skill1−100]`. Event `ObjetMenuSelect` (touch/click/release). |
| `InitMainTitle` | 2 | `interfac.wdl:950-~1000` | Spinning, hovering title models (`titjump/titjum2.wmb`, Skill1 = 3 → "Jump" item). Each tick: `pan += Skill2*time`; `tilt += cos(pan)*Skill3*time`; `z += (Skill15−z)*Skill13*time`. Clickable like `InitMainMenu`. |
| `PosTutorialLevels` / `PosJumpLevels` / `PosThinkLevels` / `PosBonusLevels` | 1 each | `interfac.wdl:903-923` | Invisible, passable camera markers (`posTutLevels_Syn` etc.; the tutorial one is also the default `posChooseLevels_Syn`). The menu cuts the camera to them. Skill2/3 = 45/135 pan limits for `PanCameraChoose`, which is dead (`End;` first line, `:929`). |
| `InitPlayerPatrol` | 1 | `Intro2.wdl:231-329` | Intro Sammy (`suricat1`, skills `[0,15,0,0,600,…]`, flags 28): sets `pov_Syn`, scans id 30 (cone pan 45 / tilt 90, range 300) for the first path node, then waits for `EtapesIntro != 0` and runs `MoveSammyOnPath` (`:412`), an auto-walk that also scans id 25 (radius 300) for camera zones. |
| `InitPathPlayer` | 45 | `Intro2.wdl:392-411` | Intro path nodes (`setpath.wmb`, morphed to `small.pcx`, invisible). On scan 30 they set Sammy's `PlayerTargetX/Y`, `PlayerTargetPan = pan`, `PlayerPatrolEtat = 1`, and `PlayerKey = Skill4`. That value drives the intro step machine (`EtapesIntro = Pov_Syn.PlayerKey`, `Intro2.wdl:107-115`). |

---

## 3. Prioritized implementation order

Ranked by "can the shipped levels be finished, and do they play like the original", in progression order (tutorial 1-12, jump 17-28, think 33-44, then the extro). Level names as in `gameplay.md` §3.1.

| # | Item | Spec | Unblocks / affects | Why this rank |
|---|---|---|---|---|
| 1 | **Boulder gem emitters** (`InitBoulder`, `BoulderGem_Hit`, `EmitGems`, `EjectNewKey`, `LaunchNewKey`, lifetime blink/explode) | §2.4, §2.2 | **112e_j** (lvl 20: no `InitKey` at all; green/orange/magenta doors need the random-colour rain), **174a_j07** (lvl 22 "red-gem fall": the only red source for its red door and red-locked platform), **178d_t** (lvl 41: 4 `dfixred` + `donered`, no red key; shoot the boulder), **176g_t** (lvl 42: red doors and lock, one red key), extro rain | Without it these levels cannot be completed except by reaching the score goal |
| 2 | **Surprise key gems** (Skill5 roll, respawn re-roll) and **Flag7 gem gravity** | §2.2 | **180c_t** (lvl 36 "mosaic": 5 gems have colour 0 and are useless without the roll; 9 Flag7 gems) | Blocks one think level; small change |
| 3 | **Permanent-door gem flight** (`CreateGemKeyAnim`/`AnimGemKey`, recall when walking away, validation by walking under, `doordark` morph) and the **`PlayerOldKey` fix** (pickup blocked only during a door flight) | §2.5, §2.3 | 117 doors in 29 levels (all three worlds) | Today the key is spent the instant a `dfix` door starts opening and the door stays up for ever; in the original walking away returns the key and closes the door, which think-level puzzles rely on [I] |
| 4 | **Thrown-back key physics** (`LaunchYouKey`: forward, 2× Sammy's Skill2, +32 ceiling, 5th-bounce rule) and **death gem** (decorative) | §2.3, §2.4 | every level with mixable keys | Where the old key lands decides whether it can be re-taken; cheap once #1's launcher exists |
| 5 | **Per-level slope and turn values** (`InitGame5`) and bounce values (`InitGame4` Skill5/6) | §2.9, §2.8 | **179f_t** (lvl 28, ice slides: ForcePente 150, AccPente 2.5, SlopeAngleMin 0.98, BigSlope 3), 30 levels with SlopeAngleMin 0.9, turn speed in 34 levels | Wiring only; 179f_t is the end of the jump world |
| 6 | **Platform unlock morph and riders**: `platfor1.wmb` morph + light, no sound with master key, `PartieEncours` gate, Flag7 gems and lions carried, side IMPACT push-back | §2.6, §2.10 | 15 levels with locked platforms; 17 levels with lions and platforms; extro wagon gems | Visual cue for "unlocked"; lions/gems falling off moving platforms changes play |
| 7 | **Camera type 2** (`InitCamera1` Flag4, `SetPosCamera`, no start sweep) | §2.12 | 10001-10007, 102N_t, 104M_j, 106g_t, 108i_j (11 levels, the whole first half of the tutorial) | First thing a player sees; type 3 in these levels frames rooms differently |
| 8 | **`SetCamera` zones** (trigger, 1-tick confirm, Skill6+1-step tween, snap, 4-tick friction cap, type switch, respawn restore) | §2.12 | 21 levels (741 zones; 178d_t 122, 176g_t 103, 179f_t 85, 175c_t 78) | Hand-placed camera framing for tight rooms; levels are playable without it but awkward |
| 9 | **Player knockback from non-lion entities** + **decor collision** (InitArbre ×0.3 trunk hull, solid `cactus1`) | §2.8, §2.13 | 17 levels with trees, 10 with `InitMDL` (solid `cactus1` in 10002, 10004, 10009, extro) | Sammy currently walks through trees and cacti and never bounces off doors [U: verify sprite collision in the original] |
| 10 | **Lions on teleport pads** (Flag2) and **lions blocking door closing** (`DoorScanEntities`) | §2.10, `enemy-ai.md` §6.4 | 171c_t04, 173b_t05, 178d_t, 179f_t; all levels with lions and timed doors | Lions follow Sammy through pads in four levels |
| 11 | **Door bells / blink** (Skill8 > 48), `Bell2`, exit-door double flash | §2.5 | 170b_t00, 171c_t04, 173b_t05, 180c_t | Audible "door closing" warning on long timed doors |
| 12 | **Hives** | `wasps.md` §8 | 176g_t, 177e_j | Extra ammo in two levels |
| 13 | **`InitStartEnnemy`** | §2.13 | 180c_t | Wakes four lions; without it they stay idle (and the helper model is visible) |
| 14 | **Decoration visuals**: InitArbre ground snap, crossed sprites, Flag4 scale, alpha; InitMDL fixed random frame, skin, scale, overlay; OnGround; static no-action MDLs | §2.13 | 17 + 10 levels; 101o_j | Cosmetic |
| 15 | **Extro**: `PovTarget`, `StartExtro`/`InitWaspExtro`, `InitWaspEnnemy`, `InitSetPovSyn`, `InitSetAnimPlayer` (anim 11/12, lock, immunity), `InitSetFinExtro`, extro main loop, scan-25 titles/zones from any scanner | §2.14 | extro | Needed to "finish" the game; depends on #6 (wagon) |
| 16 | **Menu** (`InitMainMenu`, `InitMainTitle`, `Pos*Levels`, intro `InitPlayerPatrol`/`InitPathPlayer`) | §2.15, `gameplay.md` §1.2-1.3 | menu | Not gameplay |

Not worth porting for the shipped data: `InitTrampoline`, `InitBodyPlatForm`, `InitDeathDalle`, `InitRotationObjet`, `InitRemoveObjet`, `InitBonus5`/`InitMdlBonus`, `Initplanche1`, `InitDynamicLight`, gem crushing, camera type 1, `CameraSurveillance`.
