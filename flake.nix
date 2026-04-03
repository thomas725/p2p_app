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
              pkgs.rustup
              pkg-config
              openssl
              udev
              systemd
              sqlite
              cargo-cross
              upx
            ];

            RUST_BACKTRACE = "1";

            shellHook = ''
              # Initialize rustup if not already done
              if [ ! -f "$HOME/.rustup/bin/rustup" ]; then
                echo "Initializing rustup..."
                rustup init --default-toolchain none 2>/dev/null || true
              fi
              export PATH="$HOME/.rustup/bin:$PATH"
            '';
          };
        }
      );
    };
}
