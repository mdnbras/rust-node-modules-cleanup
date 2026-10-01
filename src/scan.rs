use std::{
    fs, io,
    path::{Path, PathBuf},
    thread,
};

#[derive(Debug)]
pub struct Candidate {
    pub path: PathBuf,
    pub bytes: u64,
    pub warnings: Vec<String>,
    identity: fs::Metadata,
}

#[derive(Debug)]
pub struct Scan {
    pub root: PathBuf,
    pub candidates: Vec<Candidate>,
    pub warnings: Vec<String>,
}

// Also reject junctions and other reparse points on Windows.
fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

fn same_directory(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))]
    {
        a.created()
            .ok()
            .zip(b.created().ok())
            .is_some_and(|(a, b)| a == b)
    }
}

fn warning(path: &Path, error: io::Error) -> String {
    format!("{}: {error}", path.display())
}

fn checked_directory(path: &Path) -> io::Result<fs::Metadata> {
    let meta = fs::symlink_metadata(path)?;
    if is_link(&meta) || !meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a real directory, not a link",
        ));
    }
    Ok(meta)
}

// Iterative traversal avoids stack overflow for deeply nested dependency trees.
fn size(path: &Path) -> (u64, Vec<String>) {
    let mut pending = vec![path.to_path_buf()];
    let mut bytes = 0u64;
    let mut warnings = Vec::new();
    while let Some(dir) = pending.pop() {
        if let Err(e) = checked_directory(&dir) {
            warnings.push(warning(&dir, e));
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                warnings.push(warning(&dir, e));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    warnings.push(warning(&dir, e));
                    continue;
                }
            };
            let path = entry.path();
            match fs::symlink_metadata(&path) {
                Ok(meta) if is_link(&meta) => {}
                Ok(meta) if meta.is_dir() => pending.push(path),
                Ok(meta) if meta.is_file() => bytes = bytes.saturating_add(meta.len()),
                Ok(_) => {}
                Err(e) => warnings.push(warning(&path, e)),
            }
        }
    }
    (bytes, warnings)
}

pub fn scan(path: &Path, jobs: usize) -> io::Result<Scan> {
    if !(1..=64).contains(&jobs) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "jobs must be 1..64",
        ));
    }
    checked_directory(path)?;
    let root = fs::canonicalize(path)?;
    // Root read errors must not look like a successful empty search.
    fs::read_dir(&root)?;
    let mut pending = vec![root.clone()];
    let mut candidates = Vec::new();
    let mut warnings = Vec::new();
    while let Some(dir) = pending.pop() {
        let meta = match checked_directory(&dir) {
            Ok(meta) => meta,
            Err(e) => {
                warnings.push(warning(&dir, e));
                continue;
            }
        };
        if dir.file_name().is_some_and(|name| name == "node_modules") {
            candidates.push(Candidate {
                path: dir,
                bytes: 0,
                warnings: vec![],
                identity: meta,
            });
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                warnings.push(warning(&dir, e));
                continue;
            }
        };
        for entry in entries {
            match entry {
                Ok(entry) => {
                    if entry.file_name().to_string_lossy().starts_with('.') {
                        continue;
                    }
                    match fs::symlink_metadata(entry.path()) {
                        Ok(meta) if meta.is_dir() && !is_link(&meta) => pending.push(entry.path()),
                        Ok(_) => {}
                        Err(e) => warnings.push(warning(&entry.path(), e)),
                    }
                }
                Err(e) => warnings.push(warning(&dir, e)),
            }
        }
    }
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    for batch in candidates.chunks_mut(jobs) {
        thread::scope(|scope| {
            for candidate in batch {
                scope.spawn(move || {
                    (candidate.bytes, candidate.warnings) = size(&candidate.path);
                });
            }
        });
    }
    Ok(Scan {
        root,
        candidates,
        warnings,
    })
}

impl Scan {
    pub fn total_bytes(&self) -> u64 {
        self.candidates
            .iter()
            .fold(0u64, |n, c| n.saturating_add(c.bytes))
    }

    /// Revalidate the selected path before deleting. This is not a filesystem sandbox:
    /// callers must stop processes that mutate the tree while cleanup is running.
    pub fn delete(&self, candidate: &Candidate) -> io::Result<()> {
        let path = &candidate.path;
        if !path.starts_with(&self.root)
            || !path.file_name().is_some_and(|name| name == "node_modules")
            || fs::canonicalize(path)? != *path
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "candidate path changed or is outside search root",
            ));
        }
        let meta = checked_directory(path)?;
        if !same_directory(&candidate.identity, &meta) {
            return Err(io::Error::other("directory was replaced since scanning"));
        }
        fs::remove_dir_all(path)
    }
}
