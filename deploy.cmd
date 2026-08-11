cargo build --release
mkdir deploy
copy /Y target\release\rust_snapshot_backup.exe deploy\
copy /Y config.toml deploy\
xcopy /E /Y bin deploy\bin\