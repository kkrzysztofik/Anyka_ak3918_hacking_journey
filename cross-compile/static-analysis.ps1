# Static Analysis Script for ONVIF Project
# This script provides easy access to static analysis tools in the Docker container

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("clang", "cppcheck", "snyk", "all")]
    [string]$Tool = "all",
    
    [Parameter(Mandatory=$false)]
    [string]$OutputDir = "analysis-results",
    
    [Parameter(Mandatory=$false)]
    [switch]$Verbose,
    
    [Parameter(Mandatory=$false)]
    [string]$SnykToken = $env:SNYK_TOKEN
)

# Docker image name
$DockerImage = "anyka-cross-compile"

# ONVIF source directory
$SourceDir = "onvif/src"

# Create output directory if it doesn't exist
if (!(Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    Write-Host "Created output directory: $OutputDir" -ForegroundColor Green
}

Write-Host "Starting static analysis for ONVIF project..." -ForegroundColor Cyan
Write-Host "Source directory: $SourceDir" -ForegroundColor Yellow
Write-Host "Output directory: $OutputDir" -ForegroundColor Yellow

# Function to run Clang Static Analyzer
function Invoke-ClangAnalyzer {
    Write-Host "`n=== Running Clang Static Analyzer ===" -ForegroundColor Magenta
    
    $ClangOutputDir = "$OutputDir/clang"
    if (!(Test-Path $ClangOutputDir)) {
        New-Item -ItemType Directory -Path $ClangOutputDir -Force | Out-Null
    }
    
    $ClangCmd = @(
        "docker", "run", "--rm", "-v", "`${PWD}:/workspace", $DockerImage,
        "bash", "-c",
        "cd /workspace/$SourceDir && find . -name '*.c' -exec clang --analyze -Xanalyzer -analyzer-output=html -Xanalyzer -analyzer-output-dir=/workspace/$ClangOutputDir {} \;"
    )
    
    if ($Verbose) {
        Write-Host "Command: $($ClangCmd -join ' ')" -ForegroundColor Gray
    }
    
    & $ClangCmd[0] $ClangCmd[1..($ClangCmd.Length-1)]
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Clang analysis completed. Results saved to: $ClangOutputDir" -ForegroundColor Green
        Write-Host "Open index.html in $ClangOutputDir to view results in browser" -ForegroundColor Yellow
    } else {
        Write-Host "Clang analysis failed with exit code: $LASTEXITCODE" -ForegroundColor Red
    }
}

# Function to run Cppcheck
function Invoke-Cppcheck {
    Write-Host "`n=== Running Cppcheck ===" -ForegroundColor Magenta
    
    $CppcheckOutputFile = "$OutputDir/cppcheck-results.xml"
    
    $CppcheckCmd = @(
        "docker", "run", "--rm", "-v", "`${PWD}:/workspace", $DockerImage,
        "bash", "-c",
        "cd /workspace && cppcheck --enable=all --xml --output-file=/workspace/$CppcheckOutputFile --suppress=missingIncludeSystem $SourceDir"
    )
    
    if ($Verbose) {
        Write-Host "Command: $($CppcheckCmd -join ' ')" -ForegroundColor Gray
    }
    
    & $CppcheckCmd[0] $CppcheckCmd[1..($CppcheckCmd.Length-1)]
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Cppcheck analysis completed. Results saved to: $CppcheckOutputFile" -ForegroundColor Green
        
        # Convert XML to HTML for better viewing
        $HtmlCmd = @(
            "docker", "run", "--rm", "-v", "`${PWD}:/workspace", $DockerImage,
            "bash", "-c",
            "cd /workspace && cppcheck-htmlreport --file=$CppcheckOutputFile --report-dir=$OutputDir/cppcheck-html --title='ONVIF Static Analysis'"
        )
        
        & $HtmlCmd[0] $HtmlCmd[1..($HtmlCmd.Length-1)]
        if ($LASTEXITCODE -eq 0) {
            Write-Host "HTML report generated at: $OutputDir/cppcheck-html/index.html" -ForegroundColor Yellow
        }
    } else {
        Write-Host "Cppcheck analysis failed with exit code: $LASTEXITCODE" -ForegroundColor Red
    }
}

