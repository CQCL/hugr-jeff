{
  description = "Jeff Dev Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # Toolchain
            capnproto
            capnproto-rust
            cargo
            clippy
            rustfmt

            # Dev tools
            pre-commit
            just
            uv
          ];
        };
      });
}
