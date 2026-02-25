use ttrpg_interp::state::EntityRef;

/// Offset for monster entity refs. Characters occupy 0..MONSTER_BASE-1,
/// monsters occupy MONSTER_BASE..
const MONSTER_BASE: u64 = 10_000;

/// Create an EntityRef for a party character by index.
pub fn character_ref(index: usize) -> EntityRef {
    EntityRef(index as u64)
}

/// Create an EntityRef for a combat monster by index.
pub fn monster_ref(index: usize) -> EntityRef {
    EntityRef(MONSTER_BASE + index as u64)
}

/// Returns true if the entity ref points to a character.
pub fn is_character(entity: &EntityRef) -> bool {
    entity.0 < MONSTER_BASE
}

/// Returns true if the entity ref points to a monster.
pub fn is_monster(entity: &EntityRef) -> bool {
    entity.0 >= MONSTER_BASE
}

/// Extract the character index from an entity ref.
/// Panics if not a character ref.
pub fn character_index(entity: &EntityRef) -> usize {
    debug_assert!(is_character(entity));
    entity.0 as usize
}

/// Extract the monster index from an entity ref.
/// Panics if not a monster ref.
pub fn monster_index(entity: &EntityRef) -> usize {
    debug_assert!(is_monster(entity));
    (entity.0 - MONSTER_BASE) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_refs() {
        let r = character_ref(0);
        assert!(is_character(&r));
        assert!(!is_monster(&r));
        assert_eq!(character_index(&r), 0);

        let r = character_ref(5);
        assert_eq!(character_index(&r), 5);
    }

    #[test]
    fn monster_refs() {
        let r = monster_ref(0);
        assert!(!is_character(&r));
        assert!(is_monster(&r));
        assert_eq!(monster_index(&r), 0);

        let r = monster_ref(3);
        assert_eq!(monster_index(&r), 3);
    }

    #[test]
    fn no_overlap() {
        // Even with many characters, no overlap with monsters
        let c = character_ref(9999);
        let m = monster_ref(0);
        assert!(is_character(&c));
        assert!(is_monster(&m));
        assert_ne!(c, m);
    }
}
