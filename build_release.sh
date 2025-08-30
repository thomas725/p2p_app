# https://stackoverflow.com/questions/29008127/why-are-rust-executables-so-huge
# https://github.com/johnthagen/min-sized-rust

## build with debugging info telling which src file and line caused a panic:
#debug=1

z0="build-std=std,panic_unwind"
z1="build-std=std,panic_abort"
z2="build-std-features=panic_immediate_abort"
if [ -z ${debug+x} ] || [ $debug -ne 1 ]; then
  # -Zfmt-debug=none # don't use because it breaks debug printing structs which we use a lot! also, doesn't save that much..
  RUSTFLAGS="-Zlocation-detail=none"
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
  RUSTFLAGS="$RUSTFLAGS" $bin +nightly build -Z $z1 -Z $z2 --target $target --release --no-default-features -F $features
else
  echo "debug flag set: using release-with-debug profile + CARGO_PROFILE_RELEASE_DEBUG flag."
  CARGO_PROFILE_RELEASE_DEBUG=true
  profile='release-with-debug'
  # +nightly
  RUSTFLAGS="$RUSTFLAGS" $bin build --target $target --profile=release-with-debug --no-default-features -F $features
fi
ls -al target/$target/$profile/p2p_chat_example
if [ -z ${skip_upx+x} ] || [ $skip_upx -ne 1 ]; then
  upx --best --lzma target/$target/$profile/p2p_chat_example
  ls -al target/$target/$profile/p2p_chat_example
fi

## 2025-08-30 ~01:00: default release build = 4573480 bytes = 4,4MiB
# target=x86_64-unknown-linux-gnu features=basic ./build_release.sh
# 3357696 -> 1227248

## 2025-08-30 ~19:44: with migrations embedded:
# 3395272 -> 1237180

# target/x86_64-unknown-linux-gnu/release/p2p_chat_example
# DATABASE_URL=sqlite2.db target/x86_64-unknown-linux-gnu/release/p2p_chat_example

# skip_upx=1 target=x86_64-unknown-linux-gnu features=basic ./build_release.sh

