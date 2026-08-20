//! 应用配置：从命令行参数与环境变量解析。

use std::env;

/// 默认服务端地址（对应项目内的 Python SSE 测试服务）。
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8000";

/// 应用运行配置。
#[derive(Debug, Clone)]
pub struct Config {
    /// SSE 服务端根地址。
    pub server_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
        }
    }
}

impl Config {
    /// 从环境变量 `NOTICE_SERVER_URL` 与命令行参数解析配置。
    ///
    /// 优先级：命令行 `--url <url>` / `-u <url>` > 环境变量 > 默认值。
    pub fn from_env() -> Self {
        let mut config = Self {
            server_url: env::var("NOTICE_SERVER_URL")
                .unwrap_or_else(|_| DEFAULT_SERVER_URL.to_string()),
        };

        let mut args = env::args().skip(1).peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--url" | "-u" => {
                    if let Some(url) = args.next() {
                        config.server_url = url;
                    }
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {}
            }
        }

        config.server_url = config.server_url.trim_end_matches('/').to_string();
        config
    }
}

fn print_help() {
    println!("my-notice-app - 基于 GPUI 的桌面通知盒");
    println!();
    println!("用法: my-notice-app [选项]");
    println!();
    println!("选项:");
    println!("  -u, --url <URL>   SSE 服务端根地址 (默认: {DEFAULT_SERVER_URL})");
    println!("  -h, --help        显示帮助");
    println!();
    println!("环境变量: NOTICE_SERVER_URL 等价于 --url");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_default_url() {
        let config = Config::default();
        assert_eq!(config.server_url, DEFAULT_SERVER_URL);
    }
}
