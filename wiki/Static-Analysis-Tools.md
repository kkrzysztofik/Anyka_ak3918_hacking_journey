# Static Analysis Tools

The project includes comprehensive static analysis tools integrated into the Anyka cross-compile environment to ensure code quality, security, and reliability.

## Available Tools

### 1. Clang Static Analyzer

- **Purpose**: Advanced symbolic execution and path-sensitive analysis
- **Detects**: Memory leaks, buffer overflows, null pointer dereferences, uninitialized variables
- **Output**: HTML reports with detailed analysis paths

### 2. Cppcheck

- **Purpose**: Bug detection and code quality analysis
- **Detects**: Undefined behavior, memory leaks, buffer overflows, style issues
- **Output**: XML and HTML reports

### 3. Snyk Code

- **Purpose**: Advanced security vulnerability scanning and code analysis
- **Detects**: Security vulnerabilities, code quality issues, dependency vulnerabilities
- **Output**: JSON and SARIF reports
- **Authentication**: Requires SNYK_TOKEN for full functionality

## Usage Methods

### Method 1: PowerShell Script (Recommended)

Use the provided PowerShell script for easy analysis:

```powershell
# Run all static analysis tools
.\static-analysis.ps1

# Run specific tool
.\static-analysis.ps1 -Tool clang
.\static-analysis.ps1 -Tool cppcheck
.\static-analysis.ps1 -Tool snyk

# Run with Snyk authentication token
.\static-analysis.ps1 -Tool snyk -SnykToken "your-api-token-here"

# Verbose output
.\static-analysis.ps1 -Verbose

# Custom output directory
.\static-analysis.ps1 -OutputDir "my-analysis-results"
```

**Note**: For the ONVIF Rust project, use Rust's built-in tools:

- `cargo clippy -- -D warnings` for linting
- `cargo fmt --check` for formatting
- `cargo test` for testing

## Output Files

After running analysis, results are saved in the `analysis-results/` directory:

```text
analysis-results/
├── clang/                    # Clang Static Analyzer results
│   └── index.html           # Main HTML report
├── cppcheck-results.xml     # Cppcheck XML output
├── cppcheck-html/           # Cppcheck HTML report
│   └── index.html
├── snyk-results.json        # Snyk JSON output
└── snyk-results.sarif       # Snyk SARIF output
```

## Viewing Results

### Clang Static Analyzer

- Open `analysis-results/clang/index.html` in your browser
- Shows detailed analysis paths with source code highlighting
- Click on issues to see the execution path that leads to the problem

### Cppcheck

- Open `analysis-results/cppcheck-html/index.html` in your browser
- Shows categorized issues with severity levels
- Includes suggestions for fixes

### Snyk Code

- Open `analysis-results/snyk-results.json` for programmatic analysis
- Open `analysis-results/snyk-results.sarif` for SARIF-compatible tools
- Shows security vulnerabilities with detailed remediation guidance
- Includes severity levels and CVE information when available

## Snyk Authentication Setup

### Getting Your Snyk API Token

1. **Create a free Snyk account**: Visit <https://app.snyk.io/>
2. **Navigate to Account Settings**:
   - Click on your profile → Account Settings
   - Go to General Settings → API Token
3. **Copy your token**: Click "click to show" to reveal your API token
4. **Store securely**: Never commit this token to source control

### Authentication Methods

#### Method 1: Environment Variable (Recommended)

```powershell
# Set environment variable for current session
$env:SNYK_TOKEN = "your-api-token-here"

# Or set permanently (Windows)
[Environment]::SetEnvironmentVariable("SNYK_TOKEN", "your-api-token-here", "User")
```

#### Method 2: Script Parameter

```powershell
# Pass token directly to script
.\static-analysis.ps1 -Tool snyk -SnykToken "your-api-token-here"
```

### Authentication Behavior

- **With Token**: Full Snyk functionality, cloud-based analysis, latest vulnerability database
- **Without Token**: Limited offline mode, basic analysis only
- **Script Warnings**: The script will warn you if no token is provided

## Integration with Development Workflow

### Pre-commit Analysis

Add to your development workflow:

```powershell
# Before committing changes
.\static-analysis.ps1 -Tool all
# Review results and fix issues before committing
```

To install repository-provided git hooks that run local validations (e.g., Rust `cargo fmt` for `onvif-rust`), run:

```bash
scripts/install-git-hooks.sh
```

To revert this change run:

```bash
git config --unset core.hooksPath
```

### CI/CD Integration

The tools can be integrated into CI/CD pipelines:

```yaml
# For ONVIF Rust project
- name: Run Rust Linting
  run: |
    cd cross-compile/onvif-rust
    cargo clippy -- -D warnings
    cargo fmt --check
```

### IDE Integration

- **VS Code**: Install C/C++ extension for real-time analysis
- **CLion**: Built-in static analysis support
- **Vim/Neovim**: Use ALE or coc.nvim with clangd

## Common Issues and Solutions

### Missing Include Files

If you see "missing include" warnings:

- These are often false positives for system headers
- Use `--suppress=missingIncludeSystem` flag for cppcheck
- The tools focus on your source code, not system dependencies

### Memory Analysis

For embedded systems like Anyka AK3918:

- Pay attention to memory leak warnings
- Check for buffer overflow vulnerabilities
- Verify proper resource cleanup

### Security Analysis

Focus on:

- Input validation issues
- Buffer overflow vulnerabilities
- Unsafe string operations
- Authentication and authorization flaws

## Customization

### Adding Custom Rules

Snyk uses built-in security rules, but you can configure severity thresholds and exclusions:

```bash
# Set severity threshold
snyk code test --severity-threshold=medium

# Exclude specific files or directories
snyk code test --exclude=test/,vendor/
```

### Suppressing False Positives

Add comments to suppress specific warnings:

```c
// cppcheck-suppress nullPointer
if (ptr == NULL) return;

// snyk: disable-next-line
strcpy(dest, src);  // Known safe in this context
```

## Best Practices

1. **Run analysis regularly** - Integrate into your development workflow
2. **Fix high-severity issues first** - Address security and memory issues immediately
3. **Review false positives** - Understand why tools flag certain code
4. **Use multiple tools** - Each tool has different strengths
5. **Keep tools updated** - Use latest versions for better analysis

## Static Analysis Troubleshooting

### Permission Issues

```powershell
# Fix file permissions if needed
Get-ChildItem -Path "analysis-results" -Recurse | ForEach-Object { $_.Attributes = "Normal" }
```

### Memory Issues

```powershell
# Run analysis on smaller subsets if memory is limited
.\static-analysis.ps1 -Tool cppcheck  # Start with cppcheck (lightest)
```

## Further Reading

- [Clang Static Analyzer Documentation](https://clang.llvm.org/docs/analyzer/)
- [Cppcheck Manual](http://cppcheck.sourceforge.net/manual.pdf)
- [Snyk Documentation](https://docs.snyk.io/)
- ONVIF Project Coding Standards (see project documentation)

## See Also

- [[Development-Guide]] - Development workflow and code quality standards
- [[ONVIF-Rust-Implementation]] - Rust project with built-in quality tools
