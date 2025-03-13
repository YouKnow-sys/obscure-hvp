use std::path::{Path, PathBuf};

pub fn is_file(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_file() {
        return Err("You need to pass a valid file path.".to_owned());
    }
    Ok(path.to_path_buf())
}

pub fn is_dir(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_dir() {
        return Err("You need to pass a valid dir path.".to_owned());
    }
    Ok(path.to_path_buf())
}

pub fn list_files(input: &Path, without_base: bool) -> Vec<PathBuf> {
    walkdir::WalkDir::new(input)
        .into_iter()
        .filter_map(|e| {
            let f = e.ok()?;
            if f.path().is_dir() {
                return None;
            }
            let path = if without_base {
                f.path().strip_prefix(input).ok()?
            } else {
                f.path()
            };
            Some(path.to_path_buf())
        })
        .collect()
}
