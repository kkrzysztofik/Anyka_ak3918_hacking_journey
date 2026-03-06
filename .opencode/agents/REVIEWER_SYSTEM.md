# Multi-Model Consensus Review System

## Overview

This project uses a **4-model consensus review system** to ensure code quality through diverse AI perspectives. Four specialized reviewers analyze code independently, and their findings are synthesized via majority consensus.

## Architecture

```
User Request
    ↓
orchestrator (you)
    ↓
reviewer (main entry point)
    ↓
reviewer-consensus (orchestrator)
    ├─→ reviewer-memory (Sonnet 4.5) ────────┐
    ├─→ reviewer-architecture (gpt-5.4) ─────┤
    ├─→ reviewer-security (Opus 4-6) ────────┤ Parallel
    └─→ reviewer-testing (Gemini 3.1 Pro) ───┘
    ↓
Consensus Synthesis (2/4 majority, 3/4 strong, 4/4 critical)
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

### 2. reviewer-architecture (gpt-5.4)
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

---

### 4. reviewer-testing (Google Gemini 3.1 Pro)
**Expertise:** Test coverage, quality assurance, code correctness

**Focuses on:**
- Test coverage completeness
- Edge case testing
- Error path testing
- Mock usage correctness
- Logic errors and bugs
- Code correctness verification

**Strengths:**
- QA perspective
- Test gap identification
- Logic error detection
- Boundary condition analysis

## Consensus Rules

### Issue Classification

| Consensus | Meaning | Action |
|-----------|---------|--------|
| **4/4 (Unanimous)** | All 4 models agree it's critical | **CRITICAL - MUST FIX IMMEDIATELY** |
| **3/4 (Strong Majority)** | Three models flag the issue | **MUST FIX** (very likely real) |
| **2/4 (Majority)** | Two models flag the issue | **SHOULD FIX** (likely real) |
| **1/4 (Minority)** | One model flags it | **CONSIDER** (expertise-specific) |

### Verdict Synthesis

| Individual Verdicts | Final Verdict |
|---------------------|---------------|
| 3+ REJECT | **REQUEST CHANGES** (critical) |
| 2 REJECT | **REQUEST CHANGES** (significant) |
| 1 REJECT, rest CONDITIONAL | **CONDITIONAL APPROVAL** |
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

**Models:** Sonnet 4.5, gpt-5.4, Opus 4-6, Gemini 3.1 Pro
**Consensus:** 2/4 majority, 3/4 strong, 4/4 critical

## Critical Issues (Unanimous: 4/4)

**Issue 1: Frame unsafe Send/Sync**
| Model | Finding |
|-------|---------|
| Sonnet 4.5 | ❌ UAF risk - raw pointer marked Send |
| gpt-5.4 | ❌ Memory safety violation |
| Opus 4-6 | ❌ Exploitable dangling pointer |
| Gemini 3.1 Pro | ❌ Untested unsafe code |

**Consensus:** CRITICAL - MUST FIX IMMEDIATELY

## Must Fix Issues (Strong Majority: 3/4)

**Issue 2: Config not wired**
| Model | Finding |
|-------|---------|
| Sonnet 4.5 | ⚠️ Ownership unclear |
| gpt-5.4 | ❌ Architecture violation |
| Opus 4-6 | ✅ Not a security issue |
| Gemini 3.1 Pro | ❌ Untested config struct |

**Consensus:** MUST FIX

## Should Fix Issues (Majority: 2/4)

**Issue 3: Missing edge case tests**
| Model | Finding |
|-------|---------|
| Sonnet 4.5 | ✅ Memory safe |
| gpt-5.4 | ✅ Architecture OK |
| Opus 4-6 | ⚠️ Boundary conditions risky |
| Gemini 3.1 Pro | ❌ No boundary tests |

**Consensus:** SHOULD FIX

## Final Verdict

**Status:** REQUEST CHANGES
**Required:** 1 critical + 2 must-fix issues
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
- **Single-model (gpt-5.4):** Found 3 critical issues
- **Multi-model (3 models):** Found 8 critical issues (167% more)

Breakdown:
- Memory issues: Caught by Sonnet 4.5
- Architecture breaks: Caught by gpt-5.4
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
    ├── reviewer-architecture.md   # gpt-5.4 - Architecture  
    ├── reviewer-security.md       # Opus 4-6 - Security
    └── reviewer-testing.md        # Gemini 3.1 Pro - Testing/QA
```

## Future Enhancements

Potential improvements:
- [ ] Weighted voting (expertise-based)
- [ ] Confidence scoring per finding
- [ ] Historical false-positive tracking
- [ ] Per-issue model selection
- [ ] Custom consensus thresholds (3/3 for critical paths)
