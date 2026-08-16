use std::ops::Deref;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Config {
    pub address: PathBuf,
    pub user: Arc<String>,
    pub passwd: Arc<String>,
}

#[cfg_attr(feature = "dbg", derive(Debug))]
pub struct Smb(Config);

impl From<Config> for Smb {
    fn from(config: Config) -> Self {
        Self(config)
    }
}

impl Deref for Smb {
    type Target = Config;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Smb {
    pub fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let add = self.address.to_string_lossy();
        if add.is_empty() {
            return Err("请检查配置文件, 地址为空, 无法备份".into());
        }
        if !add.starts_with(r"\\") {
            warn!("检查到为本地备份");
            return Ok(());
        }

        let output = Command::new("net")
            .arg("use")
            .arg(&self.address)
            .args(&[
                &format!(r"/user:{}", self.user.as_str()),
                self.passwd.as_str(),
                r"/persistent:no",
            ])
            .output()?;

        if output.status.success() {
            info!("SMB 认证成功");
            Ok(())
        } else {
            let (err_msg, _, _) = encoding_rs::GBK.decode(&output.stderr);
            let err_msg = err_msg.replace("\n", "").replace("\r", "");
            let err_msg = format!("Windows SMB 认证失败: {}", err_msg.trim());
            Err(err_msg.into())
        }
    }

    pub fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.address.to_string_lossy().starts_with(r"\\") {
            return Ok(());
        }
        let output = Command::new("net")
            .arg("use")
            .arg(&self.address)
            .args(&[r"/del", r"/y"])
            .output()?;

        if output.status.success() {
            info!("已断开 SMB 连接");
            Ok(())
        } else {
            let (err_msg, _, _) = encoding_rs::GBK.decode(&output.stderr);
            let err_msg = err_msg.replace("\n", "").replace("\r", "");
            let err_msg = format!("断开 SMB 连接失败: {}", err_msg.trim());
            Err(err_msg.into())
        }
    }
}
