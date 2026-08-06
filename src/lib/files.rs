use std::{
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    time::SystemTime,
};

use time::OffsetDateTime;
pub struct Files {
    dir: PathBuf,
}

impl Files {
    pub fn new(dir: &Path) -> Self {
        Self { dir: dir.into() }
    }

    pub fn all_files(self) -> impl Iterator<Item = DirEntry> {
        fs::read_dir(self.dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(move |entery| entery.metadata().is_ok_and(|x| x.is_file()))
    }

    pub fn get_ext_files(self, ext: &str) -> impl Iterator<Item = DirEntry> {
        let ext = ext.to_ascii_lowercase();
        self.all_files().filter(move |entry| {
            entry
                .path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(&ext))
        })
    }

    pub fn get_latest_file(self, ext: &str) -> Option<(PathBuf, OffsetDateTime)> {
        let ext = ext.to_ascii_lowercase();

        let latest = self.get_ext_files(&ext).max_by_key(|entry| {
            entry
                .metadata()
                .and_then(|o| o.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })?;

        let file_date_time: OffsetDateTime = latest.metadata().ok()?.modified().ok()?.into();

        Some((latest.path(), file_date_time))
    }

    pub fn has_files_count_gt_n(self, ext: &str, n: usize) -> bool {
        let ext = ext.to_ascii_lowercase();
        self.get_ext_files(&ext).nth(n - 1).is_some()
    }
}
