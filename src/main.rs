#[macro_use]
extern crate tracing;

mod lib {
    pub mod config;
    // pub mod macros;
    pub mod files;
    pub mod logs;
    pub mod smb;
    pub mod snapshot;
}

use lib::{config, files, logs, smb, snapshot};

const CONFIG_PATH: &str = "config.toml";

fn main() {
    let logs = logs::Logs::new();

    let cfg = config::AppConfig::new(CONFIG_PATH).unwrap();

    logs.update_logger_level(&cfg.generate_logs_config());
    let smb: smb::Smb = cfg.generate_smb_config().into();

    smb.connect().unwrap();
    // cfg.generate_snapshot_config();

    let snapshot: snapshot::Snapshot = cfg.generate_snapshot_config().into();

    if let Ok(_) = snapshot.init_backup() {
        snapshot
            .start_backup()
            .inspect(|_| info!("备份完成"))
            .inspect_err(|err| error!("出错了: {}", err))
            .ok();
    };

    smb.disconnect()
        .inspect(|_| info!("已断开 SMB 连接"))
        .inspect_err(|err| error!("断开 SMB 连接失败: {}", err))
        .ok();
}
