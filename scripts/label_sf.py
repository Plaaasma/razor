#!/usr/bin/env python3
"""Relabel positions with a strong teacher (Stockfish) — the SF-teacher
distillation step. Reads bullet-text `fen | cp | result`, replaces cp with the
teacher's fixed-node white-relative score, keeps the result.

Cross-platform: teacher binary from $RAZOR_SF (else the local Windows default).
Each worker writes its own shard incrementally (flush every 5k) so partial data
survives a stop/crash. Concat the shards afterwards.

Usage: RAZOR_SF=/path/to/stockfish python3 label_sf.py <in.txt> <out_prefix> [nodes=50000] [n_procs]
  -> writes <out_prefix>.<idx>.txt per worker.
"""
import os, sys, subprocess
from concurrent.futures import ProcessPoolExecutor

SF = os.environ.get("RAZOR_SF", r"H:\RazorBot\play\Stockfish-18.exe")
NODES = int(sys.argv[3]) if len(sys.argv) > 3 else 50_000
NPROC = int(sys.argv[4]) if len(sys.argv) > 4 else (os.cpu_count() or 24)


def stm_is_white(fen):
    return fen.split()[1] == "w"


def label_shard(args):
    idx, lines, nodes, out_path = args
    p = subprocess.Popen([SF], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         text=True, bufsize=1)

    def send(s):
        p.stdin.write(s + "\n"); p.stdin.flush()

    send("uci")
    while not p.stdout.readline().startswith("uciok"):
        pass
    send("setoption name Threads value 1")
    send("setoption name Hash value 64")
    done = 0
    with open(out_path, "w") as fout:
        for line in lines:
            parts = line.rstrip("\n").split("|")
            if len(parts) != 3:
                continue
            fen, _old, result = (x.strip() for x in parts)
            send("position fen " + fen)
            send(f"go nodes {nodes}")
            score_cp = None
            mate = None
            while True:
                ln = p.stdout.readline()
                if not ln:
                    break
                if ln.startswith("info") and " score " in ln:
                    tok = ln.split()
                    i = tok.index("score")
                    if tok[i + 1] == "cp":
                        score_cp = int(tok[i + 2]); mate = None
                    elif tok[i + 1] == "mate":
                        mate = int(tok[i + 2])
                if ln.startswith("bestmove"):
                    break
            if mate is not None:
                score_cp = 30000 - abs(mate) if mate > 0 else -(30000 - abs(mate))
            if score_cp is None:
                continue
            white_cp = score_cp if stm_is_white(fen) else -score_cp
            fout.write(f"{fen} | {white_cp} | {result}\n")
            done += 1
            if done % 5000 == 0:
                fout.flush()
                if idx == 0:
                    print(f"progress: ~{done * NPROC} / {len(lines) * NPROC} positions", flush=True)
    send("quit")
    p.wait()
    return done


def main():
    inp, prefix = sys.argv[1], sys.argv[2]
    with open(inp) as f:
        lines = f.readlines()
    n = len(lines)
    shards = [lines[i::NPROC] for i in range(NPROC)]
    args = [(i, shards[i], NODES, f"{prefix}.{i}.txt") for i in range(NPROC)]
    print(f"teacher={SF}\nlabeling {n} positions @ {NODES} nodes across {NPROC} workers", flush=True)
    with ProcessPoolExecutor(max_workers=NPROC) as ex:
        counts = list(ex.map(label_shard, args))
    print(f"done: wrote {sum(counts)} labeled positions to {prefix}.*.txt", flush=True)


if __name__ == "__main__":
    main()
