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

### 3. cargo audit (Rust dependency advisories)

Checks both `Cargo.lock` files against the [RustSec advisory database](https://rustsec.org).
Runs in CI on every push; no account or token required.

```bash
cargo install cargo-audit --locked
(cd cross-compile && cargo audit)
(cd validation/rust && cargo audit)
```

Vulnerabilities fail the build. Informational advisories — `unmaintained`,
`unsound`, `yanked` — are reported as warnings and do not.

## Usage Methods

### Method 1: PowerShell Script (Recommended)

Use the provided PowerShell script for easy analysis:

```powershell
# Run all static analysis tools
.\static-analysis.ps1

# Run specific tool
.\static-analysis.ps1 -Tool clang
.\static-analysis.ps1 -Tool cppcheck

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
└── cppcheck-html/           # Cppcheck HTML report
    └── index.html
```

`cargo audit` output goes to the CI job log and PR summary, not this directory.

## Viewing Results

### Clang Static Analyzer

- Open `analysis-results/clang/index.html` in your browser
- Shows detailed analysis paths with source code highlighting
- Click on issues to see the execution path that leads to the problem

### Cppcheck

- Open `analysis-results/cppcheck-html/index.html` in your browser
- Shows categorized issues with severity levels
- Includes suggestions for fixes

### cargo audit

- Output appears directly in the CI job log and the PR summary comment
- Failures list the RustSec advisory ID, affected crate/version, and a link to the advisory
- No local report file — re-run `cargo audit` locally for the same output

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

Cppcheck rule severity is configured on the command line (e.g.
`--suppress=missingIncludeSystem`, see above). `cargo audit` has no
per-project exclusion file in this repo; every RustSec vulnerability fails
the build by design.

### Suppressing False Positives

Add comments to suppress specific warnings:

```c
// cppcheck-suppress nullPointer
if (ptr == NULL) return;
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
- [RustSec Advisory Database](https://rustsec.org)
- ONVIF Project Coding Standards (see project documentation)

## See Also

- [[Development-Guide]] - Development workflow and code quality standards
- [[ONVIF-Rust-Implementation]] - Rust project with built-in quality tools
