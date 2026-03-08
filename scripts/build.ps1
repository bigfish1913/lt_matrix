# ltmatrix Build Script for PowerShell
# Run this script from PowerShell: .\scripts\build.ps1

Write-Host "Building ltmatrix workspace..." -ForegroundColor Cyan

# Build workspace
Write-Host "Building all crates..." -ForegroundColor Yellow
cargo build --workspace

if ($LASTEXITCODE -eq 0) {
    Write-Host "Build successful!" -ForegroundColor Green

    # Run tests
    Write-Host "Running tests..." -ForegroundColor Yellow
    cargo test --workspace -- --test-threads=1

    if ($LASTEXITCODE -eq 0) {
        Write-Host "All tests passed!" -ForegroundColor Green
    } else {
        Write-Host "Some tests failed." -ForegroundColor Red
    }
} else {
    Write-Host "Build failed!" -ForegroundColor Red
}
