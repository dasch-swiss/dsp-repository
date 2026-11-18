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

        # Pinned with CI and the justfile; the locked rust-overlay must know the date in
        # .rustfmt-nightly, or this fails to evaluate.
        rustfmtNightlyDate = pkgs.lib.removePrefix "nightly-"
          (pkgs.lib.trim (builtins.readFile ./.rustfmt-nightly));
        rustNightly = pkgs.rust-bin.nightly.${rustfmtNightlyDate}.minimal.override {
          extensions = [ "rustfmt" ];
        };

        # macOS: unified Apple SDK provides all frameworks (Security, CoreFoundation, etc.)
        darwinDeps = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
          pkgs.apple-sdk
          pkgs.libiconv
        ];

      in {
        devShells.default = pkgs.mkShell {
          # Use nightly rustfmt — .rustfmt.toml requires nightly features
          # (group_imports, imports_granularity). `just fmt` and `just check` run this binary.
          RUSTFMT = "${rustNightly}/bin/rustfmt";

          buildInputs = [
            # Rust: Bazel builds with its own toolchain (MODULE.bazel). Stable serves `cargo binstall`
            # and dsp-cli's publish; nightly serves rustfmt.
            rustStable
            rustNightly

            # Build dependencies for Rust crates
            pkgs.pkg-config

            # Development tools (version-matched in nixpkgs)
            pkgs.just
            pkgs.nodejs_24    # for the Playwright e2e suites (npx)
            pkgs.cargo-binstall
            pkgs.jq           # `just fair-check` reads F-UJI's JSON result with it

            # Bazel: Bazelisk reads .bazelversion. nixpkgs installs no `bazel` command, which the
            # recipes call, so a shim provides it.
            pkgs.bazelisk
            (pkgs.writeShellScriptBin "bazel" ''exec ${pkgs.bazelisk}/bin/bazelisk "$@"'')
            # The two halves of `just audit`, both reading Cargo.Bazel.lock.
            pkgs.cargo-audit
            pkgs.osv-scanner

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
            _ensure_tool commitlint      commitlint-rs     0.2.4    # commit-message gate (just commit-lint)

            unset -f _ensure_tool
          '';
        };
      }
    );
}
