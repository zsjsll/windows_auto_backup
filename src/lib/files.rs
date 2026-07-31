use std::{
    fs::{self, DirEntry, ReadDir},
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct Files {
    pub dir: PathBuf,
}

impl Files {
    pub fn new(dir: &Path) -> Self {
        Self { dir: dir.into() }
    }

    pub fn all_files(&self) -> impl Iterator<Item = DirEntry> + '_ {
        fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|entery| entery.metadata().is_ok_and(|x| x.is_file()))
    }

    pub fn get_ext_file(&self, ext: &str) -> impl Iterator<Item = DirEntry> + '_ {
        let ext1 = ext.to_ascii_lowercase();
        self.all_files().filter(move |entry| {
            entry
                .path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(&ext1))
        })
    }
}
