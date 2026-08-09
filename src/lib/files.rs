use std::{
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    time::SystemTime,
};

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
            .filter(|entry| entry.file_type().is_ok_and(|ft| ft.is_file()))
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

    pub fn get_latest_file(self, ext: &str) -> Option<DirEntry> {
        let ext = ext.to_ascii_lowercase();

        self.get_ext_files(&ext).max_by_key(|entry| {
            entry
                .metadata()
                .and_then(|o| o.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
    }

    pub fn has_files_count_gt_n(self, ext: &str, n: usize) -> bool {
        let ext = ext.to_ascii_lowercase();

        if n > 0 {
            return self.get_ext_files(&ext).nth(n - 1).is_some();
        }
        return true;
    }
}
