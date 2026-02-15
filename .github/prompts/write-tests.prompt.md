---
agent: 'agent'
tools: ['search/codebase', 'edit/editFiles', 'search', 'search/usages', 'terminal', 'read/problems', 'findTestFiles']
description: 'Generate unit tests for Rust code using mockall'
---

# Write Tests

Your goal is to generate comprehensive unit tests for the specified code.

## Requirements

- Use Rust's built-in testing framework
- Use `mockall` for mocking traits
- Use `#[tokio::test]` for async tests
- Follow naming: `test_<function>_<scenario>_<outcome>`

## Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_function_success() {
        // Arrange
        let mut mock = MockDependency::new();
        mock.expect_method()
            .times(1)
            .returning(|| Ok(expected_value));

        // Act
        let result = function_under_test(&mock).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }
}
```

## Coverage Requirements

Generate tests for:
1. Happy path (success case)
2. Error cases (each error variant)
3. Edge cases (empty input, boundary values)
4. Async behavior (if applicable)

## Mock Patterns

- Use `expect_<method>()` for setting expectations
- Use `with(eq(value))` for argument matching
- Use `times(n)` for call count
- Use `returning(|args| value)` for return values

Analyze the target code and generate appropriate tests.
