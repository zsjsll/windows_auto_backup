#[macro_use]
extern crate tracing;

mod lib {
    pub mod config;

    pub mod files;
    pub mod logs;
    pub mod smb;
    pub mod snapshot;
}

use lib::{config, files, logs, smb, snapshot};

use std::{
    fs, io,
    path::{Path, PathBuf},
};

fn create_config_file(template_path: &Path, destination_dir: &Path) -> Result<PathBuf, io::Error> {
    let hostname = whoami::hostname().unwrap_or("unknown".into());
    let username = whoami::username().unwrap_or("unknown".into());
    let config_name = format!(r"{}-[{}].toml", hostname, username);
    let config_path = destination_dir.join(config_name);

    if !config_path.exists() {
        warn!("缺少专属配置文件: {}", &config_path.display());
        warn!("使用默认配置文件: {} 进行创建", &template_path.display());
        fs::copy(&template_path, &config_path)?;
    }
    Ok(config_path)
}

fn backup_config(
    config_path: &Path,
    destination_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_bak_dir = destination_dir.join("bak");
    fs::create_dir_all(&config_bak_dir).ok();
    let config_file_name = config_path
        .file_name()
        .ok_or_else(|| format!("无法找到文件名字: {}", config_path.display()))?;

    let config_bak_path = config_bak_dir.join(config_file_name);

    fs::copy(config_path, config_bak_path)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logs = logs::Logs::new();

    let config_dir = PathBuf::from("config");
    let template_config_path = config_dir.join(r"@template.toml");

    let config_path = create_config_file(&template_config_path, &config_dir)?;

    info!(
        "正在加载自定义 TOML 配置文件, 路径: {}",
        &config_path.display()
    );
    let cfg = config::AppConfig::new(&config_path)?;

    logs.update_logger_level(&cfg.generate_logs_config())
        .inspect_err(|e| warn!("请检查配置文件, 加载日志级别失败: {}", e))
        .ok();

    let smb: smb::Smb = cfg.generate_smb_config().into();
    info!("正在建立 SMB 认证通道");
    smb.connect().inspect_err(|e| error!("{}", e))?;

    let snapshot: snapshot::Snapshot = cfg.generate_snapshot_config().into();
    info!("开始备份目录初始化");

    if let Ok(_) = snapshot.init_backup().inspect_err(|e| error!("{e}")) {
        if let Ok(result) = snapshot.start_backup().inspect_err(|e| error!("{e}")) {
            match result {
                snapshot::SnapshotStatus::Ok(ok) => {
                    info!("备份成功: {}", ok);
                    backup_config(&config_path, &config_dir)
                        .inspect(|_| info!("备份配置文件成功"))
                        .inspect_err(|e| warn!("备份配置文件失败: {}", e))
                        .ok();
                }
                snapshot::SnapshotStatus::Err(e) => error!("备份失败: {}", e),
            }
        }
    }

    smb.disconnect().inspect_err(|e| error!("{e}"))?;

    Ok(())
}
