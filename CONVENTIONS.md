# Conventions

These conventions are intended to guide development on this project, ensuring consistency and maintainability.

* **Formatting:** Respect and adhere to the rules defined in `.editorconfig` and `.rustfmt.toml`. Use `cargo fmt` to automatically format Rust code.
* **Concurrency:** Avoid using `async` and `await`. All operations, including I/O and external requests, must be blocking.
* **Comments:** Do not add comments unless they are crucial for explaining non-obvious logic or complex algorithms. Strive for self-documenting code through clear structure and naming.
* **Naming:** Use descriptive, self-documenting names for variables, functions, structs, and other identifiers. Follow the pattern of starting with the item or base name, followed by a descriptor or qualifier (e.g., `path_config`, `client_llm`).
* **Fail Fast:** Check for and handle all error conditions and invalid inputs at the beginning of a function. Return immediately upon encountering a problem. Do not use an `else` block for code that handles the successful case; place the successful case logic directly after the early return checks.
