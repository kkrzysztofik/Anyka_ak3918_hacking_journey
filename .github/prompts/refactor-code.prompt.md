---
agent: 'agent'
tools: ['search/codebase', 'edit/editFiles', 'search/usages', 'search', 'terminal', 'read/problems', 'findTestFiles']
description: 'Refactor code while preserving behavior'
---

# Refactor Code

Your goal is to refactor the specified code while preserving functionality.

## Refactoring Principles

1. **Preserve Behavior**: No functional changes unless explicitly requested
2. **Improve Readability**: Make code easier to understand
3. **Reduce Complexity**: Break down large functions
4. **Apply DRY**: Extract common patterns
5. **Maintain Tests**: Update tests if signatures change

## Common Refactorings

### Extract Function
Break large functions into focused, single-purpose functions.

### Extract Trait
Create traits for dependency injection and testability.

### Error Consolidation
Consolidate scattered error handling into proper error types.

### Async Cleanup
Ensure proper async patterns (tokio::sync, no blocking).

### Module Reorganization
Move code to appropriate modules based on responsibility.

## Process

1. Analyze the current code structure
2. Identify refactoring opportunities
3. Apply changes incrementally
4. Verify tests still pass
5. Update documentation if needed

## Constraints

- Keep changes focused on one refactoring at a time
- Maintain backward compatibility for public APIs
- Follow project naming conventions
- Ensure all tests pass after refactoring
