#[macro_use]
extern crate tracing;

mod lib {
    pub mod config;
    // pub mod macros;
    pub mod logs;
    pub mod smb;
    pub mod snapshot;
}

use lib::{config, logs, smb, snapshot};

const CONFIG_PATH: &str = "config.toml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logs = logs::Logs::new();

    let cfg = config::AppConfig::new(CONFIG_PATH)?;

    logs.update_logger_level(&cfg.generate_logs_config());
    let smb: smb::Smb = cfg.generate_smb_config().into();

    smb.connect()?;

    cfg.generate_snapshot_config();

    let snapshot: snapshot::Snapshot = cfg.generate_snapshot_config().into();

    // snapshot.show_config();
    snapshot.init_backup_dir()?;

    snapshot.backup()?;

    smb.disconnect()?;

    Ok(())
}
