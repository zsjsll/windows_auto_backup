# windows_auto_backup

调用 snapshot64.exe 自动备份系统

## 调试

1. `cargo build` 会根据 `config.toml.bak` 在根目录下生成 `config/default.toml`

2. 运行程序 会根据 `config/default.toml` 生成文件 `config/{主机名}-{[用户名]}.toml` 配置文件, 程序根据此文件运行

3. 运行成功后会生成 `config/bak/config/{主机名}-{[用户名]}.toml` 进行备份

## 编译

直接运行 vscode 的tasks 所有需要文件 打包到 `deploy/`



双击运行 编译后的二进制文件 rust_snapshot_backup.exe 会显示  [32m、[0m、[1;32m 的字符
但是 在ide中 运行 就是正常的颜色
我需要解决这个问题