# jenga

Create stacked pull requests from your jj bookmarks. Work in progress!

## Usage

```
Submit your jj stack to GitHub

Usage: jenga <COMMAND>

Commands:
  status  View GitHub status
  submit  Submit a local stack to remote
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Installation

- Install Rust 2024 edition
- Clone the repo and `cd` into it
- `cargo install --path ./cli` - This compiles the binary and makes it available in your PATH

## Development

Currently jenga only supports operating in the jj repo of the current working directory. To run the dev build in a different jj repository than jenga, run:

```
cargo run --manifest-path /path/to/jenga/Cargo.toml -- {args}
```

