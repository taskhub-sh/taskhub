- Always run `cargo audit` to check for security vulnerability in packages. You **must** fix all errors. Allowed warnings are ok. You **must never add** ignore rules for vulnerabilities to fix cargo audit.
- Always run `cargo test` to check tests pass. You **must** fix all errors and warnings.
- Always run `cargo build` to check the program compiles correctly. You **must** fix all errors and warnings.
- Always run `cargo clippy` to lint the program. You **must** fix all errors and warnings.
- Always run `cargo deny check` to lint dependencies. You **must** fix all errors.

- When adding a dependency to `Cargo.toml` prefer to use the latest version of the dependency unless there is an explicit reason not to.
