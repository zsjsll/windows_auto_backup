#[macro_use]
extern crate tracing;

mod lib {
    pub mod config;
    // pub mod macros;
    pub mod logs;
    pub mod smb;
    pub mod snapshot;
    pub mod files;
}

use lib::{config, logs, smb, snapshot,files};

const CONFIG_PATH: &str = "config.toml";

fn main() {
    let logs = logs::Logs::new();

    let cfg = config::AppConfig::new(CONFIG_PATH).unwrap();

    logs.update_logger_level(&cfg.generate_logs_config());
    let smb: smb::Smb = cfg.generate_smb_config().into();

    smb.connect().unwrap();

    cfg.generate_snapshot_config();

    let snapshot: snapshot::Snapshot = cfg.generate_snapshot_config().into();

    let _ = snapshot.backup().ok();

    smb.disconnect().ok();
}
