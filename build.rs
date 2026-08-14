//! 构建脚本：检测并生成配置文件 config.toml
//!
//! 每次编译时：
//! 1. 若 config.toml 不存在，则从 config.toml.bak 模板复制生成，并给出 cargo:warning 提醒。
//! 2. 若 config.toml 已存在但 [smb].address 为空（或缺失、TOML 解析失败），
//!    则输出 warning 并 panic 强制中断构建，提醒填写配置后重新编译。

use std::env;
use std::fs;
use std::path::PathBuf;

const CONFIG_PATH: &str = "config.toml";
const BAK_PATH: &str = "config.toml.bak";

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let config_path = manifest_dir.join(CONFIG_PATH);
    let bak_path = manifest_dir.join(BAK_PATH);

    // 配置模板或配置发生变化时，重新运行本构建脚本
    println!("cargo:rerun-if-changed={}", BAK_PATH);
    println!("cargo:rerun-if-changed={}", CONFIG_PATH);

    // 1. config.toml 不存在 -> 从模板复制生成
    if !config_path.exists() {
        if bak_path.exists() {
            if let Err(err) = fs::copy(&bak_path, &config_path) {
                println!(
                    "cargo:warning=⚠️ 无法从 {} 生成 {}: {}",
                    BAK_PATH, CONFIG_PATH, err
                );
            } else {
                println!(
                    "cargo:warning=⚠️ 检测到 {} 不存在，已从模板 {} 生成，请检查 {} 中的配置",
                    CONFIG_PATH, BAK_PATH, CONFIG_PATH
                );
            }
        } else {
            println!(
                "cargo:warning=⚠️ 未找到 {}，且模板 {} 也不存在，请手动创建配置文件",
                CONFIG_PATH, BAK_PATH
            );
        }
    }

    // 2. 校验 [smb].address 是否已填写（匿名用户场景下 username/password 可为空）
    let content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(err) => {
            let msg = format!(
                "❌ 读取配置文件 {} 失败: {}。请检查后重新编译。",
                CONFIG_PATH, err
            );
            println!("cargo:warning={}", msg);
            panic!("{}", msg);
        }
    };

    let address: String = match toml::from_str::<toml::Value>(&content) {
        Ok(toml) => toml
            .get("smb")
            .and_then(|s| s.get("address"))
            .and_then(|a| a.as_str())
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        Err(err) => {
            let msg = format!(
                "❌ 解析配置文件 {} 失败: {}。请检查 TOML 语法后重新编译。",
                CONFIG_PATH, err
            );
            println!("cargo:warning={}", msg);
            panic!("{}", msg);
        }
    };

    if address.is_empty() {
        let msg = format!(
            "❌ 配置文件 {} 中的 [smb].address 为空，请编辑该文件填写 SMB 服务器地址后重新编译。",
            CONFIG_PATH
        );
        println!("cargo:warning={}", msg);
        panic!("{}", msg);
    }

    // address 非空，配置有效，正常编译
}
