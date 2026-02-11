use std::path::{Path, PathBuf};

pub fn resolve_path(base: &Path, path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        base.join(p)
    }
}

pub fn display_path_rel_to_cwd(path: &Path, cwd: Option<&Path>) -> String {
    if let Some(cwd) = cwd {
        if let Ok(rel) = path.strip_prefix(cwd) {
            return rel.display().to_string();
        }
    }
    path.display().to_string()
}

pub fn ensure_parent_directory(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn is_binary_file(path: &Path) -> bool {
    let data = match std::fs::read(path) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if data.is_empty() {
        return false;
    }
    data.iter().take(4096).any(|b| *b == 0)
}
