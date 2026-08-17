{
  description = "p2p_app development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs =
    { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config = {
              allowUnfree = true;
              android_sdk.accept_license = true;
            };
          };
          androidSdk = pkgs.androidenv.composeAndroidPackages {
            platformToolsVersion = "36.0.1";
            buildToolsVersions = [ "36.0.0" "28.0.3" ];
            platformVersions = [ "36" ];
            abiVersions = [ "arm64-v8a" "armeabi-v7a" ];
            includeNDK = true;
          };
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
              # Dioxus Desktop + Flutter Desktop dependencies
              gtk3
              xdotool
              # libsoup_2_4 # insecure! let's comment this out for now..
              webkitgtk_4_1
              glib
              pango
              cairo
              gdk-pixbuf
              harfbuzz
              libepoxy
              libx11
              libxcursor
              libxrandr
              libxi
              libxrender
              libxext
              libxdamage
              libxfixes
              xorgproto
              zlib
              xz
              # nodejs_24 # for installing and trying https://freebuff.ai/ in our sandbox: npm install -g freebuff # doesn't work in Austria.
              # Flutter + Android development
              flutter
              cmake
              ninja
              jdk17
              gradle
            ];

            RUST_BACKTRACE = "1";

            # ANDROID_HOME set in shellHook (builtins.getEnv is empty in pure eval)

            shellHook = ''
              export ANDROID_HOME="$HOME/.android-sdk"
              # Source rustup environment if available
              if [ -f "$HOME/.rustup/bin/rustup" ]; then
                source "$HOME/.rustup/bin/rustup-init.sh" 2>/dev/null || true
              fi
              rustup show 2>/dev/null || rustup install stable
              # Add Android cross-compilation targets
              rustup target add aarch64-linux-android armv7-linux-androideabi 2>/dev/null || true
              # Create writable SDK overlay from Nix-built SDK
              SDK_SRC="${androidSdk.androidsdk}/libexec/android-sdk"
              if [ ! -d "$HOME/.android-sdk" ] || [ ! -f "$HOME/.android-sdk/.initialized" ]; then
                chmod -R u+w "$HOME/.android-sdk" 2>/dev/null || true
                rm -rf "$HOME/.android-sdk"
                cp -r "$SDK_SRC" "$HOME/.android-sdk"
                chmod -R u+w "$HOME/.android-sdk"
                touch "$HOME/.android-sdk/.initialized"
              fi
            '';
          };
        }
      );
    };
}
