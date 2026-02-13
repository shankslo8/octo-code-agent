{
  description = "octo-code-agent — AI coding assistant for the terminal";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, crane, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Filter source to only Rust-relevant files (faster rebuilds)
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
            sqlite
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          env = {
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          };
        };

        # Build only dependencies (cached layer — rebuilds only when Cargo.lock changes)
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Full build
        octo-code = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        # `nix build` → result/bin/octo-code
        packages = {
          default = octo-code;
          octo-code = octo-code;
        };

        # `nix run` → builds & runs octo-code (all deps auto-installed)
        apps.default = flake-utils.lib.mkApp {
          drv = octo-code;
          name = "octo-code";
        };

        # `nix flake check` → build + clippy + tests
        checks = {
          inherit octo-code;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });

          tests = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
          });

          fmt = craneLib.cargoFmt { inherit src; };
        };

        # `nix develop` → full dev shell with all tools
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = with pkgs; [
            cargo-watch
            cargo-nextest
          ];

          env = {
            RUST_BACKTRACE = "1";
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          };

          shellHook = ''
            echo "🐙 octo-code-agent dev shell"
            echo "  rust  : $(rustc --version)"
            echo "  cargo : $(cargo --version)"
          '';
        };
      }
    );
}
