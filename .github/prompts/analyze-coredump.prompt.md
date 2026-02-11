---
agent: 'agent'
tools: ['search/codebase', 'terminal', 'search', 'search/usages']
description: 'Analyze coredumps from embedded ONVIF daemon crashes'
---

# Analyze Coredump

Your goal is to analyze a coredump from the ONVIF daemon and provide actionable debugging insights.

## Constraints

- **Maximum response**: 300 words
- **ONLY use**: `debugging/run_gdb_multiarch_analysis.sh` script
- **NEVER**: Run GDB directly
- **Working directory**: Must be project root

## Pre-Analysis Checklist

```bash
# Verify environment before analysis
test -f debugging/run_gdb_multiarch_analysis.sh
chmod +x debugging/run_gdb_multiarch_analysis.sh
ls -la debugging/coredumps/core.*
```

## Analysis Execution

```bash
# Navigate to project root
cd /home/kmk/anyka-dev

# Run analysis
./debugging/run_gdb_multiarch_analysis.sh [coredump_filename] onvifd_debug
```

## Focus Areas

1. **Stack Trace**: Exact crash location and call sequence
2. **Register Values**: Invalid pointers (0x32, 0x0, <0x1000)
3. **Memory Patterns**: Corruption, null pointers, stack overflow
4. **Function Parameters**: Argument values at crash point
5. **Thread Context**: Race conditions or thread-specific issues

## Common Error Patterns

- **Null pointer dereference**: Parameters showing 0x0 or 0x32
- **Stack overflow**: Unusual stack pointer values
- **Memory corruption**: Repeated or invalid memory patterns
- **Invalid function calls**: Wrong parameters
- **Threading issues**: Race conditions, deadlocks

## Required Output Format

```markdown
## CRASH ANALYSIS SUMMARY

**Coredump**: [filename]
**Signal**: [SIGSEGV/SIGABRT/etc.]
**Crash Location**: [function@address in file:line]
**Root Cause**: [1-sentence description]

**Critical Findings**:
- [Finding 1]
- [Finding 2]
- [Finding 3]

**Immediate Actions**:
1. [Action 1]
2. [Action 2]

**Confidence Level**: [High/Medium/Low]
```

## Prohibited Actions

- ❌ Running GDB directly
- ❌ Bypassing the analysis script
- ❌ Analysis longer than 300 words
- ❌ Working from incorrect directory
- ❌ Ignoring script failures
