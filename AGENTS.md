# AGENTS.md

`mcp-rss` is a Rust MCP server that provides RSS tooling. It is a Cargo
workspace with a single crate, `mcp-rss`, inside `src`.

## Structure

- `src/mcp-rss/src/lib.rs` - MCP server and tool definitions.
- `src/mcp-rss/src/main.rs` - stdio entry point.
- `docs` - mdBook documentation.
- `flake.nix` - flake exposing the development shell, the `mcp-rss` package and
  runnable apps.

## Development

The default development shell (assume you are already running inside it)
provides the following scripts:

- `dev run` - run the MCP server over stdio.
- `dev format` - format the repository.
- `dev lint` - lint and test the repository.
- `dev test` - check clippy warnings and run rust tests.
