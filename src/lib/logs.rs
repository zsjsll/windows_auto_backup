use std::{io::Write, sync::Arc};

use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{EnvFilter, Registry, fmt, prelude::*, reload};

pub struct Logs {
    log_handle: reload::Handle<EnvFilter, Registry>,
    _file_guard: WorkerGuard,
}

pub struct Config {
    pub log_level: Arc<String>,
}

impl Logs {
    pub fn new() -> Self {
        let filter = EnvFilter::new("info"); // 👈 默认先给 info 级别

        // 🌟 核心魔法：用 reload::layer 把过滤器包装起来
        let (reload_layer, handle) = reload::Layer::new(filter);
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY) // DAILY 代表按天滚动（对标你原先的 daily 函数）
            .max_log_files(10) // 🎯 核心大招：死死锁住，全宇宙最多只留最近 10 个日志文件！
            .filename_suffix("log") // 日志后缀
            .build("./logs") // 📂 存放的目录
            .expect("初始化日志目录翻车");

        let (non_blocking, file_guard) = tracing_appender::non_blocking(file_appender);

        let mut writer = non_blocking.clone();
        let _ = writer.write_all(b"\n\n");

        let timer_format =
            time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
        let custom_timer = fmt::time::LocalTime::new(timer_format);

        tracing_subscriber::registry()
            .with(reload_layer)
            .with(
                fmt::layer()
                    .with_timer(custom_timer.clone())
                    .pretty()
                    .with_writer(std::io::stdout),
            ) // 刷到你的 CLI 黑色窗口
            .with(
                fmt::layer()
                    .compact()
                    .with_timer(custom_timer.clone())
                    .with_target(false)
                    .with_ansi(false)
                    .with_writer(non_blocking),
            )
            .init();
        info!("日志系统冷启动成功，暂定默认级别: INFO");

        Self {
            log_handle: handle,
            _file_guard: file_guard,
        }
    }

    pub fn update_logger_level(&self, config: &Config) {
        let new_filter = EnvFilter::new(&*config.log_level);

        // 🌟 绝杀：拉动电闸，原地在线修改全局生效的日志等级！
        if self.log_handle.reload(new_filter).is_ok() {
            info!(
                "日志等级已动态同步为 config.toml 配置: {}",
                config.log_level.as_str()
            );
        };
    }
}
