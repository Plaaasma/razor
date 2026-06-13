"""Convert one gen1 shard from stm-relative to white-relative labels.
gen1 was generated before the datagen source was fixed to emit white-relative
directly. Line format in: `<fen> | <stm_cp> | <stm_result>`.
Out: `<fen> | <white_cp> | <white_result>` (bullet's required form).

Flip rule: if side-to-move in the FEN is black, negate score and complement
result (1 - r). White-to-move lines pass through unchanged.
"""
import sys

inp, out = sys.argv[1], sys.argv[2]
n = 0
with open(inp, "r", encoding="utf-8") as f, open(out, "w", encoding="utf-8", newline="\n") as o:
    for line in f:
        parts = line.split(" | ")
        if len(parts) != 3:
            continue
        fen, cp, res = parts
        stm = fen.split()[1]  # 'w' or 'b'
        if stm == "b":
            cp = str(-int(cp))
            res = f"{1.0 - float(res):.1f}"
        else:
            res = res.strip()
        o.write(f"{fen} | {cp} | {res}\n")
        n += 1
print(f"{inp}: {n} lines", flush=True)
