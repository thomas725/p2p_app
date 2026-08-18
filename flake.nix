{
  description = "p2p_app development environment with newer Flutter/Android";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-unstable, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        baseConfig = {
          allowUnfree = true;
          # or allowUnfreePredicate = … if you want to restrict

          android_sdk.accept_license = true;
        };

        pkgs = import nixpkgs {
          inherit system;
          config = baseConfig;
        };

        pkgs-unstable = import nixpkgs-unstable {
          inherit system;
          config = baseConfig;  # same config, including license acceptance
        };

        jdk = pkgs.jdk;

        # Unstable tools
        flutter     = pkgs-unstable.flutter;
        # jdk         = pkgs-unstable.temurin-bin-21;
        # rustup      = pkgs-unstable.rustup;
        # cmake       = pkgs-unstable.cmake;
        # ninja       = pkgs-unstable.ninja;
        # clang       = pkgs-unstable.clang;
        # lld         = pkgs-unstable.lld;
        # cargo-cross = pkgs-unstable.cargo-cross;
        # upx         = pkgs-unstable.upx;
        # gradle      = pkgs-unstable.gradle;

        androidPackages = pkgs.androidenv.composeAndroidPackages {
          platformToolsVersion = "36.0.1";
          buildToolsVersions   = [ "36.0.0" "28.0.3" ];
          platformVersions     = [ "36" ];
          abiVersions          = [ "arm64-v8a" "armeabi-v7a" ];
          includeNDK           = true;
        };
        androidSdk = androidPackages.androidsdk;
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            flutter
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

            gtk3
            xdotool
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

            cmake
            ninja
            jdk
            gradle
          ];

          RUST_BACKTRACE = "1";

          shellHook = ''
            export ANDROID_HOME="$HOME/.android-sdk"

            export JAVA_HOME="${jdk}"
            export PATH="$JAVA_HOME/bin:$PATH"

            if [ -f "$HOME/.rustup/bin/rustup" ]; then
              source "$HOME/.rustup/bin/rustup-init.sh" 2>/dev/null || true
            fi
            rustup show 2>/dev/null || rustup install stable
            rustup target add aarch64-linux-android armv7-linux-androideabi 2>/dev/null || true

            SDK_SRC="${androidSdk}/libexec/android-sdk"
            if [ ! -d "$HOME/.android-sdk" ] || [ ! -f "$HOME/.android-sdk/.initialized" ]; then
              rm -rf "$HOME/.android-sdk"
              cp -r "$SDK_SRC" "$HOME/.android-sdk"
              chmod -R u+w "$HOME/.android-sdk"
              touch "$HOME/.android-sdk/.initialized"
            fi
          '';
        };

        # Optional: also expose this config via flake outputs
        nixpkgs.config = baseConfig;
      }
    );
}