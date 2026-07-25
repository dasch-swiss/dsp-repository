{
  description = "DSP Repository development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Stable Rust toolchain — reads rust-toolchain.toml so version stays in sync
        rustStable = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        # Nightly rustfmt — needed by `cargo +nightly fmt` in justfile
        rustNightly = pkgs.rust-bin.nightly.latest.minimal.override {
          extensions = [ "rustfmt" ];
        };

        # Wrapper that makes `cargo +nightly fmt` work without rustup.
        # The justfile uses `cargo +nightly fmt` which is a rustup-only feature.
        # This shim strips `+toolchain` args and delegates to the real cargo,
        # while RUSTFMT env var ensures nightly rustfmt is used for formatting.
        cargoWrapper = pkgs.writeShellScriptBin "cargo" ''
          args=()
          for arg in "$@"; do
            case "$arg" in
              +*) ;; # strip +toolchain args (e.g., +nightly)
              *)  args+=("$arg") ;;
            esac
          done
          exec "${rustStable}/bin/cargo" "''${args[@]}"
        '';

        # macOS: unified Apple SDK provides all frameworks (Security, CoreFoundation, etc.)
        darwinDeps = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
          pkgs.apple-sdk
          pkgs.libiconv
        ];

      in {
        devShells.default = pkgs.mkShell {
          # Use nightly rustfmt — .rustfmt.toml requires nightly features
          # (group_imports, imports_granularity). cargo fmt respects this env var.
          RUSTFMT = "${rustNightly}/bin/rustfmt";

          buildInputs = [
            # Rust: wrapper first on PATH so it shadows the raw cargo binary
            cargoWrapper
            rustStable
            rustNightly

            # Build dependencies for Rust crates
            pkgs.cmake      # aws-lc-sys
            pkgs.pkg-config

            # Development tools (version-matched in nixpkgs)
            pkgs.just
            pkgs.nodejs_24    # for the Playwright e2e suites (npx)
            pkgs.cargo-watch  # 8.5.3 — matches justfile pin
            pkgs.cargo-binstall

          ] ++ darwinDeps;

          shellHook = ''
            # Install version-pinned tools not available (or version-mismatched) in nixpkgs.
            # Uses cargo-binstall for fast binary downloads. Idempotent — skips if correct
            # version is already installed.

            _ensure_tool() {
              local cmd="$1" pkg="$2" version="$3"
              if ! command -v "$cmd" &>/dev/null || \
                 [ "$("$cmd" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)" != "$version" ]; then
                echo "Installing $pkg@$version via cargo-binstall..."
                cargo binstall -y "$pkg@$version" --quiet
              fi
            }

            # Versions pinned to match justfile install-requirements
            _ensure_tool mdbook          mdbook            0.4.52
            _ensure_tool mdbook-alerts   mdbook-alerts     0.8.0
            _ensure_tool mdbook-mermaid  mdbook-mermaid    0.16.2   # 0.17+ requires mdbook 0.5
            # Maud dev tooling (DEV-6642)
            _ensure_tool bacon           bacon             3.23.0   # dev loop: kill_then_restart server
            _ensure_tool maudfmt         maudfmt           0.1.8    # Maud template formatting
            _ensure_tool cargo-machete   cargo-machete     0.9.2    # unused-dependency check (just check)

            unset -f _ensure_tool
          '';
        };
      }
    );
}
