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

const CONFIG_PATH: &str = "config.toml";

/// 解析命令行参数：`-c <path>` 返回 `Some(path)`，否则返回 `None`（使用默认 `config.toml`）
fn parse_args() -> Option<String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "-c" {
            return args.next();
        }
    }
    None
}

fn main() {
    let logs = logs::Logs::new();

    // 无参数时使用默认 config.toml，有 -c 参数时使用指定路径
    let config_path = parse_args().unwrap_or_else(|| CONFIG_PATH.to_string());

    // 配置文件不存在时报错提醒
    if !std::path::Path::new(&config_path).exists() {
        error!("错误: 未找到配置文件: {}", config_path);
        std::process::exit(1);
    }

    let cfg = config::AppConfig::new(config_path).unwrap();

    logs.update_logger_level(&cfg.generate_logs_config());
    let smb: smb::Smb = cfg.generate_smb_config().into();

    smb.connect().unwrap();
    // cfg.generate_snapshot_config();

    let snapshot: snapshot::Snapshot = cfg.generate_snapshot_config().into();

    if let Ok(_) = snapshot.init_backup().inspect(|_| info!("初始化已完成")) {
        let _ = snapshot.start_backup().inspect(|ok| match ok {
            snapshot::SnapshotStatus::Ok(r) => info!("备份成功: {}", r),
            snapshot::SnapshotStatus::Err(e) => error!("备份失败: {}", e),
        });
    }

    smb.disconnect()
        .inspect(|_| info!("已断开 SMB 连接"))
        .inspect_err(|err| error!("断开 SMB 连接失败: {}", err))
        .ok();
}
