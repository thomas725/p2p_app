# https://stackoverflow.com/questions/29008127/why-are-rust-executables-so-huge
# https://github.com/johnthagen/min-sized-rust

## build with debugging info telling which src file and line caused a panic:
#debug=1

project="p2p_chat_example"
z1="build-std=std,panic_abort"

if [ -z "$features" ]; then
  features="default"
fi

if [ -z ${debug+x} ] || [ $debug -ne 1 ]; then
  # -Zfmt-debug=none # don't use because it breaks debug printing structs which we use a lot! also, doesn't save that much..
  if [ -z ${stable+x} ] || [ $stable -ne 1 ]; then
    # error: the option `Z` is only accepted on the nightly compiler
    RUSTFLAGS="-Zlocation-detail=none"
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=gcc
  else
    RUSTFLAGS=""
  fi
else
  RUSTFLAGS='-C debuginfo=2'
fi
if [[ $features == *"tokio_console"* ]]; then
  echo "building with RUSTFLAGS --cfg tokio_unstable!"
  RUSTFLAGS+=" --cfg tokio_unstable"
fi
if [ "$target" == "x86_64-unknown-linux-gnu" ]; then
  echo "building for x86_64, can use cargo, don't need cross"
  bin=cargo
else
  echo "cross-compiling to: $target - using 'cross' instead of 'cargo'"
  bin=cross
fi
if [ -z ${debug+x} ] || [ $debug -ne 1 ]; then
  profile='release'
  if [ -z ${stable+x} ] || [ $stable -ne 1 ]; then
    RUSTFLAGS="$RUSTFLAGS" $bin +nightly build -Z $z1 --target $target --release --no-default-features -F $features
  else
    # nightly sometimes doesn't work. especially with cross compiling. if it doesn't, set environment stable=1 flag for:
    RUSTFLAGS="$RUSTFLAGS" $bin build --target $target --release --no-default-features -F $features
  fi
else
  echo "debug flag set: using release-with-debug profile + CARGO_PROFILE_RELEASE_DEBUG flag."
  CARGO_PROFILE_RELEASE_DEBUG=true
  profile='release-with-debug'
  # +nightly
  RUSTFLAGS="$RUSTFLAGS" $bin build --target $target --profile=release-with-debug --no-default-features -F $features
fi
ls -al target/$target/$profile/$project
ls -alh target/$target/$profile/$project
if [ -z ${skip_upx+x} ] || [ $skip_upx -ne 1 ]; then
  # upx version 5 outputs binaries that don't work on legacy system:
  # > UPX-5.0 wants mead_create(), or needs /dev/shm(,O_TMPFILE,)
  # so we use latest version 4 instead:
  # cd /opt ; wget https://github.com/upx/upx/releases/download/v4.2.4/upx-4.2.4-amd64_linux.tar.xz ; tar xf upx-4.2.4-amd64_linux.tar.xz
  # /opt/upx-4.2.4-amd64_linux/
  upx --best --lzma target/$target/$profile/$project
  ls -al target/$target/$profile/$project
  ls -alh target/$target/$profile/$project
fi

## 2025-08-30 ~01:00: default release build = 4573480 bytes = 4,4MiB
# target=x86_64-unknown-linux-gnu features=basic ./build_release.sh
# 3357696 -> 1227248

## 2025-08-30 ~19:44: with migrations embedded:
# 3395272 -> 1237180

# target/x86_64-unknown-linux-gnu/release/p2p_chat_example
# DATABASE_URL=sqlite2.db target/x86_64-unknown-linux-gnu/release/p2p_chat_example

# skip_upx=1 target=x86_64-unknown-linux-gnu features=basic ./build_release.sh

# with embedded sqlite for Roborock Q7
# skip_upx=1 target=armv7-unknown-linux-musleabihf features=basic ./build_release.sh
#3445368 -> 1458072 # can't use UPX:
# > UPX-5.0 wants memfd_create(), or needs /dev/shm(,O_TMPFILE,)
#uncompressed 3,4MiB binary fails because of some protocol not being supported, I believe it's about quic:
# INFO quinn_udp::imp::gso: GSO disabled: kernel too old (3.4.39); need 4.18+
# Error: Protocol not available (os error 92)
#removed quic to find significant smaller binary size:
#2583120 ->   112959 and a different error message:
# Error: Multiaddr is not supported: /ip4/0.0.0.0/udp/0/quic-v1

