//! Shared path security utilities for preventing path traversal attacks.
//!
//! These functions are used by both the save/load system (`persist`) and
//! module loading (`rules::module`) to ensure user-controlled paths resolve
//! within their designated directories.

use std::path::{Component, Path, PathBuf};

/// Normalize a path by resolving `.` and `..` components without filesystem access.
///
/// This is a pure string-level operation that does NOT follow symlinks or check
/// file existence. Use this for early rejection of traversal attempts before
/// hitting the filesystem.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut parts: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                // Only pop if the last component is a normal directory
                if matches!(parts.last(), Some(Component::Normal(_))) {
                    parts.pop();
                } else {
                    parts.push(component);
                }
            }
            Component::CurDir => {} // skip
            c => parts.push(c),
        }
    }
    parts.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_simple_path() {
        let p = normalize_path(Path::new("/a/b/c"));
        assert_eq!(p, PathBuf::from("/a/b/c"));
    }

    #[test]
    fn normalize_removes_dotdot() {
        let p = normalize_path(Path::new("/a/b/../c"));
        assert_eq!(p, PathBuf::from("/a/c"));
    }

    #[test]
    fn normalize_removes_dot() {
        let p = normalize_path(Path::new("/a/./b/./c"));
        assert_eq!(p, PathBuf::from("/a/b/c"));
    }

    #[test]
    fn normalize_multiple_dotdot() {
        let p = normalize_path(Path::new("/a/b/c/../../d"));
        assert_eq!(p, PathBuf::from("/a/d"));
    }

    #[test]
    fn normalize_dotdot_at_root() {
        let p = normalize_path(Path::new("/a/../.."));
        // After popping /a, the .. has nothing normal to pop — it stays
        assert_eq!(p, PathBuf::from("/.."));
    }

    #[test]
    fn normalize_relative_path() {
        let p = normalize_path(Path::new("a/b/../c"));
        assert_eq!(p, PathBuf::from("a/c"));
    }
}
