use std::{
    io::Write,
    sync::Arc,
};

use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{EnvFilter, Registry, fmt, prelude::*, reload};

/// 判断 stdout 是否应该输出 ANSI 颜色。
///
/// - 在 Windows 上：先探测 stdout 是否为控制台句柄，若是则尝试启用
///   Virtual Terminal Processing（VT）。老系统(如 Win7/8)或 conhost 未开启
///   VT 时 `SetConsoleMode` 会失败，此时返回 `false`，避免把 `[32m` 之类的
///   原始 ANSI 序列当作普通文本打印。
/// - 在非 Windows 或 stdout 被重定向时：返回 `false`，不输出颜色码。
pub fn stdout_ansi_enabled() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{
            ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, SetConsoleMode,
            STD_OUTPUT_HANDLE,
        };
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle.is_null() {
                return false;
            }
            let mut mode: u32 = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                // 不是控制台句柄（例如输出被重定向到文件/管道）
                return false;
            }
            // 尝试开启 VT 解析；系统不支持时返回 0 => false
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
        }
    }
    #[cfg(not(windows))]
    {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }
}

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
        writer.write_all(b"\n\n").ok();

        let timer_format =
            time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
        let custom_timer = fmt::time::LocalTime::new(timer_format);

        tracing_subscriber::registry()
            .with(reload_layer)
            .with(
                fmt::layer()
                    .with_timer(custom_timer.clone())
                    .pretty()
                    .with_ansi(stdout_ansi_enabled())
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

    pub fn update_logger_level(
        &self,
        config: &Config,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let new_filter = EnvFilter::new(config.log_level.as_str());

        self.log_handle.reload(new_filter)?;

        info!("根据配置文件, 加载日志级别为: {}", config.log_level);

        Ok(config.log_level.to_string())
    }
}
