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

        # macOS: unified Apple SDK provides all frameworks (Security, CoreFoundation, etc.)
        darwinDeps = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
          pkgs.apple-sdk
          pkgs.libiconv
        ];

      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            # Rust toolchains
            rustStable
            rustNightly

            # Build dependencies for Rust crates
            pkgs.cmake      # aws-lc-sys
            pkgs.pkg-config

            # Development tools (version-matched in nixpkgs)
            pkgs.just
            pkgs.nodejs_24
            pkgs.pnpm         # exact version managed by corepack via packageManager field
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
            _ensure_tool wasm-bindgen    wasm-bindgen-cli  0.2.105   # must match Cargo.toml wasm-bindgen = "=0.2.105"
            _ensure_tool cargo-leptos    cargo-leptos      0.3.4
            _ensure_tool mdbook          mdbook            0.4.52
            _ensure_tool leptosfmt       leptosfmt         0.1.33
            _ensure_tool mdbook-alerts   mdbook-alerts     0.8.0

            unset -f _ensure_tool
          '';
        };
      }
    );
}
