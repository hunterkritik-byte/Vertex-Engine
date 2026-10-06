use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetEntry { pub path: PathBuf, pub is_directory: bool }

pub fn scan_assets(root: impl AsRef<Path>) -> Vec<AssetEntry> {
    let Ok(entries) = fs::read_dir(root) else { return Vec::new() };
    let mut assets = entries.flatten().map(|entry| {
        let path = entry.path();
        AssetEntry { is_directory: path.is_dir(), path }
    }).collect::<Vec<_>>();
    assets.sort_by(|a,b| a.path.cmp(&b.path));
    assets
}
