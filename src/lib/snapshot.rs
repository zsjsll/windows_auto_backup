use std::ops::Deref;
use std::path::PathBuf;
use std::process::Command;
use std::{fs, path::Path};

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
    pub now_date_time: OffsetDateTime,
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
    fn check_backup_interval(
        &self,
        latest_backup_date_time: OffsetDateTime,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let interval_hours = (self.now_date_time - latest_backup_date_time).whole_hours();
        info!("时间间隔: {}h", &interval_hours);
        // 判断是否需要备份
        if interval_hours < self.backup_interval {
            let e = format!("未满足条件, 间隔时间 > {}h", self.backup_interval);
            return Err(e.into());
        }
        info!("已满足条件, 间隔时间 > {}h", self.backup_interval);
        Ok(())
    }

    fn check_backup_file(
        &self,
        latest_backup_file_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if latest_backup_file_path.is_dir() {
            info!("不存在备份文件, 跳过检查");
            return Ok(());
        }
        let quick_check = format!(
            r"--QuickCheck:{}",
            latest_backup_file_path.to_string_lossy()
        );
        self.doing(&[quick_check])
            .inspect(|_| info!("备份文件完整, 通过检查"))
            .inspect_err(|_| {
                warn!("备份文件错误, 清理完成");
            })
            .ok();
        Ok(())
    }

    fn check_backup_dir(&self) -> Result<(), Box<dyn std::error::Error>> {
        let archived_dir = &self.backup_dir.join("archived");

        // 检查是否对文件进行归档, 并对归档文件进行清理
        let has_enough_backup_files = Files::new(&self.backup_dir)
            .has_files_count_gt_n(self.file_ext.backup, self.limit_backup_files_count);

        if !has_enough_backup_files {
            info!("文件数量 < {}份, 不需要归档", self.limit_backup_files_count);
            return Ok(());
        }

        info!(
            "文件数量 >= {}份, 需要进行归档",
            self.limit_backup_files_count
        );

        if archived_dir.is_dir() {
            info!("发现并清空原归档文件夹: {}", archived_dir.display());
            fs::remove_dir_all(archived_dir)
                .inspect(|_| info!("清空成功"))
                .inspect_err(|e| error!("清空失败: {}", e))
                .ok();
        }

        info!("归档文件到: {}", archived_dir.display());

        let temp_dir = &self
            .backup_dir
            .parent()
            .map(|dir| dir.join("temp"))
            .ok_or("没有父目录, 无法完成归档")?;

        fs::rename(&self.backup_dir, temp_dir)?;
        fs::create_dir_all(&self.backup_dir)?;
        fs::rename(temp_dir, archived_dir)?;

        Ok(())
    }

    #[instrument(err(Display), level = "debug")]
    pub fn init_backup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 获取最新文件的信息
        let latest_backup_file = Files::new(&self.backup_dir).get_latest_file(self.file_ext.backup);
        let latest_backup_file_path = latest_backup_file
            .as_ref()
            .map(|f| f.path())
            .unwrap_or(self.backup_dir.clone());
        // 获取时间
        let latest_backup_date_time = latest_backup_file
            .as_ref()
            .and_then(|f| f.metadata().ok()?.modified().ok())
            .map(OffsetDateTime::from)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        // 获取时区偏移量
        let offset = self.now_date_time.offset();
        let offset = offset.is_utc().then(|| offset!(+8)).unwrap_or(offset);

        info!(
            "最新备份文件信息\n路径: {}\n时间: {}",
            latest_backup_file_path.display(),
            latest_backup_date_time.to_offset(offset)
        );

        info!("检查时间间隔");
        self.check_backup_interval(latest_backup_date_time)?;

        info!("检查最新备份文件完整性");
        self.check_backup_file(&latest_backup_file_path)
            .inspect_err(|e| error!("{e}"))
            .ok();

        info!("检查是否需要归档");
        self.check_backup_dir().inspect_err(|e| error!("{e}")).ok();

        Ok(())
    }

    pub fn start_backup(&self) -> Result<(), Box<dyn std::error::Error>> {
        let backup_args = self.create_backup_args();
        self.doing(&backup_args)?;

        Ok(())
    }

    fn create_backup_args(&self) -> Vec<String> {
        // 判断备份方式
        let timer_format = format_description!("[year]-[month]-[day]_[hour][minute]");
        let time_string = self.now_date_time.format(timer_format).unwrap_or_default();

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
            .map(|f| {
                info!("创建 [差异备份] 参数");
                format!("-h{}", f.path().to_string_lossy())
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
