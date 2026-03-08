@echo off
set PATH=C:\Windows\System32;C:\Windows;C:\Users\bigfish\scoop\apps\rustup\current\.cargo\bin;%PATH%
cd /d c:\Users\bigfish\Projects\github.com\bigfish1913\lt_matrix
cargo build --workspace 2>&1
