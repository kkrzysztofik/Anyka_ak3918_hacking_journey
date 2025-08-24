# Build Anyka cross-compilation Docker image
# Usage: .\docker-build.ps1

Write-Host "Building Anyka cross-compilation environment..." -ForegroundColor Green

try {
    docker build -t anyka-cross-compile .
    
    Write-Host "Docker image built successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor Yellow
    Write-Host "  To start interactive shell:" -ForegroundColor Cyan
    Write-Host "    docker run -it --rm -v `${PWD}:/workspace anyka-cross-compile" -ForegroundColor White
    Write-Host ""
    Write-Host "  To build ONVIF project:" -ForegroundColor Cyan  
    Write-Host "    docker run --rm -v `${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif" -ForegroundColor White
    Write-Host ""
    Write-Host "  To run any command:" -ForegroundColor Cyan
    Write-Host "    docker run --rm -v `${PWD}:/workspace anyka-cross-compile <command>" -ForegroundColor White
}
catch {
    Write-Host "Error building Docker image: $_" -ForegroundColor Red
    Write-Host "Make sure Docker is installed and running." -ForegroundColor Yellow
}
