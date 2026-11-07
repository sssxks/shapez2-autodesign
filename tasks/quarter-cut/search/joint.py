# /// script
# requires-python = ">=3.11"
# dependencies = ["ortools==9.15.6755"]
# ///
"""Joint native-interface placement and two-commodity routing experiment.

The Python layer constructs constraints; OR-Tools' native C++ solver performs
search. Candidate positions and rotations are bounded by the selected seed,
translation radius and half-turn option. Infeasibility applies to that model.
"""

from ortools.sat.python import cp_model
from collections import defaultdict
from pathlib import Path
import itertools, json, os

import argparse

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("seed", type=str)
parser.add_argument("output", type=str)
parser.add_argument("--seconds", type=float, default=300)
parser.add_argument("--workers", type=int, default=8)
parser.add_argument("--radius", type=int, default=1)
parser.add_argument("--half-turn", action="store_true")
parser.add_argument("--chunks", type=int, choices=[2, 3], default=2)
parser.add_argument("--witness", type=str)
parser.add_argument("--backend", choices=["cp-sat", "scip"], default="cp-sat")
args = parser.parse_args()
with open(args.seed, encoding="utf-8") as stream:
    layers = json.load(stream)
dest = args.output
Path(dest).parent.mkdir(parents=True, exist_ok=True)
witness = None
if args.witness:
    with open(args.witness, encoding="utf-8") as stream:
        witness = {
            (c, tuple(a), tuple(b)): amount for c, a, b, amount in json.load(stream)
        }
witness_seen = set()
original = os.environ.get("QUARTER_PORTS", "original") == "original"
D = [(1, 0), (0, 1), (-1, 0), (0, -1)]
foot = [(0, 1), (0, 2), (1, 1), (1, 2)] + [
    (x, y) for x in range(2, 5) for y in range(4)
]


def rot(p, r):
    x, y = p
    return [(x, y), (-y, x), (-x, -y), (y, -x)][r]


def pos(c, p, z):
    x, y = rot(p, c["r"])
    return x + c["x"], y + c["y"], z


floor = {
    (x, y, z)
    for x in range(3, args.chunks * 20 - 3)
    for y in range(3, 17)
    for z in range(3)
}
poses = {}
fixed = set()
for z, layer in enumerate(layers):
    for index, c in enumerate(layer):
        variants = []
        bodies = []
        for dx, dy, turn in itertools.product(
            range(-args.radius, args.radius + 1),
            range(-args.radius, args.radius + 1),
            [0, 2] if args.half_turn else [0],
        ):
            shift = rot((4, 3), c["r"]) if turn else (0, 0)
            ref = dict(
                c,
                x=c["x"] + dx + shift[0],
                y=c["y"] + dy + shift[1],
                r=(c["r"] + turn) % 4,
            )
            body = {pos(ref, p, z) for p in foot}
            if not body <= floor or any(
                (
                    (x + 20, 3, z)
                    if args.chunks == 3
                    else (x, 3, z)
                    if original
                    else (3, x, z)
                )
                in body
                for x in range(8, 12)
            ):
                continue
            variants.append(ref)
            bodies.append(body)
        poses[z, index] = variants
        fixed.update(set.intersection(*bodies))
free = floor - fixed
external_inputs = [((x, 2, z), 1) for z in range(3) for x in range(8, 12)]
external_outputs = [
    ((x, 17, z), 1) for x in [8, 9, 10, 11, 28, 29, 30, 31] for z in range(3)
] + [
    ((x, y, z), d) for x, d in [(2, 2), (37, 0)] for y in range(8, 12) for z in range(3)
]
if not original:
    external_inputs = [((2, y, z), 0) for z in range(3) for y in range(8, 12)]
    external_outputs = [
        ((x, y, z), d)
        for y, d in [(2, 3), (17, 1)]
        for x in [8, 9, 10, 11, 28, 29, 30, 31]
        for z in range(3)
    ]
if args.chunks == 3:
    external_inputs = [((x, 2, z), 1) for z in range(3) for x in range(28, 32)]
    external_outputs = [
        ((x, y, z), d)
        for y, d in [(2, 3), (17, 1)]
        for x in [8, 9, 10, 11, 48, 49, 50, 51]
        for z in range(3)
    ]
inputmap = dict(external_inputs)
outputmap = dict(external_outputs)
m = cp_model.CpModel()
incoming = defaultdict(list)
outgoing = defaultdict(list)
fanin = defaultdict(list)
capacity = defaultdict(list)
emissions = defaultdict(list)
sinks = defaultdict(list)
options = []
arcs = []
ys = []
jumps = defaultdict(list)


def node(p, d, c=False):
    return (*p, d, c)


def flowvar(y, name):
    f = m.new_int_var(0, 4, name)
    m.add(f >= y)
    m.add(f <= 4 * y)
    return f


