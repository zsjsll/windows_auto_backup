use std::{
    borrow::Cow,
    ffi::OsStr,
    fs::{self, DirEntry, ReadDir},
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

    fn all_files(self) -> impl Iterator<Item = DirEntry> {
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

    pub fn get_latest_file(self, ext: &str) {
        let ext = ext.to_ascii_lowercase();

        let latest = self
            .get_ext_files(&ext)
            .filter_map(|entry| {
                entry
                    .metadata()
                    .ok()?
                    .modified()
                    .ok()
                    .map(|x| (entry.path(), x))
            })
            .max_by_key(|(_, times)| *times);
    }
}
