//! `--root` enforcement (RFC-094 §4): "every path an agent passes is
//! resolved, following symbolic links; a path that is not inside a
//! `--root` is refused, with an error naming the rule; the person who
//! configures the server decides the roots, and the agent cannot widen
//! them."

use std::path::PathBuf;

/// The server's fixed set of roots, canonicalized once at startup
/// (`mcp::parse_args`). Never mutated after that — nothing an agent sends
/// can add to or change this set.
#[derive(Clone)]
pub struct Roots(Vec<PathBuf>);

impl Roots {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self(roots)
    }

    /// Resolves `path` (following symlinks, and normalizing `..`, since
    /// both are exactly what [`std::fs::canonicalize`] does against the
    /// real filesystem) and checks the result falls inside one of the
    /// configured roots. A path that does not exist is refused the same
    /// way a path outside every root is — `canonicalize` cannot succeed
    /// on it either, so there is nothing to check containment against.
    pub fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let canonical = std::fs::canonicalize(path).map_err(|e| format!("{path}: {e}"))?;
        if self.0.iter().any(|root| canonical.starts_with(root)) {
            Ok(canonical)
        } else {
            Err(format!("{path}: outside every configured --root directory"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_inside_the_root_resolves() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        let roots = Roots::new(vec![std::fs::canonicalize(tmp.path()).unwrap()]);
        assert!(roots.resolve(file.to_str().unwrap()).is_ok());
    }

    #[test]
    fn a_path_outside_every_root_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let outside_file = elsewhere.path().join("b.txt");
        std::fs::write(&outside_file, "x").unwrap();
        let roots = Roots::new(vec![std::fs::canonicalize(tmp.path()).unwrap()]);
        let err = roots.resolve(outside_file.to_str().unwrap()).unwrap_err();
        assert!(
            err.contains("outside every configured --root directory"),
            "{err}"
        );
    }

    #[test]
    fn dot_dot_cannot_escape_the_root() {
        let tmp = tempfile::tempdir().unwrap();
        let inside = tmp.path().join("inside");
        std::fs::create_dir(&inside).unwrap();
        let roots = Roots::new(vec![std::fs::canonicalize(&inside).unwrap()]);
        // inside/../ is the root's own parent - outside the root itself.
        let escape = inside.join("..");
        let err = roots.resolve(escape.to_str().unwrap()).unwrap_err();
        assert!(
            err.contains("outside every configured --root directory"),
            "{err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_escaping_the_root_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let root_dir = tmp.path().join("root");
        std::fs::create_dir(&root_dir).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.txt");
        std::fs::write(&secret, "s").unwrap();
        let link = root_dir.join("escape");
        std::os::unix::fs::symlink(&secret, &link).unwrap();

        let roots = Roots::new(vec![std::fs::canonicalize(&root_dir).unwrap()]);
        let err = roots.resolve(link.to_str().unwrap()).unwrap_err();
        assert!(
            err.contains("outside every configured --root directory"),
            "{err}"
        );
    }

    #[test]
    fn a_nonexistent_path_is_refused_not_panicked_on() {
        let tmp = tempfile::tempdir().unwrap();
        let roots = Roots::new(vec![std::fs::canonicalize(tmp.path()).unwrap()]);
        let missing = tmp.path().join("does-not-exist.txt");
        assert!(roots.resolve(missing.to_str().unwrap()).is_err());
    }
}
