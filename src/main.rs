#[macro_use]
extern crate tracing;

mod lib {
    pub mod config;

    pub mod files;
    pub mod logs;
    pub mod smb;
    pub mod snapshot;
}

use std::{fs, path::PathBuf, process};

use lib::{config, files, logs, smb, snapshot};

fn main() {
    let logs = logs::Logs::new();

    let config_dir = PathBuf::from("config");

    let default_config_path = config_dir.join("default.toml");
    let hostname = whoami::hostname().unwrap_or("unknown".into());
    let username = whoami::username().unwrap_or("unknown".into());
    let config_name = format!(r"{}-[{}].toml", hostname, username);
    let config_path = config_dir.join(config_name);

    let config_path = if default_config_path.is_file() {
        warn!("缺少专属配置文件: {}", &config_path.display());
        warn!("使用默认配置文件: {}", &default_config_path.display());
        default_config_path
    } else {
        process::exit(1);
    };

    let cfg = config::AppConfig::new(&config_path).unwrap();

    logs.update_logger_level(&cfg.generate_logs_config());
    let smb: smb::Smb = cfg.generate_smb_config().into();

    smb.connect().unwrap();
    // cfg.generate_snapshot_config();

    let snapshot: snapshot::Snapshot = cfg.generate_snapshot_config().into();

    if let Ok(_) = snapshot.init_backup().inspect(|_| info!("初始化已完成")) {
        let _ = snapshot.start_backup().inspect(|ok| match ok {
            snapshot::SnapshotStatus::Ok(r) => {
                info!("备份成功: {}", r);
                let config_bak_dir = config_dir.join("bak");
                fs::create_dir_all(&config_bak_dir).ok();
                let config_bak_path = config_bak_dir.join(config_dir).join(&config_path);
                fs::copy(config_path, config_bak_path)
                    .inspect(|_| info!("备份配置文件成功"))
                    .inspect_err(|e| warn!("备份配置文件失败: {}", e))
                    .ok();
            }
            snapshot::SnapshotStatus::Err(e) => error!("备份失败: {}", e),
        });
    }

    smb.disconnect()
        .inspect(|_| info!("已断开 SMB 连接"))
        .inspect_err(|err| error!("断开 SMB 连接失败: {}", err))
        .ok();
}
