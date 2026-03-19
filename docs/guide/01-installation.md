# Installation

This section describes how to install the Oneil CLI (Rust implementation) on Linux, Windows, and macOS. Pre-built binaries are provided for these platforms via [GitHub Releases](https://github.com/careweather/oneil/releases).

## Prerequisites

- **Rust** (for building from source): [rustup](https://rustup.rs/) — install and ensure `cargo` is on your `PATH`.

Optional, for full functionality:

- **Python 3.10+** (for Python breakout functions and optional runtime features). The CLI can run without it; Python is only needed when your models use `import` and Python-defined functions.

## Option 1: Download a release from GitHub (recommended)

Pre-built binaries are published on the [Releases](https://github.com/careweather/oneil/releases) page for:

- **Linux** (x86_64, `unknown-linux-gnu`)
- **Windows** (x86_64, `pc-windows-msvc`)
- **macOS** (x86_64 and Apple Silicon, `apple-darwin`)

### Linux / macOS

1. Open the [latest release](https://github.com/careweather/oneil/releases/latest).
2. Download the archive for your OS and architecture (e.g. `oneil-v0.15.0-x86_64-unknown-linux-gnu.tar.gz`).
3. Unpack and put the `oneil` binary on your `PATH`:

   ```sh
   tar -xzf oneil-v*-x86_64-unknown-linux-gnu.tar.gz
   sudo mv oneil /usr/local/bin/
   # or, without sudo:
   mkdir -p ~/.local/bin && mv oneil ~/.local/bin/
   # ensure ~/.local/bin is in your PATH
   ```

   On macOS, use the appropriate archive (e.g. `oneil-v*-aarch64-apple-darwin.tar.gz` for Apple Silicon).

4. Confirm:

   ```sh
   oneil --version
   ```

### Windows

1. Open the [latest release](https://github.com/careweather/oneil/releases/latest).
2. Download the `.zip` for Windows (e.g. `oneil-v0.15.0-x86_64-pc-windows-msvc.zip`).
3. Unzip and either move `oneil.exe` into a directory on your `PATH`, or add the folder containing `oneil.exe` to your `PATH`.
4. Confirm in PowerShell or Command Prompt:

   ```cmd
   oneil --version
   ```

## Option 2: Install from source with Cargo

Use this if you want the latest development version or need to customize the build.

1. Clone the repository:

   ```sh
   git clone https://github.com/careweather/oneil.git
   cd oneil
   ```

2. Build and install the `oneil` binary (requires Rust):

   ```sh
   cargo install --path src-rs/oneil
   ```

   This places the `oneil` executable in `~/.cargo/bin` (or `%USERPROFILE%\.cargo\bin` on Windows). Ensure that directory is on your `PATH`.

3. Optional: build without Python support (avoids Python/pyo3 dependencies):

   ```sh
   cargo install --path src-rs/oneil --no-default-features --features rust-lib
   ```

4. Confirm:

   ```sh
   oneil --version
   ```

## Option 3: Install from source using the install script

From the repository root, you can run the project’s install script to install the Python package and set up Vim syntax highlighting:

```sh
git clone https://github.com/careweather/oneil.git
cd oneil
./install.sh
```

This installs the Python implementation and editor support. For the Rust CLI, use Option 1 or Option 2.

## Option 4: Run from the repository (development)

For day-to-day development without installing:

```sh
git clone https://github.com/careweather/oneil.git
cd oneil
cargo build -p oneil
./target/debug/oneil --version
# or run directly:
cargo run -p oneil -- path/to/model.on
```

## Editor and tooling (optional)

- **Vim**: See the [Vim support](https://github.com/careweather/oneil#vim-support) section in the main README for syntax highlighting.

- **VS Code / Cursor**: Install the [Oneil extension](https://marketplace.visualstudio.com/items?itemName=careweather.oneil) from the Marketplace for LSP and syntax highlighting.


## Install Oneil Python

To install the `oneil` package into your current Python environment from a checkout of the repository, run `pip install .` from the **project root** (the directory that contains `pyproject.toml`):

```sh
git clone https://github.com/careweather/oneil.git
cd oneil
pip install .
```

For an editable install while developing the package:

```sh
pip install -e .
```

After installation you can `import oneil` in Python.

> ![NOTE]
> This does NOT install the CLI, only the python library.


## Troubleshooting

- **`oneil: command not found`**  
  Ensure the directory containing the `oneil` binary is on your `PATH`.

- **Python-related build errors** (from source)  
  Either install Python 3.10+ and development headers, or install with `--no-default-features --features rust-lib` to disable Python support.

- **Permission denied** (Linux/macOS)  
  After moving the binary, run `chmod +x /path/to/oneil` (or the path you used).
