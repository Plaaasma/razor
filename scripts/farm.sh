#!/usr/bin/env bash
# Razor SF-teacher labeling farm node — self-contained.
# Builds Razor, fetches a strong teacher (Stockfish), generates self-play
# positions, then relabels them with the teacher at high nodes. Output:
# labeled_<SEED_BASE>.txt  (bullet-text `fen | sf_cp | result`), which you
# collect and merge centrally for training.
#
# Run from the repo root on a Linux CPU box:
#   SEED_BASE=2000 NODES=50000 PER=200000 bash scripts/farm.sh
#
# Tuning:
#   PER       positions per core to gen+label (200k ~= 8h/core-batch @ 50k nodes)
#   NODES     teacher nodes per position (50000 strong; 25000 ~2x throughput)
#   SEED_BASE disjoint per box (box1=2000, box2=3000, local uses 1000) so games
#             never duplicate across machines.
set -euo pipefail

NPROC=${NPROC:-$(nproc)}
SEED_BASE=${SEED_BASE:-2000}
PER=${PER:-200000}
NODES=${NODES:-50000}

echo "== building razor =="
RUSTFLAGS="-C target-cpu=native" cargo build --release

if [ -z "${RAZOR_SF:-}" ]; then
  ARCH=$(uname -m)
  if [ "$ARCH" = "x86_64" ]; then
    echo "== fetching teacher (Stockfish x86-64-avx2) =="
    curl -fsSL https://github.com/official-stockfish/Stockfish/releases/latest/download/stockfish-ubuntu-x86-64-avx2.tar -o sf.tar
    tar xf sf.tar
    RAZOR_SF="$(pwd)/stockfish/stockfish-ubuntu-x86-64-avx2"
    chmod +x "$RAZOR_SF"
  else
    # aarch64 (Spark / ARM rentals / Apple Silicon): build Stockfish from source.
    SFARCH=armv8
    [ "$(uname -s)" = "Darwin" ] && SFARCH=apple-silicon
    echo "== building teacher (Stockfish $SFARCH) from source =="
    rm -rf Stockfish && git clone --depth 1 https://github.com/official-stockfish/Stockfish.git
    ( cd Stockfish/src && make -j"$NPROC" build ARCH=$SFARCH >/dev/null 2>&1 )
    RAZOR_SF="$(pwd)/Stockfish/src/stockfish"
  fi
fi
export RAZOR_SF
echo "teacher: $RAZOR_SF"

echo "== generating $((PER*NPROC)) self-play positions ($NPROC workers) =="
for i in $(seq 0 $((NPROC-1))); do
  DG_NODES=5000 ./target/release/razor datagen "pos_$((SEED_BASE+i)).txt" "$PER" "$((SEED_BASE+i))" &
done
wait
cat pos_*.txt > pool_${SEED_BASE}.txt && rm -f pos_*.txt
echo "pool: $(wc -l < pool_${SEED_BASE}.txt) positions"

echo "== labeling with teacher @ $NODES nodes (the slow step) =="
python3 scripts/label_sf.py "pool_${SEED_BASE}.txt" "sflab_${SEED_BASE}" "$NODES" "$NPROC"
cat sflab_${SEED_BASE}.*.txt > "labeled_${SEED_BASE}.txt" && rm -f sflab_${SEED_BASE}.*.txt
echo "== DONE: labeled_${SEED_BASE}.txt ($(wc -l < labeled_${SEED_BASE}.txt) positions) — collect this file =="
