# Oneil Shared

This crate provides tools that are used throughout the project, including:
- [span information](./src/span)
- [standardized errors](./src/error)

A unified error handling system for the Oneil programming language.

This crate enables components to use their own error types while also defining a unified interface with which to work.

## Spans

Spans refer to a location in a source file. They store the the offset, line, and column for the beginnig and end of the important text.

## `AsOneilDiagnostic`

The main feature of the error handling provided by this library is the `AsOneilDiagnostic` trait found in [`traits.rs`](src/traits.rs). Error and other diagnostic types should implement this trait in order to be compatible with Oneil CLI diagnostic printing.

### Example

```rust
use oneil_shared::error::{DiagnosticKind, OneilDiagnostic, AsOneilDiagnostic, Context, ErrorLocation};
use std::path::PathBuf;

// Define an error type that implements AsOneilDiagnostic
struct MyError {
    message: String,
    offset: usize,
}

impl AsOneilDiagnostic for MyError {
    fn kind(&self) -> DiagnosticKind {
        DiagnosticKind::Error
    }

    fn message(&self) -> String {
        self.message.clone()
    }

    fn diagnostic_location(&self, source: &str) -> Option<ErrorLocation> {
        if self.offset < source.len() {
            Some(ErrorLocation::from_source_and_offset(source, self.offset))
        } else {
            None
        }
    }

    fn context(&self) -> Vec<Context> {
        vec![Context::Help("Try checking your syntax".to_string())]
    }
}

// Convert to OneilDiagnostic
let my_error = MyError {
    message: "Unexpected token".to_string(),
    offset: 10,
};

let source = "My X: x = $";
let path = PathBuf::from("example.on");
let diagnostic = OneilDiagnostic::from_error_with_source(&my_error, path, source);
```
