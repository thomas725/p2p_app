{
  description = "p2p_app development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
  };

  outputs = { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustup
              pkg-config
              openssl
              udev
              systemd
              sqlite
              cargo-cross
              upx
              lld
              clang
              binutils
              bashInteractive
              gcc
            ];

            RUST_BACKTRACE = "1";

            shellHook = ''
              # Source rustup environment if available
              if [ -f "$HOME/.rustup/bin/rustup" ]; then
                source "$HOME/.rustup/bin/rustup-init.sh" 2>/dev/null || true
              fi
              rustup show 2>/dev/null || rustup install stable
            '';
          };
        }
      );
    };
}
