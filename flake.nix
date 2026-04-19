{
    # Tremendous thanks to @oati for her help
    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay.url = "github:oxalica/rust-overlay";
    };
    outputs = { self, nixpkgs, rust-overlay, flake-utils }:
        flake-utils.lib.eachDefaultSystem (system:
        let
            pkgs = import nixpkgs {
                inherit system;
                overlays = [ rust-overlay.overlays.default ];
            };

            python-package-list = pkgs: with pkgs; [
                pip
            ];
            python = pkgs.python312.withPackages python-package-list;
        in
        {
            devShell = pkgs.mkShell rec {
                buildInputs = with pkgs; [
                    # Development / deployment
                    (rust-bin.stable.latest.default.override {
                        extensions = [ "rust-src" ];
                    })
                    (firebase-tools.override {
                        buildNpmPackage = buildNpmPackage.override { nodejs = nodejs_22; };
                    })

                    # Build dependencies
                    pkg-config
                    openssl
                    openssl.dev
                    libz
                    libxcb
                    libgcc
                    libglvnd
                    glib

                    # Runtime tools
                    bun
                    nodejs

                    # Useful for microservices development
                    ffmpeg  # media-conversion, reframing, pose-estimation
                    python  # validity-check, pose-estimation, cycle-detection, prediction
                ];
                shellHook =
                    ''
                    python -m venv .venv
                    source .venv/bin/activate
                    pip install -r ./igait-stages/pose-estimation/igait-mediapipe/requirements.txt
                    export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath (buildInputs ++ [ pkgs.stdenv.cc.cc ])}
                    '';

                # Set up environment for OpenSSL
                PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
                OPENSSL_DIR = "${pkgs.openssl.dev}";
                OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
                OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
            };
        });
}
