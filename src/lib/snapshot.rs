use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use std::fs;
use std::io;

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Config {
    pub exe_path: PathBuf,
    pub args: Vec<String>,
    pub archived_number: usize,
}

impl From<Config> for Snapshot {
    fn from(config: Config) -> Self {
        Self { config: config }
    }
}

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Snapshot {
    config: Config,
}

impl Snapshot {
    pub fn show_config(&self) {
        dbg!(&self.config);
    }

    #[instrument(err(Display), level = "debug")]
    pub fn init_backup_dir(&self) -> io::Result<()> {
        let work_dir = Path::new(r"\\10.10.0.201\backup\snapshot\work");
        let archived_dir = work_dir.join("archived");
        // fs::create_dir_all(&doc_dir)?;

        let backup_files: Vec<_> = fs::read_dir(work_dir)?
            .filter_map(|e| {
                let path = e.ok()?.path();
                let ext = path.extension()?.to_str()?;

                if ext.eq_ignore_ascii_case("sna") || ext.eq_ignore_ascii_case("hsh") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

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
                .nth(self.config.archived_number)
                .is_some()
        } else {
            false
        };

        dbg!(&has_enough_archived_files);

        let has_enough_backup_files = backup_files.len() > self.config.archived_number;

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
            .arg(&self.config.exe_path)
            .args(&self.config.args)
            // .arg("/?")
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