# trying debug version to see where we need to gracefully handle errors:
# debug=1 target=armv7-unknown-linux-musleabihf features=basic ./build_release.sh
#49568564 ->  10329136

# target=aarch64-unknown-linux-gnu features=basic ./build_release.sh

# 2025-09-02 00:12 with bundled libsqlite3-sys:
# target=x86_64-unknown-linux-gnu features=basic ./build_release.sh
# 4429560 = 4,3M   ->   1690308 = 1,7M

# 2025-09-02 00:14 without bundled libsqlite3-sys:
# 3403840 = 3,3M   ->   1238336 = 1,2M

# without bundled libsqlite3-sys:
# target=armv7-unknown-linux-musleabihf features=basic ./build_release.sh
# fails to build: GLIBC_2.32, GLIBC_2.33, GLIBC_2.34 & GLIBC_2.39 not found

# with bundled libsqlite3-sys:
# target=armv7-unknown-linux-musleabihf features=sqlite_bundled ./build_release.sh
# 3461632 = 3,4M   ->   1465792 = 1,4M
# 3461632 = 3,4M   ->   1465512 = 1,4M

# without bundled libsqlite3-sys:
# target=mipsel-unknown-linux-gnu features=basic ./build_release.sh
# fails to build: GLIBC_2.32, GLIBC_2.33, GLIBC_2.34 & GLIBC_2.39 not found

# with bundled libsqlite3-sys:
# target=mipsel-unknown-linux-gnu features=sqlite_bundled ./build_release.sh
# 5110952 = 4,9M   ->   1641332 = 1,6M
# doesn't start, output:
# /usr/bin/p2p_chat_example: line 1: EL@�g4p4: not found
# /usr/bin/p2p_chat_example: line 2: :��=1VZ,%5IN.P��2��H�a�d��U�3@�9�qk+gh�.��K�6���������E@��FR�0����L��8L�nW�N40: not found
# /usr/bin/p2p_chat_example: /usr/bin/p2p_chat_example: line 2: line 2: �ަ�~��O��5Eq���6�Vyc
# ���jO�^��G��S�������k�r�٪>5��(&��|���▒�w�D߈��̽��:!*?Ĺ�[�<t�}Gr���Z1ԭ�D<p���0>&óF�+d�CI-��D
# ���m�!�L��▒▒�: not found
# syntax error: unexpected word (expecting ")")

# with bundled libsqlite3-sys + stable compiler:
# stable=1 target=mipsel-unknown-linux-gnu features=sqlite_bundled ./build_release.sh
# zerocopy v0.8.26 fails to build: GLIBC_2.32, GLIBC_2.33, GLIBC_2.34 & GLIBC_2.39 not found

# stable=1 target=mips-unknown-linux-gnu features=sqlite_bundled ./build_release.sh

## 2026-04-03
# rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
# target=x86_64-unknown-linux-gnu features=basic ./build_release.sh

# $ stable=1 target=x86_64-unknown-linux-gnu features=basic bash build_release.sh
# Finished `release` profile [optimized] target(s) in 42.21s
# -rwxr-xr-x 2 user users 4770784 Apr  3 16:21 target/x86_64-unknown-linux-gnu/release/p2p_chat_example
# -rwxr-xr-x 2 user users 4.6M Apr  3 16:21 target/x86_64-unknown-linux-gnu/release/p2p_chat_example
#                        Ultimate Packer for eXecutables
#                           Copyright (C) 1996 - 2025
# UPX 5.0.2       Markus Oberhumer, Laszlo Molnar & John Reiser   Jul 20th 2025
# 
#         File size         Ratio      Format      Name
#    --------------------   ------   -----------   -----------
#    4770784 ->   1618124   33.92%   linux/amd64   p2p_chat_example              
# 
# Packed 1 file.
# -rwxr-xr-x 1 user users 1618124 Apr  3 16:21 target/x86_64-unknown-linux-gnu/release/p2p_chat_example
# -rwxr-xr-x 1 user users 1.6M Apr  3 16:21 target/x86_64-unknown-linux-gnu/release/p2p_chat_example
