@echo off
REM Generate man pages for distribution (Windows)

echo Generating man pages for ltmatrix...

REM Create man directory
if not exist target\man mkdir target\man

REM Generate man pages using the ltmatrix binary
cargo build --release --bin ltmatrix
target\release\ltmatrix.exe man --output target\man

echo Man pages generated successfully in target\man/
echo.
echo To view a man page, use a man page viewer on Linux/macOS,
echo or view the file directly in a text editor.
echo.
echo Example: target\man\ltmatrix.1