for commodity in range(2):
    states = [node(p, d, c) for p in free for d in range(4) for c in [False, True]]
    if commodity == 0:
        states += [node(p, d) for p, d in external_inputs]
    else:
        states += [node(p, d) for p, d in external_outputs]
    for a in states:
        p = a[:3]
        heading = a[3]
        caught = a[4]
        if p in outputmap:
            continue
        locked = caught or p in inputmap
        for z in range(3):
            lift = z != p[2]
            if locked and lift:
                continue
            shaft = [
                (p[0], p[1], k)
                for k in range(min(z, p[2]), max(z, p[2]) + 1)
                if k != p[2]
            ]
            if any(t not in free for t in shaft):
                continue
            for d, (dx, dy) in enumerate(D):
                if locked and d != heading or not lift and (d - heading) % 4 == 2:
                    continue
                for length in range(1, 6):
                    jump = length > 1
                    if jump and (lift or locked or d != heading):
                        continue
                    q = (p[0] + dx * length, p[1] + dy * length, z)
                    if q not in free and not (
                        commodity == 1
                        and q in outputmap
                        and not jump
                        and d == outputmap[q]
                    ):
                        continue
                    b = node(q, d, jump)
                    i = len(arcs)
                    y = m.new_bool_var(f"e{i}")
                    f = flowvar(y, f"f{i}") if commodity == 0 else y
                    if witness is not None:
                        key = (commodity, a, b)
                        m.add(y == int(key in witness))
                        m.add(f == witness.get(key, 0))
                        if key in witness:
                            witness_seen.add(key)
                    outgoing[commodity, a].append(f)
                    incoming[commodity, b].append(f)
                    fanin[commodity, q].append(y)
                    capacity[p].append(y)
                    for t in shaft:
                        capacity[t].append(y)
                    arcs.append((commodity, a, b))
                    ys.append(y)
                    if jump:
                        jumps[p, q, d].append(y)
if witness is not None and set(witness) != witness_seen:
    raise ValueError(
        f"Native witness contains unsupported moves: {list(set(witness) - witness_seen)[:5]}"
    )
# Optional native interfaces. One choice per rigid machine body.
input_groups = defaultdict(list)
for z, layer in enumerate(layers):
    for index, original in enumerate(layer):
        choices = []
        for poseindex, ref in enumerate(poses[z, index]):
            body = {pos(ref, p, z) for p in foot}
            posechoices = []
            posev = m.new_bool_var(f"pose{z}_{index}_{poseindex}")
            for p in body - fixed:
                capacity[p].append(posev)
            for mirror, out, split in itertools.product(
                [False, True], range(4), [-1, 0, 1, 2, 3]
            ):
                localop = [(5, 3), (4, 4), (5, 1), (5, 2)][out]
                if mirror:
                    localop = (localop[0], 3 - localop[1])
                op = pos(ref, localop, z)
                od = (ref["r"] + ((3 if mirror else 1) if out == 1 else 0)) % 4
                ip = pos(ref, (-1, 1 if mirror else 2), z)
                inc = (ref["r"] + ([0, 0, 3, 1][split] if split >= 0 else 0)) % 4
                cont = (ref["r"] + ([3, 1, 3, 1][split] if split >= 0 else 0)) % 4
                a = (ip[0] - D[inc][0], ip[1] - D[inc][1], z) if split >= 0 else ip
                b = (ip[0] + D[cont][0], ip[1] + D[cont][1], z) if split >= 0 else None
                required = [a, op] + ([ip, b] if b else [])
                if len(set(required)) != len(required) or any(
                    p in body for p in required
                ):
                    continue
                if any(
                    p not in free
                    and not (p == op and p in outputmap and od == outputmap[p])
                    and not (p == a and p in inputmap and inc == inputmap[p])
                    for p in required
                ):
                    continue
                v = m.new_bool_var(f"cell{z}_{index}_{len(choices)}")
                m.add_hint(
                    v,
                    int(
                        ref["x"] == original["x"]
                        and ref["y"] == original["y"]
                        and ref["r"] == original["r"]
                        and mirror == original.get("mirror", False)
                        and out == original["out"]
                        and split == original["split"]
                    ),
                )
                choices.append(v)
                posechoices.append(v)
                emissions[1, node(op, od)].append(v)
                fanin[1, op].append(v)
                input_groups[
                    z, index, poseindex, mirror, split, a, b, ip, inc, cont
                ].append(v)
                c = dict(
                    ref,
                    mirror=mirror,
                    out=out,
                    split=split,
                    a=a[:2],
                    ad=inc,
                    b=b[:2] if b else None,
                    bd=cont,
                    op=op[:2],
                    od=od,
                )
                options.append((v, z, index, c, a, b, op, inc, cont, od))
            m.add(sum(posechoices) == posev)
        m.add_exactly_one(choices)
        print("cell", z, index, "choices", len(choices), flush=True)
