use serde::Deserialize;

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    archived_number: usize,
    source_dir: String,
    dist_dir: String,
    limit_IO_rate: u8,
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
    fn get_system_info(&self) -> snapshot::SystemInfo {
        let tags = OsVersion::current();
        let computer_name = hostname::get().unwrap().to_string_lossy().to_string();

        snapshot::SystemInfo {
            computer_name,
            major: tags.major.to_string(),
            minor: tags.minor.to_string(),
            pack: tags.pack.to_string(),
            build: tags.build.to_string(),
            ubr: revision().to_string(),
        }
    }

    fn get_time_info(&self) -> snapshot::TimeInfo {
        // 优先拿本地时间，拿不到就用 UTC 时间兜底
        let now =
            time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
        snapshot::TimeInfo {
            date: now.date(),
            time: now.time(),
            offset: now.offset(),
        }
    }

    pub fn generate_snapshot_config(&self) -> snapshot::Config {
        let exe_path = PathBuf::from(r"./").join(&self.snapshot.exe_path);
        // 获取 计算机名字
        let computer_name = hostname::get().unwrap();
        let backup_path = self.get_def_path().join("snapshot").join(computer_name);

        let system_info = self.get_system_info();
        // 获取 系统版本号
        let sys_name = "unknown";

        let time_info = self.get_time_info();
        let timer_format = time::macros::format_description!("[year]-[month]-[day]_[hour][minute]");
        let custom_time = time::OffsetDateTime::now_local()
            .unwrap()
            .format(timer_format)
            .unwrap();

        let backup_file_ext = "sna".to_string();
        let hash_file_ext = "hsh".to_string();

        let file_ext = snapshot::FileExt {
            backup: backup_file_ext,
            hash: hash_file_ext,
        };

        let dist_name = format!("{sys_name}_{custom_time}.sna");
        let hash_name = format!("{sys_name}_{custom_time}.hsh");
        let dist_path = backup_path.join(dist_name);
        let hash_path = backup_path.join(hash_name);

        let mut args: Vec<String> = Vec::with_capacity(15);

        args.extend([
            self.snapshot.source_dir.clone(),
            dist_path.to_string_lossy().to_string(),
            format!("-o{}", hash_path.to_string_lossy()),
            "-L0".to_string(),
            "--CreateDir".to_string(),
        ]);

        let limit_io_rate = (self.snapshot.limit_IO_rate != 0)
            .then(|| format!("--LimitIORate:{}", self.snapshot.limit_IO_rate));
        dbg!(&limit_io_rate);
        args.extend(limit_io_rate);

        macro_rules! push_flag {
            ($($field:expr => $flag:expr),* $(,)?) => {
                $(
                    if $field {
                        args.push($flag.to_string());
                    }
                )*
            };
        }
        push_flag!(
            self.snapshot.save_all_sectors => "-A",
            self.snapshot.disable_key      => "-W",
            self.snapshot.test             => "-T",
            self.snapshot.graph            => "-G",
            self.snapshot.clean_recycle    => "-R",
        );

        snapshot::Config {
            exe_path,
            backup_dir: backup_path,
            args,
            archived_number: self.snapshot.archived_number,
            system_info,
            time_info,
            file_ext,
        }
    }
}
