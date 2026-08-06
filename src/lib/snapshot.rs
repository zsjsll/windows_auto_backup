use std::ops::Deref;

use std::path::PathBuf;

use std::process::Command;

use std::fs;

use crate::files::Files;

use encoding_rs::GBK;
use time::{
    OffsetDateTime,
    macros::{format_description, offset},
};

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Config {
    pub backup_dir: PathBuf,
    pub exec_path: ExecPath,
    pub args: Vec<String>,
    pub limit_backup_files_count: usize,
    pub now_utc: OffsetDateTime,
    pub system_info: String,
    pub backup_interval: i64,
    pub file_ext: FileExt,
}

#[cfg_attr(feature = "dbg", derive(Debug))]

pub struct FileExt {
    pub backup: &'static str,
    pub hash: &'static str,
}

#[cfg_attr(feature = "dbg", derive(Debug))]

pub struct ExecPath {
    pub backup: PathBuf,
    pub command: PathBuf,
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
    #[instrument(err(Display), level = "debug")]
    pub fn check_backup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 获取最新文件的信息
        let (latest_backup_file, latest_timestamp) = Files::new(&self.backup_dir)
            .get_latest_file(self.file_ext.backup)
            .unwrap_or_else(|| (self.backup_dir.clone(), OffsetDateTime::UNIX_EPOCH));

        // 获取时区偏移量
        let utc_offset = OffsetDateTime::now_local()
            .ok()
            .map(|v| v.offset())
            .unwrap_or_else(|| offset!(+8));
        info!(
            "最新备份文件信息\n路径: {}\n时间: {}",
            latest_backup_file.display(),
            latest_timestamp.to_offset(utc_offset)
        );

        let diff_hours = (self.now_utc - latest_timestamp).whole_hours();
        info!("时间间隔: {}h", &diff_hours);
        // 判断是否需要备份
        if diff_hours < self.backup_interval {
            let e = format!("未满足间隔时间: {} 小时", self.backup_interval);
            return Err(e.into());
        }
        // 检查并删除上一次的错误备份
        info!("检查最新备份文件是否完整");
        if latest_backup_file.is_file() {
            let check = format!(r"--QuickCheck:{}", latest_backup_file.to_string_lossy());
            self.doing(&[check])
                .inspect(|_| info!("备份文件完整"))
                .inspect_err(|err| {
                    error!(err);
                    error!("备份文件错误, 进行清理");

                    fs::remove_file(&latest_backup_file).ok();
                    let latest_hash_file = latest_backup_file.with_extension(self.file_ext.hash);
                    fs::remove_file(&latest_hash_file).ok();
                })
                .ok();
        }
        Ok(())
    }

    pub fn init_backup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 创建需要的目录
        info!("初始化备份信息");
        let archived_dir = &self.backup_dir.join("archived");
        fs::create_dir_all(archived_dir).ok();

        // 检查是否对文件进行归档, 并对归档文件进行清理
        let has_enough_backup_files = Files::new(&self.backup_dir)
            .has_files_count_gt_n(self.file_ext.backup, self.limit_backup_files_count);
        let has_enough_archived_files = Files::new(archived_dir)
            .has_files_count_gt_n(self.file_ext.backup, self.limit_backup_files_count);

        if has_enough_archived_files && has_enough_backup_files {
            fs::remove_dir_all(archived_dir).ok();
            warn!("达到归档数量上限, 已进行清理");
        }

        if has_enough_backup_files {
            fs::create_dir_all(archived_dir).ok();

            Files::new(&self.backup_dir)
                .all_files()
                .try_for_each(|file| {
                    let destination = archived_dir.join(file.file_name());
                    fs::rename(file.path(), &destination)
                })
                .ok();
            warn!("已成功将文件移动到 {} 目录!", archived_dir.display());
        }
        Ok(())
    }

    pub fn start_backup(&self) -> Result<(), Box<dyn std::error::Error>> {
        let args = self.create_backup_args();
        self.doing(&args)?;

        Ok(())
    }

    fn create_backup_args(&self) -> Vec<String> {
        // 判断备份方式
        let timer_format = format_description!("[year]-[month]-[day]_[hour][minute]");
        let time_string = self.now_utc.format(timer_format).unwrap_or_default();

        let backup_file_name = format!(
            "{}_{}.{}",
            self.system_info, time_string, self.file_ext.backup
        );

        let backup_volumes = &self.args[0];
        let backup_file_path = self.backup_dir.join(backup_file_name);

        let mut args: Vec<String> = Vec::with_capacity(30);
        args.extend_from_slice(&[
            backup_volumes.into(),
            backup_file_path.to_string_lossy().into(),
        ]);

        let hash_arg = Files::new(&self.backup_dir)
            .get_latest_file(&self.file_ext.hash)
            .map(|(f, _)| {
                info!("创建 [差异备份] 参数");
                format!("-h{}", f.to_string_lossy())
            })
            .unwrap_or_else(|| {
                info!("创建 [完整备份] 参数");
                let hash_file_path = backup_file_path.with_extension(self.file_ext.hash);
                format!("-o{}", hash_file_path.to_string_lossy())
            });

        args.push(hash_arg);
        args.extend_from_slice(&self.args[1..]);
        args
    }

    fn doing(&self, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = if self.exec_path.command.is_file() {
            let mut cmd = Command::new(&self.exec_path.command);
            cmd.arg(&self.exec_path.backup);
            cmd
        } else {
            Command::new(&self.exec_path.backup)
        };

        let output = cmd.args(args).output()?;

        if output.status.success() {
            let (msg, _, _) = GBK.decode(&output.stdout);
            info!("{}", msg);
            Ok(())
        } else {
            let (err_msg, _, _) = GBK.decode(&output.stderr);
            let (msg, _, _) = GBK.decode(&output.stdout);
            let err = format!("\n{msg}\n{err_msg}");
            Err(err.into())
        }
    }
}
