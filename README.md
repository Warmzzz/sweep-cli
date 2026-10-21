# sweep

[![CI](https://github.com/Warmzzz/sweep-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Warmzzz/sweep-cli/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Warmzzz/sweep-cli)](https://github.com/Warmzzz/sweep-cli/releases/latest)

A lightweight, cross-platform CLI that finds the folders your build tools generate
(`node_modules`, `target`, `__pycache__`, `bin/obj`…) and safely frees the disk space they use.

```
$ sweep ~/projects
Sweep - scanning /home/me/projects

Found 3 directory(ies):
      1.2 GB  [Node.js] /home/me/projects/web/node_modules
    364.3 MB  [Rust] /home/me/projects/api/target
     12.0 KB  [Python] /home/me/projects/tools/__pycache__

By project type:
      1.2 GB  Node.js (1 directory(ies))
    364.3 MB  Rust (1 directory(ies))
     12.0 KB  Python (1 directory(ies))

Space that can be freed: 1.6 GB

Delete these directories and free 1.6 GB? Type 'yes' to confirm:
```

## Features

- **Project aware**: a folder is only reported when it sits next to the file that identifies its project.
  A `target` folder is cleaned only if there is a `Cargo.toml` (or `pom.xml`…) beside it, so random folders named `bin` or `build` are left alone.
- **16 project types out of the box**, and you can [add your own](#custom-rules).
- **Shows the space you get back**, for each folder, per project type and in total.
- **Safe by default**: explicit confirmation, exclusions, and a last safety check before each deletion.
- **Fast**: every directory is read only once, artifacts are never walked into, and sizes are measured and folders deleted in parallel.
- **Single binary** for Windows, Linux and macOS, with no runtime to install.

## Installation

### Download a binary

Get the archive for your platform from the [latest release](https://github.com/Warmzzz/sweep-cli/releases/latest):

| Platform              | Archive                                       |
| --------------------- | --------------------------------------------- |
| Windows x86_64        | `sweep-<version>-x86_64-pc-windows-msvc.zip`  |
| Linux x86_64          | `sweep-<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64           | `sweep-<version>-aarch64-unknown-linux-musl.tar.gz` |
| macOS (Apple Silicon) | `sweep-<version>-aarch64-apple-darwin.tar.gz` |
| macOS (Intel)         | `sweep-<version>-x86_64-apple-darwin.tar.gz`  |

Extract it and put `sweep` (or `sweep.exe`) in a folder that is on your `PATH`.
Each release also has a `SHA256SUMS.txt` file to check the downloads.

> **macOS:** the binaries are not signed by Apple. If macOS refuses to open sweep, run
> `xattr -d com.apple.quarantine sweep` once.

### Build from source

With [Rust](https://rustup.rs) installed:

```bash
cargo install --git https://github.com/Warmzzz/sweep-cli
```

## Usage

```
sweep [OPTIONS] [PATH]
```

| Option              | Description                                                                 |
| ------------------- | --------------------------------------------------------------------------- |
| `[PATH]`            | Directory to scan (default: the current directory)                          |
| `--dry-run`         | Show what would be removed, and how much space it frees, without deleting   |
| `--exclude <PATH>`  | Leave this directory and everything inside it untouched (can be repeated)   |
| `-y`, `--yes`       | Delete without asking for confirmation                                      |
| `--config <PATH>`   | Config file with your own rules (default: `~/.config/sweep/config.toml`)    |
| `-h`, `--help`      | Print help                                                                  |
| `-V`, `--version`   | Print version                                                               |

### Examples

See what can be cleaned, without deleting anything:

```bash
sweep ~/projects --dry-run
```

Clean everything except one project:

```bash
sweep ~/projects --exclude ~/projects/client-app
```

Clean without the confirmation question, for example in a script:

```bash
sweep ~/projects --yes
```

## Supported projects

| Project type      | Recognized by                                        | Cleaned folders                                                                 |
| ----------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------- |
| Rust              | `Cargo.toml`                                         | `target`                                                                        |
| Node.js           | `package.json`                                       | `node_modules`, `.next`, `.nuxt`, `.svelte-kit`, `.angular`, `.turbo`, `.parcel-cache` |
| Python            | `pyproject.toml`, `requirements.txt`, `setup.py`, `Pipfile`, `*.py` | `__pycache__`, `.venv`, `venv`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox` |
| .NET              | `*.csproj`, `*.fsproj`, `*.vbproj`                   | `bin`, `obj`                                                                    |
| Maven             | `pom.xml`                                            | `target`                                                                        |
| Gradle            | `build.gradle(.kts)`, `settings.gradle(.kts)`        | `build`, `.gradle`                                                              |
| Scala (sbt)       | `build.sbt`                                          | `target`                                                                        |
| Clojure           | `project.clj`                                        | `target`                                                                        |
| PHP (Composer)    | `composer.json`                                      | `vendor`                                                                        |
| Elixir            | `mix.exs`                                            | `_build`, `deps`                                                                |
| Dart / Flutter    | `pubspec.yaml`                                       | `.dart_tool`, `build`                                                           |
| Swift             | `Package.swift`                                      | `.build`                                                                        |
| Haskell           | `stack.yaml`, `*.cabal`                              | `.stack-work`, `dist-newstyle`                                                  |
| Zig               | `build.zig`                                          | `zig-cache`, `.zig-cache`, `zig-out`                                            |
| C / C++ (CMake)   | `CMakeLists.txt`                                     | `build`, `cmake-build-debug`, `cmake-build-release`                             |
| Godot             | `project.godot`                                      | `.godot`                                                                        |

Folders that are often committed or written by hand (`dist` for Node.js, `vendor` for Go…) are not cleaned by default.
Add a [custom rule](#custom-rules) if you want them gone.

## Custom rules

Create `~/.config/sweep/config.toml` (on Windows: `C:\Users\<you>\.config\sweep\config.toml`),
or pass any file with `--config <PATH>`. Your rules are used in addition to the built-in ones.

```toml
[[rule]]
name = "Terraform"
markers = ["*.tf"]
artifacts = [".terraform"]

[[rule]]
name = "Node.js build output"
markers = ["package.json"]
artifacts = ["dist", "coverage"]
```

Each `[[rule]]` has:

- `name`: shown next to each folder found.
- `markers`: files that identify the project. `"main.tf"` is an exact file name, `"*.tf"` means any file with this extension.
- `artifacts`: folder names that are deleted when they sit next to one of the markers.

The config file is checked when sweep starts: a misspelled field, a rule without markers or artifacts,
or an artifact that is a path (like `../src`) stops sweep with a clear error instead of being ignored.
A ready-to-copy example is in [`config.example.toml`](config.example.toml), also included in every release archive.

## Safety

Deleting files is not something to get wrong, so sweep is careful at every step:

1. **Nothing is deleted without your consent.** You must type `yes` in full; any other answer cancels.
   Use `--dry-run` to only look, or `--yes` to skip the question on purpose.
2. **Exclusions are checked.** An `--exclude` path that does not exist stops sweep, so a typo cannot delete what you wanted to keep.
3. **Each folder is checked right before it is deleted.** sweep refuses to delete a folder that:
   - is not strictly inside the scanned directory;
   - is inside a system directory (`C:\Windows`, `Program Files`, `/usr`, `/etc`, `/System`…);
   - contains your home directory, the current directory or the sweep program itself;
   - is a symbolic link, or is no longer a directory.
4. **Version control folders** (`.git`, `.hg`, `.svn`) are never scanned.

Sizes are an estimate: they add up file sizes, which can differ slightly from the space used on disk.

## Development

```bash
cargo test
```

```bash
cargo clippy --all-targets -- -D warnings
```

```bash
cargo run -- ~/projects --dry-run
```

The code is split into small modules, each with one job:

| Module          | Role                                                         |
| --------------- | ------------------------------------------------------------ |
| `rules.rs`      | Built-in rules and how a rule recognizes a project           |
| `config.rs`     | Reads and checks the user's own rules                        |
| `directory.rs`  | Reads one directory from the disk, only once                 |
| `scanner.rs`    | Walks the tree and finds the artifacts                       |
| `size.rs`       | Measures and displays disk space                             |
| `report.rs`     | Totals and statistics per project type                       |
| `path_list.rs`  | Compares paths reliably (exclusions, system directories)     |
| `safety.rs`     | The last check before each deletion                          |
| `cleaner.rs`    | Deletes the artifacts and sums up the result                 |

To support a new language for everyone, add a `Rule::new(...)` line to `built_in()` in `src/rules.rs`.

### Releasing

1. Update `version` in `Cargo.toml`, run `cargo build` to update `Cargo.lock`, and commit.
2. Tag and push:

   ```bash
   git tag v1.0.0
   ```

   ```bash
   git push origin main --tags
   ```

GitHub Actions checks that the tag matches the version, runs the tests, builds the five binaries
and publishes them on a GitHub release.
