#!/bin/sh
# Lamina validation environment bootstrap (idempotent).
# Installs Rust toolchain (rsproxy mirror) and Python deps, builds the bridge.
set -e
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v cargo >/dev/null 2>&1; then
  echo "[setup] installing rust via rsproxy..."
  curl -sSf --max-time 300 -o /tmp/rustup-init https://rsproxy.cn/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
  chmod +x /tmp/rustup-init
  RUSTUP_DIST_SERVER=https://rsproxy.cn RUSTUP_UPDATE_ROOT=https://rsproxy.cn/rustup \
    /tmp/rustup-init -y --profile minimal --default-toolchain stable
fi
mkdir -p "$HOME/.cargo"
cat > "$HOME/.cargo/config.toml" <<'CARGO'
[source.crates-io]
replace-with = 'rsproxy-sparse'
[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"
[net]
retry = 3
CARGO
echo "[setup] installing python deps..."
mkdir -p "$HOME/.pip"
printf '[global]\nindex-url = https://pypi.mirrors.msh.team/simple\n' > "$HOME/.pip/pip.conf"
pip install -q wfdb numpy scipy pandas matplotlib pytest 2>&1 | tail -1 || true
echo "[setup] building lamina bridge (target dir on local disk)..."
REPO="${1:-$HOME/lamina}"
export CARGO_TARGET_DIR="$HOME/.lamina_bridge-target"
cargo build --release --manifest-path "$REPO/validation/lamina_bridge/Cargo.toml"
mkdir -p "$REPO/validation/lamina_bridge/target/release"
cp "$HOME/.lamina_bridge-target/release/lamina_bridge" "$REPO/validation/lamina_bridge/target/release/" 2>/dev/null || true
echo "[setup] done: $(cargo --version)"
