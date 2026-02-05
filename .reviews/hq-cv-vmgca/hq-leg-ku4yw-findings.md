# Wiring Review

## Summary

This review examines the codebase for dependencies, configs, or code that was added but never properly integrated. The three Cargo.toml dependencies (serde, serde_json, rand) are all heavily used across the codebase. However, there are several wiring gaps: two CLI commands are defined but never registered in the command registry, a data file exists on disk but is never loaded (with its values hardcoded instead), the `ModuleDef.sections` field is parsed but never consumed by any production code, and the entire `npc_party` module is declared and implemented but never referenced outside its own file.

## Critical Issues
(P0 - Must fix before merge)

None.

## Major Issues
(P1 - Should fix before merge)

- **`SpawnEncounterCommand` defined but not registered in CLI** (`src/command/gm_cmds.rs:13-78`, `src/main.rs:141-149`): The `SpawnEncounterCommand` struct implements `Command` with name `"spawn_encounter"` but is never imported in `main.rs` and never passed to `registry.register()`. CLI users cannot access this command. The GM API (`src/gmapi/interface.rs:35,275`) has its own parallel `spawn_encounter()` implementation that IS wired, so the JSON protocol path works. But the CLI command is dead code. Either register it (add to the `use` import at main.rs:20 and add `registry.register(Box::new(SpawnEncounterCommand))`) or delete it if it's only needed via the GM API.

- **`RollReactionCommand` is an unwired duplicate of `ReactionCommand`** (`src/command/gm_cmds.rs:102-123` vs `src/command/encounter_cmds.rs:119-140`): `RollReactionCommand` (name: `"roll_reaction"`) is functionally identical to the already-registered `ReactionCommand` (name: `"reaction"`). Both call `encounter_engine::reaction_roll(cha)` with the same formatting. `RollReactionCommand` is never imported or registered. The GM API again has its own `roll_reaction()` at `interface.rs:74,775`. This is dead code that should be deleted to avoid confusion about which is canonical.

## Minor Issues
(P2 - Nice to fix)

- **`data/core/gems_jewellery.json` exists but is never loaded** (`data/core/gems_jewellery.json`, `src/rules/treasure.rs:253-258`): A 27-line JSON data file defines the gem value table and jewellery formula, but the code hardcodes identical values as `const GEM_VALUE_TABLE` at `treasure.rs:253` and inline arithmetic at `treasure.rs:289-293`. Every other core data file (`equipment.json`, `magic_items.json`, `monsters.json`, `spells.json`, `treasure.json`) is loaded at runtime via `fs::read_to_string()`. This file appears to have been authored as part of the same data-driven pattern but was never wired in. Either load from the JSON file for consistency or delete the data file to avoid the impression of incomplete migration.

- **`ModuleDef.sections` parsed but never consumed** (`src/rules/module.rs:17-20`): The `sections: HashMap<String, String>` field was added in commit `b2ccabb` to "extract non-room content" like introduction, background, and hooks. The field is correctly deserialized from module JSON, but no production code reads it. Only test code at `module.rs:416-419` accesses `module.sections`. This is loaded data that goes nowhere — the sections are silently discarded after parsing. If sections are intended for future display (e.g., a `module_info` command), this is a partially wired feature.

- **`npc_party` module declared but never used outside its file** (`src/rules/mod.rs:11`, `src/rules/npc_party.rs`): The `npc_party` module is declared as `pub mod npc_party` and contains a complete NPC party generation system (14+ public functions, data loading from `npc_parties.json`). The entire module is annotated with `#![allow(dead_code)]` (line 6). No other file in the codebase imports or references `npc_party`. The module compiles and its data file loads successfully, but it's entirely disconnected from the rest of the system. The `#![allow(dead_code)]` confirms the author knew this — it's intentional future work, but worth tracking as unwired functionality.

## Observations
(Non-blocking notes and suggestions)

- **All Cargo.toml dependencies are actively used**: `serde` (28 files), `serde_json` (21 files), `rand` (14 files). No phantom dependencies.

- **No unused environment variables**: Only `HOME` and `CARGO_PKG_VERSION` are referenced in the codebase. Both are consumed where defined. No `.env` files exist.

- **Inconsistent data loading strategy**: Core data files use two different approaches — `include_str!()` for compile-time embedding (`encounters.json`, `npc_parties.json`) vs `fs::read_to_string()` for runtime loading (equipment, magic items, monsters, spells, treasure). This isn't a bug, but it means some data changes require recompilation while others don't.

- **Five `#[allow(dead_code)]` annotations on JSON metadata structs** (`treasure.rs:112`, `spell_data.rs:50`, `monster.rs:168`, `magic_item.rs:66`, `equipment.rs:311`): These suppress warnings on wrapper structs like `TreasureFile` whose `source` field is deserialized but never read. This is correct — the fields exist for JSON schema compatibility. Added in commit `67bdf9e`.

- **`TODO` comments in monster loading** (`src/rules/monster.rs:263-264`): Two TODO markers note planned support for module-specific and user-custom monster data loading. These represent known future work, not incomplete integration.
