#! /bin/sh

set -e

export CARGO_HOME="`pwd`/.cargo"
export RUSTUP_HOME="`pwd`/.rustup"

case ${CI_TRACER_KIND} in
    "sw" | "hw" ) true;;
    *) echo "CI_TRACER_KIND must be set to either 'hw' or 'sw'"
       exit 1;;
esac


# Use the most recent successful ykrustc build.
tar jxf /opt/ykrustc-bin-snapshots/ykrustc-${CI_TRACER_KIND}-stage2-latest.tar.bz2
export PATH=`pwd`/ykrustc-stage2-latest/bin:${PATH}

RUSTFLAGS="-D warnings -C tracer=${CI_TRACER_KIND}" cargo test

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh
sh rustup.sh --default-host x86_64-unknown-linux-gnu --default-toolchain nightly -y --no-modify-path

export PATH=`pwd`/.cargo/bin/:$PATH

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh
sh rustup.sh --default-host x86_64-unknown-linux-gnu \
    --default-toolchain nightly \
    --no-modify-path \
    --profile minimal \
    -y

rustup toolchain install nightly --allow-downgrade --component rustfmt
cargo +nightly fmt --all -- --check

which cargo-deny | cargo install cargo-deny
cargo-deny check license
