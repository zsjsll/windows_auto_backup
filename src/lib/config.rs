use serde::Deserialize;

use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

use windows_version::{OsVersion, revision};

use super::{logs, smb, snapshot};

#[cfg_attr(feature = "dbg", derive(Debug))]
#[derive(Deserialize)]
pub struct AppConfig {
    log_level: Arc<String>,
    smb: SmbConfig,           // 对应 [smb] 区块
    snapshot: SnapshotConfig, // 对应 [strategy] 区块
}

#[cfg_attr(feature = "dbg", derive(Debug))]
#[derive(Deserialize)]
struct SmbConfig {
    server_ip: String,
    share_name: String,
    username: Arc<String>,
    password: Arc<String>,
}

#[cfg_attr(feature = "dbg", derive(Debug))]
#[derive(Deserialize)]
struct SnapshotConfig {
    exe_path: String,
    backup_volumes: String,
    dist_dir: String,
    backup_interval: i64,
    archived_number: usize,
    exclude: Vec<String>,
    limit_io_rate: u8,
    save_all_sectors: bool,
    disable_key: bool,
    test: bool,
    graph: bool,
    clean_recycle: bool,
}

impl AppConfig {
    #[instrument(err(Display), level = "debug")]
    pub fn new(path: impl AsRef<Path> + Debug) -> Result<Self, Box<dyn std::error::Error>> {
        // 🏅 1. 手动把入参带进来打印，想要就要，不想要可以随时删掉
        info!(
            "🚀 正在加载自定义 TOML 配置文件, 路径: {}",
            path.as_ref().display()
        );

        // 🌟 2. 读文件：如果翻车，用 map_err 物理拦截，打印最纯净的多行文本错误，然后用 ? 拍扁往上抛
        let config_content = fs::read_to_string(path).inspect_err(|_| {
            error!("❌ 读取配置文件失败");
        })?;

        // 🌟 3. 解析 TOML：如果翻车，同样原地打日志拦截，支持多行平铺展开
        let config: Self = toml::from_str(&config_content).inspect_err(|_| {
            error!("❌ TOML 语法解析失败");
        })?;

        Ok(config)
    }

    fn get_def_path(&self) -> PathBuf {
        if self.snapshot.dist_dir.is_empty() {
            PathBuf::from(r"\\")
                .join(&self.smb.server_ip)
                .join(&self.smb.share_name)
        } else {
            PathBuf::from(&self.snapshot.dist_dir)
        }
    }
    pub fn generate_smb_config(&self) -> smb::Config {
        let p = self.get_def_path();
        smb::Config {
            url: p,
            user: Arc::clone(&self.smb.username),
            passwd: Arc::clone(&self.smb.password),
        }
    }

    pub fn generate_logs_config(&self) -> logs::Config {
        logs::Config {
            log_level: Arc::clone(&self.log_level),
        }
    }
    // 获取 系统版本号
    fn get_system_info(&self) -> String {
        let build = OsVersion::current().build.to_string();
        let ubr = revision().to_string();
        format!("{build}({ubr})")
    }

    pub fn generate_snapshot_config(&self) -> snapshot::Config {
        let exe_path = PathBuf::from(r"./").join(&self.snapshot.exe_path);
        // 获取 计算机名字
        let computer_name = hostname::get().unwrap_or("unknown".into());
        let backup_dir = self.get_def_path().join("snapshot").join(computer_name);

        let system_info = self.get_system_info();

        let backup_file_ext = "sna".to_string();
        let hash_file_ext = "hsh".to_string();
        let file_ext = snapshot::FileExt {
            backup: backup_file_ext,
            hash: hash_file_ext,
        };
        let pre_exclude: Vec<String> = self
            .snapshot
            .exclude
            .iter()
            .map(|p| {
                // 先解析环境变量
                let path = p
                    .strip_prefix('%')
                    .and_then(|s| s.strip_suffix('%'))
                    .and_then(|var_name| env::var(var_name).ok())
                    .map(|env_path| env_path + r"\*")
                    .unwrap_or_else(|| p.clone());

                // 再统一剥离前缀
                path.strip_prefix(r"C:")
                    .map(|s| s.to_string())
                    .unwrap_or(path)
            })
            .collect();
        dbg!(&pre_exclude);

        let exclude = format!("--exclude:{}", pre_exclude.join(","));

        let mut args = Vec::with_capacity(50);

        args.extend([
            self.snapshot.backup_volumes.clone(),
            "-L0".into(),
            "--CreateDir".into(),
            "--FullIfHashIsMissing".into(),
        ]);

        // args.extend(exclude_args);

        macro_rules! push_flag {
            ($($field:expr => $flag:expr),* $(,)?) => {
                $(
                    if $field {
                        args.push($flag.into());
                    }
                )*
            };
        }

        let limit_io_rate = format!("--LimitIORate:{}", self.snapshot.limit_io_rate);
        push_flag!(
            !self.snapshot.exclude.is_empty()   => exclude,
            self.snapshot.limit_io_rate != 0    => limit_io_rate,
            self.snapshot.save_all_sectors      => "-A",
            self.snapshot.disable_key           => "-W",
            self.snapshot.test                  => "-T",
            self.snapshot.graph                 => "-G",
            self.snapshot.clean_recycle         => "-R",
        );

        snapshot::Config {
            exe_path,
            backup_dir,
            args,
            archived_number: self.snapshot.archived_number,
            system_info,
            backup_interval: self.snapshot.backup_interval,
            file_ext,
        }
    }
}