# Function to run Snyk Code Analysis
function Invoke-SnykAnalysis {
    Write-Host "`n=== Running Snyk Code Analysis ===" -ForegroundColor Magenta
    
    $SnykJsonFile = "$OutputDir/snyk-results.json"
    $SnykSarifFile = "$OutputDir/snyk-results.sarif"
    
    # Check if Snyk token is provided
    if (-not $SnykToken) {
        Write-Host "Warning: No SNYK_TOKEN provided. Snyk will run in offline mode." -ForegroundColor Yellow
        Write-Host "For full functionality, set SNYK_TOKEN environment variable or use --SnykToken parameter" -ForegroundColor Yellow
    }
    
    # Build Docker command with environment variables
    $DockerEnv = @()
    if ($SnykToken) {
        $DockerEnv += @("-e", "SNYK_TOKEN=$SnykToken")
    }
    
    $SnykJsonCmd = @(
        "docker", "run", "--rm", "-v", "`${PWD}:/workspace"
    ) + $DockerEnv + @(
        $DockerImage,
        "bash", "-c",
        "cd /workspace && snyk code test --json --json-file-output=/workspace/$SnykJsonFile $SourceDir"
    )
    
    $SnykSarifCmd = @(
        "docker", "run", "--rm", "-v", "`${PWD}:/workspace"
    ) + $DockerEnv + @(
        $DockerImage,
        "bash", "-c",
        "cd /workspace && snyk code test --sarif --sarif-file-output=/workspace/$SnykSarifFile $SourceDir"
    )
    
    if ($Verbose) {
        Write-Host "JSON Command: $($SnykJsonCmd -join ' ')" -ForegroundColor Gray
        Write-Host "SARIF Command: $($SnykSarifCmd -join ' ')" -ForegroundColor Gray
    }
    
    # Run JSON output
    & $SnykJsonCmd[0] $SnykJsonCmd[1..($SnykJsonCmd.Length-1)]
    $JsonExitCode = $LASTEXITCODE
    
    # Run SARIF output
    & $SnykSarifCmd[0] $SnykSarifCmd[1..($SnykSarifCmd.Length-1)]
    $SarifExitCode = $LASTEXITCODE
    
    if ($JsonExitCode -eq 0 -or $JsonExitCode -eq 1) {
        Write-Host "Snyk JSON analysis completed. Results saved to: $SnykJsonFile" -ForegroundColor Green
    } else {
        Write-Host "Snyk JSON analysis failed with exit code: $JsonExitCode" -ForegroundColor Red
    }
    
    if ($SarifExitCode -eq 0 -or $SarifExitCode -eq 1) {
        Write-Host "Snyk SARIF analysis completed. Results saved to: $SnykSarifFile" -ForegroundColor Green
    } else {
        Write-Host "Snyk SARIF analysis failed with exit code: $SarifExitCode" -ForegroundColor Red
    }
    
    # Display summary if vulnerabilities found
    if ($JsonExitCode -eq 1 -or $SarifExitCode -eq 1) {
        Write-Host "`nSnyk found security vulnerabilities in your code!" -ForegroundColor Red
        Write-Host "Review the results files for detailed information:" -ForegroundColor Yellow
        Write-Host "  - JSON: $SnykJsonFile" -ForegroundColor White
        Write-Host "  - SARIF: $SnykSarifFile" -ForegroundColor White
    } elseif ($JsonExitCode -eq 0 -and $SarifExitCode -eq 0) {
        Write-Host "`nSnyk found no security vulnerabilities!" -ForegroundColor Green
    }
}

# Main execution logic
switch ($Tool) {
    "clang" {
        Invoke-ClangAnalyzer
    }
    "cppcheck" {
        Invoke-Cppcheck
    }
    "snyk" {
        Invoke-SnykAnalysis
    }
    "all" {
        Invoke-ClangAnalyzer
        Invoke-Cppcheck
        Invoke-SnykAnalysis
        
        Write-Host "`n=== Analysis Summary ===" -ForegroundColor Cyan
        Write-Host "All static analysis tools completed!" -ForegroundColor Green
        Write-Host "Results saved in: $OutputDir" -ForegroundColor Yellow
        Write-Host "`nTo view results:" -ForegroundColor White
        Write-Host "  - Clang: Open $OutputDir/clang/index.html in browser" -ForegroundColor White
        Write-Host "  - Cppcheck: Open $OutputDir/cppcheck-html/index.html in browser" -ForegroundColor White
        Write-Host "  - Snyk: Review $OutputDir/snyk-results.json or $OutputDir/snyk-results.sarif" -ForegroundColor White
    }
}

Write-Host "`nStatic analysis completed!" -ForegroundColor Green

