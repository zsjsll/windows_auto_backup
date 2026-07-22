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

    logs.update_logger_level(&cfg.load_logs());
    let smb: smb::Smb = cfg.load_smb().into();

    smb.connect()?;

    cfg.load_snapshot();

    let snapshot: snapshot::Snapshot = cfg.load_snapshot().into();

    // snapshot.show_config();
    snapshot.init_backup_dir()?;

    // snapshot.backup()?;

    smb.disconnect()?;

    Ok(())
}
