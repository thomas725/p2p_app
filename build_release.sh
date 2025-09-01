# https://stackoverflow.com/questions/29008127/why-are-rust-executables-so-huge
# https://github.com/johnthagen/min-sized-rust

## build with debugging info telling which src file and line caused a panic:
#debug=1

project="p2p_chat_example"
z0="build-std=std,panic_unwind"
z1="build-std=std,panic_abort"
z2="build-std-features=panic_immediate_abort"
if [ -z ${debug+x} ] || [ $debug -ne 1 ]; then
  # -Zfmt-debug=none # don't use because it breaks debug printing structs which we use a lot! also, doesn't save that much..
  if [ -z ${stable+x} ] || [ $stable -ne 1 ]; then
    # error: the option `Z` is only accepted on the nightly compiler
    RUSTFLAGS="-Zlocation-detail=none"
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
    RUSTFLAGS="$RUSTFLAGS" $bin +nightly build -Z $z1 -Z $z2 --target $target --release --no-default-features -F $features
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
  # > UPX-5.0 wants memfd_create(), or needs /dev/shm(,O_TMPFILE,)
  # so we use latest version 4 instead:
  # cd /opt ; wget https://github.com/upx/upx/releases/download/v4.2.4/upx-4.2.4-amd64_linux.tar.xz ; tar xf upx-4.2.4-amd64_linux.tar.xz
  /opt/upx-4.2.4-amd64_linux/upx --best --lzma target/$target/$profile/$project
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
#uncompressed 3,4MiB binary fails becuase of some protocol not being supported, I believe it's about quic:
# INFO quinn_udp::imp::gso: GSO disabled: kernel too old (3.4.39); need 4.18+
# Error: Protocol not available (os error 92)
#removed quic to find siginficant smaller binary size:
#2583120 ->   112959 and a different error message:
# Error: Multiaddr is not supported: /ip4/0.0.0.0/udp/0/quic-v1

# trying debug version to see where we need to gracefully handle errors:
# debug=1 target=armv7-unknown-linux-musleabihf features=basic ./build_release.sh
#49568564 ->  10329136

# target=aarch64-unknown-linux-gnu features=basic ./build_release.sh