# oneil_snapshot_tests

Snapshot tests for Oneil evaluation output and errors.

These are **integration-style** tests: they run the full pipeline (parse -> resolve -> eval) on fixture `.on` files and compare the formatted output and errors against stored snapshots using [insta](https://docs.rs/insta).

## What is snapshot tested

- **Evaluation output**: model path, test results (PASS/FAIL), and parameter values.
- **Errors**: parse, resolution, and evaluation errors in a canonical format (`error: <message>` and `at <path>:<line>:<column>`).

## Running the tests

```bash
cargo test -p oneil_snapshot_tests
```

## Updating snapshots

When you intentionally change output or error format, update the snapshots:

```bash
INSTA_UPDATE=1 cargo test -p oneil_snapshot_tests
```

Or use the insta CLI to review and accept changes:

```bash
cargo install cargo-insta
cargo test -p oneil_snapshot_tests
cargo insta review
```

## Layout

- `fixtures/*.on` – Oneil source files used as test inputs.
- `src/test.rs` – Snapshot test module (integration tests that call `run_model_and_format` and assert with `insta::assert_snapshot!`).
- `src/snapshots/*.snap` – Stored snapshots (created on first run or when `INSTA_UPDATE=1`).

## Adding a new snapshot test

1. Add a new fixture in `fixtures/<name>.on`.
2. In `src/test.rs`, add a test that calls `run_model_and_format(fixture_path("<name>.on"))` and `insta::assert_snapshot!(output)`.
3. Run with `INSTA_UPDATE=1` to generate the new snapshot.
