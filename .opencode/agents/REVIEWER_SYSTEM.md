# Multi-Model Consensus Review System

## Overview

This project uses a **multi-model consensus review system** to ensure code quality through diverse AI perspectives. Three specialized reviewers analyze code independently, and their findings are synthesized via majority consensus.

## Architecture

```
User Request
    ↓
orchestrator (you)
    ↓
reviewer (main entry point)
    ↓
reviewer-consensus (orchestrator)
    ├─→ reviewer-memory (Sonnet 4.5) ────┐
    ├─→ reviewer-architecture (GPT-5.2) ─┤ Parallel
    └─→ reviewer-security (Opus 4-6) ────┘
    ↓
Consensus Synthesis (2/3 majority)
    ↓
Unified Report
```

## The Review Team

### 1. reviewer-memory (Claude Sonnet 4.5)
**Expertise:** Rust memory safety, ownership, lifetimes

**Focuses on:**
- Unsafe code justification
- Ownership patterns
- Lifetime correctness
- Send/Sync bounds
- Resource cleanup
- Data race risks

**Strengths:**
- Deep Rust understanding
- Lifetime analysis
- Borrow checker reasoning
- Memory leak detection

---

### 2. reviewer-architecture (GPT-5.2)
**Expertise:** Architecture patterns, API design, system integration

**Focuses on:**
- Pattern consistency with onvif-rust
- API ergonomics
- Config propagation
- Service lifecycle
- Module organization
- Breaking changes

**Strengths:**
- System-level thinking
- Pattern recognition
- Integration analysis
- API design evaluation

---

### 3. reviewer-security (Claude Opus 4-6)
**Expertise:** Security vulnerabilities, DoS vectors, edge cases

**Focuses on:**
- Authentication bypass
- Credential leaks
- Timing attacks
- Resource exhaustion
- Input validation
- Concurrent access races

**Strengths:**
- Threat modeling
- Attack vector identification
- Embedded security constraints
- Fault injection analysis

## Consensus Rules

### Issue Classification

| Consensus | Meaning | Action |
|-----------|---------|--------|
| **3/3 (Unanimous)** | All models agree it's critical | **MUST FIX** before merge |
| **2/3 (Majority)** | Two models flag the issue | **SHOULD FIX** (likely real) |
| **1/3 (Minority)** | One model flags it | **CONSIDER** (expertise-specific) |

### Verdict Synthesis

| Individual Verdicts | Final Verdict |
|---------------------|---------------|
| 2+ REJECT | **REQUEST CHANGES** |
| 1 REJECT, 2 CONDITIONAL | **CONDITIONAL APPROVAL** |
| All APPROVE/CONDITIONAL | **APPROVE** |

## Usage

### For Users (Simple)

Just invoke the reviewer:

```
Task(subagent_type="reviewer", prompt="Review Phase 3 changes...")
```

The reviewer automatically:
1. Invokes reviewer-consensus
2. Which dispatches to all 3 specialists
3. Synthesizes their findings
4. Returns unified report

### For Orchestrator (Advanced)

Direct consensus invocation:

```
Task(
  subagent_type="reviewer-consensus",
  description="Multi-model consensus review",
  prompt="Review [changes] for [purpose]..."
)
```

## Example Output

```markdown
# Multi-Model Consensus Review Report

**Models:** Sonnet 4.5, GPT-5.2, Opus 4-6
**Consensus:** 2/3 majority

## Critical Issues (Unanimous: 3/3)

**Issue 1: Frame unsafe Send/Sync**
| Model | Finding |
|-------|---------|
| Sonnet 4.5 | ❌ UAF risk - raw pointer marked Send |
| GPT-5.2 | ❌ Memory safety violation |
| Opus 4-6 | ❌ Exploitable dangling pointer |

**Consensus:** MUST FIX

## Major Issues (Majority: 2/3)

**Issue 2: Config not wired**
| Model | Finding |
|-------|---------|
| Sonnet 4.5 | ⚠️ Ownership unclear |
| GPT-5.2 | ❌ Architecture violation |
| Opus 4-6 | ✅ Not a security issue |

**Consensus:** SHOULD FIX

## Final Verdict

**Status:** REQUEST CHANGES
**Required:** 2 critical issues to fix
```

## Why Multi-Model?

### Single-Model Limitations

- Blind spots in specific domains
- Model biases
- Missed edge cases
- Inconsistent severity assessment

### Multi-Model Benefits

- **Diverse perspectives:** Each model has different strengths
- **Reduced false positives:** Consensus filters noise
- **Better coverage:** 3 models catch more than 1
- **Confidence indicators:** Unanimous issues are real problems

### Empirical Results

In Phase 3 testing:
- **Single-model (GPT-5.2):** Found 3 critical issues
- **Multi-model (3 models):** Found 8 critical issues (167% more)

Breakdown:
- Memory issues: Caught by Sonnet 4.5
- Architecture breaks: Caught by GPT-5.2
- Security vulns: Caught by Opus 4-6

## When to Use Multi-Model

### Always Use (Default)

- Code changes (any size)
- Security-sensitive code
- Architecture changes
- New features
- Bug fixes

### Single-Model OK (Rare)

- Documentation-only changes
- Formatting-only changes
- Config file updates (non-code)

## Performance

- **Latency:** 3 models in parallel ≈ single-model time
- **Quality:** Significantly higher issue detection
- **False positives:** Lower (filtered by consensus)

## Integration with Workflow

```
Implementation
    ↓
coder-rust/coder-typescript/coder-c
    ↓
qa-engineer-rust/qa-engineer-www
    ↓
reviewer (AUTO-INVOKES MULTI-MODEL) ← You are here
    ↓
Fix issues if found
    ↓
Re-review until approved
    ↓
Merge
```

## File Locations

```
.opencode/agents/
├── reviewer.md                    # Main entry (auto-invokes consensus)
├── reviewer-consensus.md          # Orchestrator
└── reviewers/
    ├── reviewer-memory.md         # Sonnet 4.5 - Memory safety
    ├── reviewer-architecture.md   # GPT-5.2 - Architecture  
    └── reviewer-security.md       # Opus 4-6 - Security
```

## Future Enhancements

Potential improvements:
- [ ] Weighted voting (expertise-based)
- [ ] Confidence scoring per finding
- [ ] Historical false-positive tracking
- [ ] Per-issue model selection
- [ ] Custom consensus thresholds (3/3 for critical paths)
