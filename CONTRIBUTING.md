# Contributing

## Prerequisites

To build and run this project locally, you must have **Rust** and **Cargo** installed.
Ensure `cargo` is available in your global system `PATH`.

## Running the Project

1. **Compile the project:**
   ```bash
   cargo build
   ```

2. **Run the Language Server:**
   The compiled binary will be located at:
   `target/debug/drupal_ls`

## Testing Changes

Before pushing any changes, please run the test suite to ensure no regressions are introduced:

```bash
cargo test
```

