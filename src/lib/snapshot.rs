use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use std::fs;
use std::io;
use std::time::SystemTime;

use encoding_rs::GBK;

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Config {
    pub exe_path: PathBuf,
    pub backup_dir: PathBuf,
    pub args: Vec<String>,
    pub archived_number: usize,
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

    fn has_enough_files(&self, archived_dir: &Path) -> bool {
        archived_dir.is_dir()
            && fs::read_dir(archived_dir)
                .ok()
                .is_some_and(|mut p| p.nth(self.archived_number).is_some())
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
        let has_enough_backup_files = backup_files.len() > self.archived_number;
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

        self.doing(&ex_args)?;
        Ok(())
    }

    fn doing(&self, ex_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        info!("开始备份");
        let output = Command::new("gsudo.exe")
            // .arg("-NoL")
            .arg(&self.exe_path)
            .args(&self.args)
            .args(ex_args)
            .output()?;

        // let output = Command::new(&self.exe_path)
        //     // .args(&self.args)
        //     // .args(ex_args)
        //     // .arg(r"/?")
        //     .output()?;

        if output.status.success() {
            info!("✅ 已备份");
            let (msg, _, _) = GBK.decode(&output.stdout);
            info!("{}", msg);
            Ok(())
        } else {
            let (err_msg, _, _) = GBK.decode(&output.stderr);
            error!("❌ 备份出错");
            Err(err_msg.into())
        }
    }
}
