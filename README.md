# MCP RSS

<!-- ANCHOR: body -->

MCP server that provides RSS tooling.

## Installation

`mcp-rss` is packaged as a Nix flake. Run it directly without installing:

```sh
nix run github:haras-unicorn/mcp-rss
```

or build the `mcp-rss` binary with:

```sh
nix build github:haras-unicorn/mcp-rss
```

### Releases

Prebuilt binaries for `x86_64-linux` and `aarch64-linux` are attached to each
[GitHub release] as tarballs containing the `mcp-rss` binary.

```sh
curl -L -o mcp-rss.tar.gz \
  https://github.com/haras-unicorn/mcp-rss/releases/latest/download/mcp-rss-x86_64-linux.tar.gz
tar -xzf mcp-rss.tar.gz
./mcp-rss-x86_64-linux
```

[GitHub release]: https://github.com/haras-unicorn/mcp-rss/releases

### NixOS and home-manager

Add the flake as an input and apply its overlay so that `mcp-rss` is available
in your system configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    mcp-rss.url = "github:haras-unicorn/mcp-rss";
  };

  outputs =
    { nixpkgs, mcp-rss, ... }:
    {
      nixosConfigurations.my-machine = nixpkgs.lib.nixosSystem {
        modules = [
          { nixpkgs.overlays = [ mcp-rss.overlays.default ]; }
        ];
      };
    };
}
```

Then add `pkgs.mcp-rss` to your packages, either in NixOS:

```nix
{ pkgs, ... }:
{
  environment.systemPackages = [ pkgs.mcp-rss ];
}
```

or with home-manager:

```nix
{ pkgs, ... }:
{
  home.packages = [ pkgs.mcp-rss ];
}
```

### Binary cache

Builds are cached on the [haras cachix cache]. When the flake is used directly
(for example with `nix run github:haras-unicorn/mcp-rss`), the cache is
configured automatically through the flake's `nixConfig`. To use it when the
package comes from an overlay, add the following to your nix configuration:

```nix
{
  nix.settings = {
    substituters = [ "https://haras.cachix.org" ];
    trusted-public-keys = [
      "haras.cachix.org-1:/HIo1JYqOIH1Nwk1EGXhuPPvDW0WekxIbY5CiXUZbYw="
    ];
  };
}
```

[haras cachix cache]: https://app.cachix.org/cache/haras

## Usage

`mcp-rss` is an MCP server that speaks the Model Context Protocol over stdio.
Add it as a stdio MCP server to any MCP client, for example:

```json
{
  "mcpServers": {
    "mcp-rss": {
      "command": "nix",
      "args": ["run", "github:haras-unicorn/mcp-rss"]
    }
  }
}
```

If `mcp-rss` is already on your `PATH`, point the client at the binary directly
instead:

```json
{
  "mcpServers": {
    "mcp-rss": {
      "command": "mcp-rss",
      "args": []
    }
  }
}
```

<!-- ANCHOR_END: body -->

## Documentation

The documentation is available at <https://haras-unicorn.github.io/mcp-rss/>.
