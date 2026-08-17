{
  description = "MCP server that provides RSS tooling";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/release-26.05";

    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    crane.url = "github:ipetkov/crane";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-parts,
      crane,
      ...
    }@inputs:
    let
      makePackages =
        pkgs:
        let
          lib = pkgs.lib;

          rust = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable.latest.default.override {
            extensions = [
              "rustfmt"
              "clippy"
              "rust-analyzer"
              "rust-src"
            ];
          };

          craneLib = crane.mkLib pkgs;

          cargoToml = builtins.fromTOML (builtins.readFile "${self}/src/mcp-rss/Cargo.toml");

          commonArgs = {
            src = craneLib.cleanCargoSource self;
            strictDeps = true;
            cargoExtraArgs = "-p mcp-rss";
          };

          unwrapped = craneLib.buildPackage (
            commonArgs
            // {
              pname = cargoToml.package.name;
              version = cargoToml.package.version;
              cargoArtifacts = craneLib.buildDepsOnly commonArgs;
              meta.mainProgram = "mcp-rss";
            }
          );
        in
        {
          inherit rust unwrapped;
          package =
            pkgs.callPackage
              (
                {
                  symlinkJoin,
                  mcp-rss-unwrapped,
                }:
                symlinkJoin {
                  name = "mcp-rss";
                  paths = [ mcp-rss-unwrapped ];
                  meta.mainProgram = "mcp-rss";
                }
              )
              {
                mcp-rss-unwrapped = unwrapped;
              };
        };
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake.overlays =
        let
          overlay =
            final: prev:
            let
              packages = makePackages final;
            in
            {
              mcp-rss = packages.package;
              mcp-rss-unwrapped = packages.unwrapped;
            };
        in
        {
          default = overlay;
          mcp-rss = overlay;
        };

      perSystem =
        { pkgs, lib, ... }:
        let
          flake-root = pkgs.writeShellApplication {
            name = "flake-root";
            text = ''
              current="$PWD"
              while [[ "$current" != "/" ]]; do
                if [[ -f "$current/flake.nix" ]]; then
                  echo "$current"
                  exit 0
                fi
                current="$(dirname "$current")"
              done
              echo "no flake.nix found" >&2
              exit 1
            '';
          };

          external = with pkgs; [
            flake-root
            git
            nushell
            nil
            nixfmt
            markdownlint-cli
            marksman
            mdbook
            taplo
            fd
            delta
            cachix
            release-plz
            markdown-link-check
            cspell
            prettier
            vscode-langservers-extracted
            yaml-language-server
            cargo-edit
          ];

          devScriptText = pkgs.writeText "mcp-rss-dev.nu" ''
            def "main" [] {
              dev -h
            }

            def "main run" [] {
              cd (flake-root)
              cargo run --bin mcp-rss
            }

            def "main format" [] {
              cd (flake-root)
              prettier --write .
              nixfmt ...(fd '.*\.nix$' . | lines)
              cargo fmt --all
              cargo clippy --fix --allow-dirty
            }

            def "main test" [] {
              if ($env.NIX_BUILD_TOP? | is-empty) {
                cargo clippy -- -D warnings
                cargo test
              }
            }

            def "main lint" [] {
              cd (flake-root)
              prettier --check .
              cspell lint . --no-progress
              nixfmt --check ...(fd '.*\.nix$' . | lines)
              markdownlint --ignore-path .markdownignore .
              if ($env.NIX_BUILD_TOP? | is-empty) {
                (markdown-link-check
                  --config .markdown-link-check.json
                  --quiet
                  ...(fd '.*.md' . | lines))
                (taplo lint
                  --schema "https://raw.githubusercontent.com/release-plz/release-plz/refs/tags/release-plz-v0.3.148/.schema/latest.json"
                  .release-plz.toml)
                cargo clippy -- -D warnings
                cargo test
              }
            }
          '';

          packages = makePackages pkgs;

          devScript =
            pkgs.writeShellApplication {
              name = "dev";
              runtimeInputs = external ++ [ packages.rust ];
              text = ''nu ${devScriptText} "$@"'';
            };
        in
        {
          devShells =
            {
              default = pkgs.mkShell {
                packages = external ++ [
                  packages.rust
                  devScript
                ];
                shellHook = ''
                  # Create a symlink to the vendored dependencies in the dev shell
                  if [ -d "${packages.unwrapped}/cargo-vendor" ]; then
                    ln -sf "${packages.unwrapped}/cargo-vendor" .cargo/vendor
                  fi
                '';
              };
            };

          apps =
            let
              packages = makePackages pkgs;

              app = {
                type = "app";
                program = lib.getExe packages.package;
                meta.description = "MCP server that provides RSS tooling";
              };
              unwrappedApp = {
                type = "app";
                program = lib.getExe packages.unwrapped;
                meta.description = "MCP server that provides RSS tooling (unwrapped)";
              };
            in
            {
              default = app;
              mcp-rss = app;
              unwrapped = unwrappedApp;
              mcp-rss-unwrapped = unwrappedApp;
            };

          packages =
            let
              packages = makePackages pkgs;

              docs =
                pkgs.runCommand "mcp-rss-docs"
                  {
                    src = self;
                    nativeBuildInputs = [ pkgs.mdbook ];
                  }
                  ''
                    mdbook build -d "$out" "$src/docs"
                  '';
            in
            {
              inherit docs;
              default = packages.package;
              mcp-rss = packages.package;
              unwrapped = packages.unwrapped;
              mcp-rss-unwrapped = packages.unwrapped;
            };
        };
    };

  nixConfig = {
    extra-substituters = [
      "https://haras.cachix.org"
    ];
    extra-trusted-public-keys = [
      "haras.cachix.org-1:/HIo1JYqOIH1Nwk1EGXhuPPvDW0WekxIbY5CiXUZbYw="
    ];
  };
}
