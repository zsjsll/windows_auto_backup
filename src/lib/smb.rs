use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Config {
    pub url: PathBuf,
    pub user: Arc<String>,
    pub passwd: Arc<String>,
}

impl From<Config> for Smb {
    fn from(config: Config) -> Self {
        Self { config: config }
    }
}

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Smb {
    config: Config,
}

impl Smb {
    #[instrument(err(Display), level = "debug")]
    pub fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config.url.to_string_lossy().starts_with(r"\\") {
            warn!("进行本地备份");
            warn!("如果需要 SMB 备份, 请配置 config.toml 中的 snapshot.dist_dir = \"\"");
            return Ok(());
        }

        info!("🚀 正在建立 SMB 认证通道");

        let output = Command::new(r"net")
            .arg("use")
            .arg(&self.config.url)
            .args(&[
                &format!(r"/user:{}", self.config.user.as_str()),
                self.config.passwd.as_str(),
                r"/persistent:no",
            ])
            .output()?;

        if output.status.success() {
            info!("✅ SMB 认证成功");
            Ok(())
        } else {
            let (err_msg, _, _) = encoding_rs::GBK.decode(&output.stderr);
            error!("❌ Windows SMB 认证失败");
            Err(err_msg.replace("\n", "").replace("\r", "").trim().into())
        }
    }

    /// 运维好习惯：断开与该远程服务器的所有隐式连接
    #[instrument(err(Display), level = "debug")]
    pub fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config.url.to_string_lossy().starts_with(r"\\") {
            return Ok(());
        }
        let output = Command::new("net")
            .arg("use")
            .arg(&self.config.url)
            .args(&[r"/delete", r"/y"])
            .output()?;

        if output.status.success() {
            info!("✅ 已断开 SMB 连接");
            Ok(())
        } else {
            let (err_msg, _, _) = encoding_rs::GBK.decode(&output.stderr);
            error!("❌ 断开 SMB 连接失败");
            Err(err_msg.replace("\n", "").replace("\r", "").trim().into())
        }
    }
}
