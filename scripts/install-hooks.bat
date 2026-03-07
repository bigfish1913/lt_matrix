@echo off
REM Install Git hooks for ltmatrix (Windows)
REM
REM This script configures Git to use the hooks in .githooks/ directory
REM Run this script from the project root after cloning the repository
REM
REM Usage:
REM   scripts\install-hooks.bat

setlocal enabledelayedexpansion

echo === ltmatrix Git Hooks Installer ===
echo.

REM Get script directory and project root
set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."
cd /d "%PROJECT_ROOT%"
set "PROJECT_ROOT=%cd%"
set "GITHOOKS_DIR=%PROJECT_ROOT%\.githooks"

REM Check if we're in a Git repository
if not exist "%PROJECT_ROOT%\.git" (
    echo Error: Not a Git repository
    echo Please run this script from within the ltmatrix repository
    exit /b 1
)

REM Check if .githooks directory exists
if not exist "%GITHOOKS_DIR%" (
    echo Error: .githooks directory not found
    exit /b 1
)

REM Configure Git to use .githooks directory
echo Configuring Git to use .githooks directory...
git config core.hooksPath .githooks

REM Verify configuration
for /f "delims=" %%i in ('git config --get core.hooksPath') do set "HOOKS_PATH=%%i"

if "%HOOKS_PATH%"==".githooks" (
    echo.
    echo [92mGit hooks installed successfully![0m
    echo.
    echo Installed hooks:
    echo   * pre-commit  - Runs fmt check, clippy, and fast tests
    echo   * pre-push    - Runs full test suite and release build
    echo   * commit-msg  - Validates conventional commit format
    echo.
    echo To bypass hooks temporarily:
    echo   git commit --no-verify   # Skip pre-commit and commit-msg
    echo   git push --no-verify     # Skip pre-push
    echo.
    echo To uninstall hooks:
    echo   git config --unset core.hooksPath
) else (
    echo Failed to configure Git hooks
    exit /b 1
)

echo.
set /p "RUN_CHECKS=Run initial checks now? [y/N]: "
if /i "%RUN_CHECKS%"=="y" (
    echo.
    echo Running pre-commit checks...

    echo [1/3] Checking formatting...
    cargo fmt --check
    if !errorlevel! equ 0 (
        echo [92mFormatting OK[0m
    ) else (
        echo [91mFormatting issues found (run 'cargo fmt')[0m
    )

    echo [2/3] Running clippy...
    cargo clippy --all-targets --all-features -- -D warnings
    if !errorlevel! equ 0 (
        echo [92mClippy OK[0m
    ) else (
        echo [91mClippy issues found[0m
    )

    echo [3/3] Running tests...
    cargo test --lib -- --quiet
    if !errorlevel! equ 0 (
        echo [92mTests OK[0m
    ) else (
        echo [91mSome tests failed[0m
    )

    echo.
    echo Initial checks complete!
)

exit /b 0