port_groups = defaultdict(list)
for group_index, (key, choices) in enumerate(input_groups.items()):
    _, _, _, _, _, a, b, ip, inc, cont = key
    enabled = m.new_bool_var(f"supply_choice{group_index}")
    m.add(sum(choices) == enabled)
    amount = flowvar(enabled, f"supply_amount{group_index}")
    if b is not None:
        m.add(amount >= 2 * enabled)
        capacity[ip].append(enabled)
        emissions[0, node(b, cont)].append(amount - enabled)
        fanin[0, b].append(enabled)
    else:
        m.add(amount == enabled)
    port_groups[a, inc].append((enabled, amount))
for group_index, ((a, inc), members) in enumerate(port_groups.items()):
    receives = []
    flags = []
    for heading, caught in itertools.product(range(4), [False, True]):
        if (inc - heading) % 4 == 2 or caught and heading != inc:
            continue
        selected = m.new_bool_var(f"sink{group_index}_{heading}_{caught}")
        amount = flowvar(selected, f"sink_amount{group_index}_{heading}_{caught}")
        sinks[0, node(a, heading, caught)].append(amount)
        capacity[a].append(selected)
        receives.append(amount)
        flags.append(selected)
    m.add(sum(receives) == sum(amount for _, amount in members))
    m.add(sum(flags) == sum(enabled for enabled, _ in members))
for p, d in external_inputs:
    emissions[0, node(p, d)].append(4)
    fanin[0, p].append(1)
for p, d in external_outputs:
    sinks[1, node(p, d)].append(1)
    capacity[p].append(1)
for key in set(incoming) | set(outgoing) | set(emissions) | set(sinks):
    m.add(
        sum(incoming[key]) + sum(emissions[key]) == sum(outgoing[key]) + sum(sinks[key])
    )
for values in capacity.values():
    m.add(sum(values) <= 1)
for values in fanin.values():
    m.add(sum(values) <= 1)
by_receiver = defaultdict(list)
for key in jumps:
    by_receiver[key[1], key[2]].append(key)
conflicts = set()
for key in jumps:
    p, q, d = key
    dx, dy = D[d]
    for k in range(2, 6):
        receiver = (p[0] + dx * k, p[1] + dy * k, p[2])
        if receiver == q:
            continue
        for other in by_receiver[receiver, d]:
            if other != key:
                conflicts.add(tuple(sorted((key, other))))
for key, other in conflicts:
    m.add(sum(jumps[key]) + sum(jumps[other]) <= 1)
print(
    "JOINT MODEL",
    len(options),
    "options",
    len(arcs),
    "transport arcs",
    len(conflicts),
    "jump conflicts",
    flush=True,
)
if args.backend == "scip":
    from mip import MipSolver

    s = MipSolver(args.seconds, args.workers)
else:
    s = cp_model.CpSolver()
    s.parameters.max_time_in_seconds = args.seconds
    s.parameters.num_search_workers = args.workers
    s.parameters.log_search_progress = True
status = s.solve(m)
print("RESULT", s.status_name(status), flush=True)
with open(dest + ".status.json", "w", encoding="utf-8") as stream:
    json.dump(
        {
            "status": s.status_name(status),
            "seconds": s.wall_time,
            "seed": args.seed,
            "radius": args.radius,
            "half_turn": args.half_turn,
            "chunks": args.chunks,
            "backend": args.backend,
            "statistics": s.response_stats(),
        },
        stream,
        indent=2,
    )
if status in [cp_model.OPTIMAL, cp_model.FEASIBLE]:
    selected = [o for o in options if s.value(o[0])]
    result = [[None] * 16 for _ in range(3)]
    for o in selected:
        result[o[1]][o[2]] = o[3]
    json.dump(result, open(dest + ".json", "w"))
    chosen = {(a[0], a[1]): a[2] for y, a in zip(ys, arcs) if s.value(y)}
    roots = [(p, d) for p, d in external_inputs]
    selected.sort(key=lambda o: o[1] * 16 + o[2])
    conts = [o for o in selected if o[5] is not None]
    sources = roots + [(o[5], o[8]) for o in conts] + [(o[6], o[9]) for o in selected]
    # Rust reassigns initial output targets; consume coordinates in the importer.
    routes = []
    for i, (p, d) in enumerate(sources):
        c = int(i >= 48)
        cur = node(p, d)
        steps = [cur]
        seen = set()
        while (c, cur) in chosen:
            assert cur not in seen
            seen.add(cur)
            cur = chosen[c, cur]
            steps.append(cur)
        routes.append({"source": i, "destination": cur[:3], "steps": steps})
    json.dump(routes, open(dest + ".routes.json", "w"))
    print("SAVED", dest, flush=True)
