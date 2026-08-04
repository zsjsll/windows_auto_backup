use std::ops::Deref;

use std::path::{Path, PathBuf};

use std::process::Command;

use std::fs;
use std::io;
use std::time::SystemTime;

use crate::files::Files;

use encoding_rs::GBK;
use time::{OffsetDateTime, macros::offset};

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Config {
    pub backup_exe_path: PathBuf,
    pub command_exe_path: PathBuf,
    pub backup_dir: PathBuf,

    pub base_path: BasePath,
    pub args: Vec<String>,
    pub limit_backup_files_count: usize,
    pub system_info: String,
    pub backup_interval: i64,
    pub file_ext: FileExt,
}

#[cfg_attr(feature = "dbg", derive(Debug))]

pub struct FileExt {
    pub backup: String,
    pub hash: String,
}

#[cfg_attr(feature = "dbg", derive(Debug))]

pub struct BasePath {
    pub exec: PathBuf,
    pub backup: PathBuf,
    pub archived: PathBuf,
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

    fn get_backup_files(&self) -> io::Result<Vec<(PathBuf, SystemTime)>> {
        let backup_files = fs::read_dir(&self.backup_dir)?
            .filter_map(|p| {
                let entry = p.ok()?;
                let path = entry.path();
                let metadata = entry.metadata().ok()?;
                let timestamp = metadata.modified().ok()?;

                let is_target = path.extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case(&self.file_ext.backup)
                        || ext.eq_ignore_ascii_case(&self.file_ext.hash)
                });

                is_target.then_some((path, timestamp))
            })
            .collect();
        Ok(backup_files)
    }

    fn get_files(&self, dir: &Path) -> Option<impl Iterator<Item = (PathBuf, SystemTime)>> {
        let files = fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .filter(|entery| entery.metadata().is_ok_and(|x| x.is_file()))
            .filter(|entry| {
                entry.path().extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case(&self.file_ext.backup)
                        || ext.eq_ignore_ascii_case(&self.file_ext.hash)
                })
            })
            .filter_map(|entery| {
                entery
                    .metadata()
                    .ok()?
                    .modified()
                    .ok()
                    .map(|timestamp| (entery.path(), timestamp))
            });

        Some(files)
    }

    fn has_enough_files(&self, dir: &Path) -> bool {
        dir.is_dir()
            && fs::read_dir(dir)
                .ok()
                .is_some_and(|mut p| p.nth(self.limit_backup_file_count).is_some())
    }

    fn has_files_count_gt_n(&self, dir: &Path, ext: &str, n: usize) -> bool {
        let mut files = Files::new(dir).get_ext_files(ext);
        files.nth(n - 1).is_some()
    }

    fn create_file_name(&self, dir: &Path) {
        let files = self.get_files(dir);
    }

    fn create_backup_file_name(
        &self,
        backup_files: &[(PathBuf, SystemTime)],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let now = time::OffsetDateTime::now_local()?;

        let timestamp: time::OffsetDateTime = backup_files
            .iter()
            .filter_map(|(path, timestamp)| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(&self.file_ext.backup))
                    .then_some(*timestamp)
            })
            .max()
            .map(Into::into)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);

        let diff_hours = (now - timestamp).abs().whole_hours();

        if diff_hours < self.backup_interval {
            let e = format!("未满足间隔时间: {} 小时", self.backup_interval);
            return Err(e.into());
        }

        let timer_format = time::macros::format_description!("[year]-[month]-[day]_[hour][minute]");
        let time_string = now.format(timer_format)?;

        let backup_file_name = format!(
            "{}_{}.{}",
            self.system_info, time_string, &self.file_ext.backup
        );
        Ok(backup_file_name)
    }

    #[instrument(err(Display), level = "debug")]
    pub fn pre_packup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 获取最新文件的信息
        let (latest_backup_file, latest_timestamp) = Files::new(&self.backup_dir)
            .get_latest_file(&self.file_ext.backup)
            .unwrap_or_else(|| (self.backup_dir.clone(), OffsetDateTime::UNIX_EPOCH));

        // 获取时区偏移量
        let utc_offset = OffsetDateTime::now_local()
            .ok()
            .map(|v| v.offset())
            .unwrap_or(offset!(+8));

        info!(
            "最新备份文件信息\n路径: {}\n时间: {}",
            latest_backup_file.display(),
            latest_timestamp.to_offset(utc_offset)
        );

        // 获取当前utc时间
        let now = OffsetDateTime::now_utc();
        let diff_hours = (now - latest_timestamp).whole_hours();

        // 判断是否需要备份
        if diff_hours < self.backup_interval {
            let e = format!("未满足间隔时间: {} 小时", self.backup_interval);
            return Err(e.into());
        }

        // 检查并删除上一次的错误备份
        if latest_backup_file.is_file() {
            let check = format!(r"--QuickCheck:{}", latest_backup_file.to_string_lossy());
            let _ = self.doing(&[check]).inspect_err(|e| {
                error!(e);
                error!("发现备份错误, 进行清理");

                fs::remove_file(&latest_backup_file).ok();
                let latest_hash_file = latest_backup_file.with_extension(&self.file_ext.hash);
                fs::remove_file(&latest_hash_file).ok();
            });
        }

        // 创建需要的目录
        let archived_dir = &self.backup_dir.join("archived");
        fs::create_dir_all(archived_dir).ok();

        // 检查是否对文件进行归档, 并对归档文件进行清理

        // let has_enough_archived_files =Files::new(&)
        // let has_enough_backup_files = backup_files.len() > self.archived_number;

        Ok(())
    }

    #[instrument(err(Display), level = "debug")]
    pub fn backup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 创建需要的目录
        let archived_dir = &self.backup_dir.join("archived");
        fs::create_dir_all(archived_dir)?;

        // 读取目录下的文件
        let backup_files = self.get_backup_files()?;

        // 生成备份文件的名字
        let new_backup_file_name = self.create_backup_file_name(&backup_files)?;
        let dist_path = self.backup_dir.join(new_backup_file_name);
        let hash_file_name = format!("hash.{}", &self.file_ext.hash);
        let hash_path = self.backup_dir.join(hash_file_name);

        let ex_args = vec![
            dist_path.to_string_lossy().to_string(),
            format!("-h{}", hash_path.to_string_lossy()),
        ];

        // 检查是否对文件进行归档, 并对归档文件进行清理
        let has_enough_archived_files = self.has_enough_files(archived_dir);
        let has_enough_backup_files = backup_files.len() > self.limit_backup_file_count;
        if has_enough_archived_files && has_enough_backup_files {
            warn!("已达到归档数量上限, 进行清理");
            fs::remove_dir_all(archived_dir)?;
        }

        if has_enough_backup_files {
            fs::create_dir_all(archived_dir)?;

            backup_files.iter().try_for_each(|(backup_file, _)| {
                let backup_file_name = backup_file.file_name().unwrap_or_default();

                let destination = archived_dir.join(backup_file_name);
                fs::rename(backup_file, &destination)
            })?;
            warn!("已成功将文件移动到 doc 目录!");
        }

        let a = self.has_files_count_gt_n(&self.backup_dir, &self.file_ext.backup, 2);
        dbg!(&a);

        // self.doing(&ex_args)?;
        Ok(())
    }

    fn doing(&self, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let output = Command::new("gsudo.exe")
            .arg(&self.exe_path)
            .args(args)
            .output()?;

        if output.status.success() {
            info!("运行成功");
            let (msg, _, _) = GBK.decode(&output.stdout);
            info!("{}", msg);
            Ok(())
        } else {
            let (err_msg, _, _) = GBK.decode(&output.stderr);
            let (msg, _, _) = GBK.decode(&output.stdout);
            let err = format!("运行出错: {msg}\n{err_msg}");
            Err(err.into())
        }
    }
}
