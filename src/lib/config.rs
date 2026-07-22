use serde::Deserialize;

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use winreg::RegKey;
use winreg::enums::*;

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

    fn get_defaut_path(&self) -> PathBuf {
        if self.snapshot.dist_dir.is_empty() {
            PathBuf::from(r"\\")
                .join(&self.smb.server_ip)
                .join(&self.smb.share_name)
        } else {
            PathBuf::from(&self.snapshot.dist_dir)
        }
    }
    pub fn generate_smb_config(&self) -> smb::Config {
        let p = self.get_defaut_path();
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
        let hkcu = RegKey::predef(HKEY_LOCAL_MACHINE);
        let subkey = hkcu.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");

        if let Ok(key) = subkey {
            let major = key
                .get_value::<u32, _>("CurrentMajorVersionNumber")
                .ok()
                .map_or("unknown".to_string(), |val| val.to_string());
            let minor = key
                .get_value::<u32, _>("CurrentMinorVersionNumber")
                .ok()
                .map_or("unknown".to_string(), |val| val.to_string());
            let build = key.get_value("CurrentBuildNumber").unwrap_or_default();
            let ubr = key
                .get_value::<u32, _>("UBR")
                .ok()
                .map_or("unknown".to_string(), |val| val.to_string());
            let display_version = key.get_value("DisplayVersion").unwrap_or_default();
            return snapshot::SystemInfo {
                major,
                minor,
                build,
                ubr,
                display_version,
            };
        }

        snapshot::SystemInfo {
            major: "".to_string(),
            minor: "".to_string(),
            build: "".to_string(),
            ubr: "".to_string(),
            display_version: "".to_string(),
        }
    }

    pub fn generate_snapshot_config(&self) -> snapshot::Config {
        let exe_path = PathBuf::from(r"./").join(&self.snapshot.exe_path);
        // 获取 计算机名字
        let computer_name = hostname::get().unwrap();
        let path = self.get_defaut_path().join("snapshot").join(computer_name);

        let system_info = self.get_system_info();
        // 获取 系统版本号
        let mut sys_name = "unknown";
        let version = OsVersion::current();
        let ubr = revision().to_string();

        dbg!(ubr);
        // self.get_system_info();
        // 判断是否是 Windows 11 (Build 22000 及以上)
        if version >= OsVersion::new(10, 0, 0, 22000) {
            sys_name = "win11";
        } else if version >= OsVersion::new(10, 0, 0, 10240) {
            sys_name = "win10";
        }

        let timer_format = time::macros::format_description!("[year]-[month]-[day]_[hour][minute]");
        let custom_time = time::OffsetDateTime::now_local()
            .unwrap()
            .format(timer_format)
            .unwrap();
        let dist_name = format!("{sys_name}_{custom_time}.sna");
        let hash_name = format!("{sys_name}_{custom_time}.hsh");
        let dist_path = path.join(dist_name);
        let hash_path = path.join(hash_name);

        let mut args: Vec<String> = Vec::with_capacity(10);

        args.extend([
            self.snapshot.source_dir.clone(),
            dist_path.to_string_lossy().to_string(),
            "-o".to_string() + &hash_path.to_string_lossy(),
            "-L0".to_string(),
            "--CreateDir".into(),
        ]);

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
            args,
            archived_number: self.snapshot.archived_number,
            system_info,
        }
    }
}
