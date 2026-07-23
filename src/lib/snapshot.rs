use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use std::fs;
use std::io;

use time::Date;
use time::Time;
use time::UtcOffset;

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Config {
    pub exe_path: PathBuf,
    pub backup_dir: PathBuf,
    pub args: Vec<String>,
    pub archived_number: usize,
    pub system_info: SystemInfo,
    pub time_info: TimeInfo,
    pub file_ext: FileExt,
}

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct SystemInfo {
    pub computer_name: String,
    pub major: String,
    pub minor: String,
    pub pack: String,
    pub build: String,
    pub ubr: String,
}
#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct TimeInfo {
    pub date: Date,
    pub time: Time,
    pub offset: UtcOffset,
}

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct FileExt {
    pub backup: String,
    pub hash: String,
}

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Snapshot(Config);

impl From<Config> for Snapshot {
    fn from(config: Config) -> Self {
        Self(config)
    }
}

impl Deref for Snapshot {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Snapshot {
    fn has_hash_file(&self, backup_files: &[PathBuf]) -> bool {
        let result = backup_files.iter().any(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case(&self.file_ext.hash))
        });
        result
    }

    fn get_backup_files(&self) -> io::Result<Vec<PathBuf>> {
        let backup_files: Vec<PathBuf> = fs::read_dir(&self.backup_dir)?
            .filter_map(|p| {
                let path = p.ok()?.path();
                let is_target = path.extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case(&self.file_ext.backup)
                        || ext.eq_ignore_ascii_case(&self.file_ext.hash)
                });
                is_target.then_some(path)
            })
            .collect();
        Ok(backup_files)
    }

    fn has_enough_archived_files(&self, archived_dir: &Path) -> bool {
        archived_dir
            .is_dir()
            .then(|| {
                fs::read_dir(archived_dir)
                    .ok()
                    .is_some_and(|mut p| p.nth(self.archived_number).is_some())
            })
            .unwrap_or(false)
    }

    #[instrument(err(Display), level = "debug")]
    pub fn init_backup_dir(&self) -> io::Result<()> {
        // 1 创建需要的目录
        let archived_dir = &self.backup_dir.join("archived");
        fs::create_dir_all(archived_dir)?;

        // 2 读取目录下的文件
        let backup_files = self.get_backup_files()?;

        // fs::create_dir_all(&doc_dir)?;

        let has_enough_archived_files = if archived_dir.is_dir() {
            fs::read_dir(&archived_dir)?
                .filter_map(|e| {
                    let entry = e.ok()?.path();
                    let ext = entry.extension()?.to_str()?;

                    if ext.eq_ignore_ascii_case("sna") || ext.eq_ignore_ascii_case("hsh") {
                        Some(()) // 匹配成功，只保留一个信号单位
                    } else {
                        None
                    }
                })
                .nth(self.archived_number)
                .is_some()
        } else {
            false
        };

        let has_enough_backup_files = backup_files.len() > self.archived_number;

        if has_enough_archived_files && has_enough_backup_files {
            warn!("已达到归档数量上限, 进行清理");
            fs::remove_dir_all(&archived_dir)?;
        }

        if has_enough_backup_files {
            fs::create_dir_all(&archived_dir)?;

            for backup_file in backup_files {
                // 获取文件名（例如 "example.txt"）
                if let Some(file_name) = backup_file.file_name() {
                    // 拼接出新路径：\\10.10.0.201\backup\snapshot\work\doc\example.txt
                    let destination = archived_dir.join(file_name);
                    // 执行移动操作
                    fs::rename(&backup_file, &destination)?;
                }
            }
            info!("已成功将文件移动到 doc 目录！");
        }
        Ok(())
    }

    #[instrument(err(Display), level = "debug")]
    pub fn backup(&self) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("MinSudo.exe")
            .arg("-NoL")
            .arg(&self.exe_path)
            // .args(&self.args)
            .arg("/?")
            .output()?;

        if output.status.success() {
            info!("✅ 已备份");
            let (msg, _, _) = encoding_rs::GBK.decode(&output.stdout);
            info!("{}", msg);
            Ok(())
        } else {
            let (err_msg, _, _) = encoding_rs::GBK.decode(&output.stderr);
            error!("❌ 备份出错");
            Err(err_msg.into())
        }
    }
}
