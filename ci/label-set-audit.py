#!/usr/bin/env python3
"""Check a label set against its source shards: the checks in docs/datasets.md,
"Auditing a label set". Run through ci/label-set-audit.sh, which downloads both
sides and builds the `relabel` this needs. Standard library only.

    label-set-audit.py --data DIR --work DIR --label-id ID --engine-sha SHA
                       --relabel BIN --dataset s3://... --json FILE
                       [--seed N] [--per-run N] [--negatives N] [--skip-determinism]

DIR holds source/ and labels/, each gen=N/run=R/task=T/{manifests,positions}/.
The two checks a download can't make are left to the auditor and listed in the
output: the relabel role's write policy, and the source's shard counts against
docs/datasets.md.
"""
import argparse
import collections
import glob
import gzip
import hashlib
import json
import math
import multiprocessing
import os
import random
import re
import shutil
import subprocess
import sys

RESULT = {"1.0": 1.0, "0.5": 0.5, "0.0": 0.0}
# Plausibility is a guide, not a bar: these only catch what a wrong point of
# view would produce, which is agreement near chance or correlation near zero.
MIN_SIGN_AGREEMENT = 70.0
MIN_CORRELATION = 0.5
# Where a positions line's sigmoid is taken; the training target's scale.
SCALE = 400
SUMMED = """records changed sum_abs sign_same sign_opposite sign_zero sum_new sum_old sum_new2 sum_old2 sum_new_old
abs_new abs_old beyond_clamp mates loss_new loss_old hit_new hit_old decisive source_sha_checked""".split()


def shards(root):
    pattern = re.compile(r"/(gen=\d+/run=[^/]+/task=[^/]+)/manifests/([^/]+)\.json$")
    found = set()
    for path in glob.glob(f"{root}/gen=*/run=*/task=*/manifests/*.json"):
        match = pattern.search(path)
        found.add(f"{match[1]}/{match[2]}")
    return found


def positions(root):
    pattern = re.compile(r"/(gen=\d+/run=[^/]+/task=[^/]+)/positions/([^/]+)\.txt\.gz$")
    return {
        f"{m[1]}/{m[2]}": os.path.getsize(path)
        for path in glob.glob(f"{root}/gen=*/run=*/task=*/positions/*.txt.gz")
        for m in [pattern.search(path)]
    }


def inventory(data):
    """Check 1: every source shard, and nothing else, on both sides."""
    source, labels = shards(f"{data}/source"), shards(f"{data}/labels")
    source_files, label_files = positions(f"{data}/source"), positions(f"{data}/labels")
    problems = {
        "source shards without a relabelled manifest": source - labels,
        "relabelled manifests without a source shard": labels - source,
        "relabelled manifests without positions": labels - set(label_files),
        "relabelled positions without a manifest": set(label_files) - labels,
        "source manifests without positions": source - set(source_files),
        "empty relabelled positions files": {k for k, size in label_files.items() if size == 0},
    }
    return sorted(source), {k: sorted(v) for k, v in problems.items() if v}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def sigmoid(score):
    return 1 / (1 + math.exp(-max(min(score, 4000), -4000) / SCALE))


def check_shard(job):
    """Checks 2 and 3, and check 4's sums, over one shard."""
    rel, data, label_id, engine_sha, nodes = job
    directory, shard = rel.rsplit("/", 1)
    out = {"rel": rel, "gen": rel.split("/")[0], "run": rel.split("/")[1], "fail": []}
    fail = out["fail"]
    try:
        with open(f"{data}/labels/{directory}/manifests/{shard}.json") as f:
            new_manifest = json.load(f)
        with open(f"{data}/source/{directory}/manifests/{shard}.json") as f:
            old_manifest = json.load(f)
        with open(f"{data}/labels/{directory}/positions/{shard}.txt.gz", "rb") as f:
            new_gz = f.read()
        with open(f"{data}/source/{directory}/positions/{shard}.txt.gz", "rb") as f:
            old_gz = f.read()
        new_text, old_text = gzip.decompress(new_gz), gzip.decompress(old_gz)
        block = new_manifest["relabel"]
        stats = block["stats"]
    except (OSError, ValueError, KeyError, EOFError) as e:
        fail.append(f"read: {e}")
        return out
    if block["label_id"] != label_id:
        fail.append(f"label id {block['label_id']}")
    if block["engine_sha"] != engine_sha:
        fail.append(f"engine {block['engine_sha']}")
    if block["nodes"] != nodes:
        fail.append(f"nodes {block['nodes']}")
    if not label_id.startswith(block["net"]["id"]):
        fail.append(f"net {block['net']['id']}")
    out["net_sha"] = block["net"]["sha256"]
    # Everything but the relabel block and the checksums is the original's.
    for field in old_manifest:
        new_value, old_value = new_manifest.get(field), old_manifest[field]
        if field == "sha256":
            continue
        # relabel's JSON round trip can move this float's last digit, and nothing
        # reads it: 121 of 36,398 shards in N7, by at most 7e-15 of a second.
        if field == "seconds" and isinstance(new_value, float) and math.isclose(new_value, old_value, abs_tol=1e-9):
            continue
        if new_value != old_value:
            fail.append(f"manifest field {field} differs from the source")
    new_sums = (sha256(new_gz), sha256(new_text))
    old_sums = (sha256(old_gz), sha256(old_text))
    if new_manifest["sha256"][0] not in new_sums:
        fail.append("positions SHA-256 doesn't match the manifest")
    if block["source_positions_sha256"] not in old_sums:
        fail.append("source SHA-256 doesn't match the source file")
    if "sha256" in old_manifest:
        out["source_sha_checked"] = 1
        if old_manifest["sha256"][0] not in old_sums:
            fail.append("source file doesn't match its original manifest")
        if new_manifest["sha256"][1] != old_manifest["sha256"][1]:
            fail.append("games SHA-256 differs from the source")
    new_lines, old_lines = new_text.decode().splitlines(), old_text.decode().splitlines()
    counts = {len(new_lines), len(old_lines), new_manifest["positions"], old_manifest["positions"], stats["records"]}
    if len(counts) != 1:
        fail.append(f"record counts differ: {sorted(counts)}")
    t = collections.Counter()
    largest = 0
    mismatched = 0
    for new_line, old_line in zip(new_lines, old_lines):
        try:
            new_fen, new_score, new_result = new_line.split(" | ")
            old_fen, old_score, old_result = old_line.split(" | ")
            x, y = int(new_score), int(old_score)
            result = RESULT[new_result]
        except (ValueError, KeyError):
            mismatched += 1
            continue
        if new_fen != old_fen or new_result != old_result:
            mismatched += 1
            continue
        t["records"] += 1
        change = abs(x - y)
        if change:
            t["changed"] += 1
            t["sum_abs"] += change
            largest = max(largest, change)
        if x == 0 or y == 0:
            t["sign_zero"] += 1
        elif (x > 0) == (y > 0):
            t["sign_same"] += 1
        else:
            t["sign_opposite"] += 1
        t["sum_new"] += x
        t["sum_old"] += y
        t["sum_new2"] += x * x
        t["sum_old2"] += y * y
        t["sum_new_old"] += x * y
        t["abs_new"] += abs(x)
        t["abs_old"] += abs(y)
        t["beyond_clamp"] += abs(x) > 3000
        p, q = sigmoid(x), sigmoid(y)
        t["loss_new"] -= result * math.log(p + 1e-12) + (1 - result) * math.log(1 - p + 1e-12)
        t["loss_old"] -= result * math.log(q + 1e-12) + (1 - result) * math.log(1 - q + 1e-12)
        if result != 0.5:
            t["decisive"] += 1
            t["hit_new"] += x != 0 and (x > 0) == (result == 1.0)
            t["hit_old"] += y != 0 and (y > 0) == (result == 1.0)
    if mismatched:
        fail.append(f"{mismatched} records differ in FEN or result, or don't parse")
    elif (t["changed"], t["sum_abs"], largest) != (stats["changed"], stats["sum_abs_change"], stats["max_abs_change"]):
        fail.append("change statistics don't match the manifest")
    t["mates"] = stats["mates_clamped"]
    t["source_sha_checked"] = out.get("source_sha_checked", 0)
    out["totals"] = dict(t)
    out["largest"] = largest
    return out


def report(key, t):
    n = max(t["records"], 1)
    denominator = (n * t["sum_new2"] - t["sum_new"] ** 2) * (n * t["sum_old2"] - t["sum_old"] ** 2)
    correlation = (n * t["sum_new_old"] - t["sum_new"] * t["sum_old"]) / math.sqrt(denominator) if denominator > 0 else 0.0
    signed = t["sign_same"] + t["sign_opposite"]
    row = {
        "shards": t["shards"],
        "records": t["records"],
        "changed_pct": 100 * t["changed"] / n,
        "mean_abs_change": t["sum_abs"] / n,
        "largest_change": t["largest"],
        "mates_clamped": t["mates"],
        "beyond_clamp": t["beyond_clamp"],
        "sign_agreement_pct": 100 * t["sign_same"] / signed if signed else 0.0,
        "records_with_a_zero_score": t["sign_zero"],
        "correlation": correlation,
        "mean_abs_new": t["abs_new"] / n,
        "mean_abs_old": t["abs_old"] / n,
        "result_loss_new": t["loss_new"] / n,
        "result_loss_old": t["loss_old"] / n,
        "decisive_sign_new_pct": 100 * t["hit_new"] / max(t["decisive"], 1),
        "decisive_sign_old_pct": 100 * t["hit_old"] / max(t["decisive"], 1),
        "source_checksums_checked": t["source_sha_checked"],
    }
    print(
        f"{key}: {row['shards']} shards, {row['records']} records, changed {row['changed_pct']:.2f}%, "
        f"mean |change| {row['mean_abs_change']:.1f}, largest {row['largest_change']}, mates {row['mates_clamped']}, "
        f"beyond 3000 {row['beyond_clamp']}, sign {row['sign_agreement_pct']:.2f}%, correlation {row['correlation']:.4f}, "
        f"mean |score| new {row['mean_abs_new']:.0f} old {row['mean_abs_old']:.0f}, "
        f"result loss new {row['result_loss_new']:.4f} old {row['result_loss_old']:.4f}, "
        f"decisive sign new {row['decisive_sign_new_pct']:.2f}% old {row['decisive_sign_old_pct']:.2f}%, "
        f"source checksums {row['source_checksums_checked']}"
    )
    return row


def determinism(args, all_shards, nodes):
    """Check 5: a seeded sample of every run re-relabelled, byte for byte, and
    a negative control: the same check must fail at half the nodes."""
    runs = collections.defaultdict(list)
    for rel in all_shards:
        runs[rel.split("/")[1]].append(rel)
    rng = random.Random(args.seed)
    sample = [x for run in sorted(runs) for x in rng.sample(runs[run], min(args.per_run, len(runs[run])))]
    negative = rng.sample(sample, min(args.negatives, len(sample)))
    data = args.data

    def relabel(chosen, budget, tag):
        root = f"{args.work}/{tag}"
        shutil.rmtree(root, ignore_errors=True)
        for rel in chosen:
            directory, shard = rel.rsplit("/", 1)
            for sub in ("manifests", "positions", "games"):
                os.makedirs(f"{root}/in/{directory}/{sub}", exist_ok=True)
            shutil.copy(f"{data}/source/{directory}/manifests/{shard}.json", f"{root}/in/{directory}/manifests/")
            shutil.copy(f"{data}/source/{directory}/positions/{shard}.txt.gz", f"{root}/in/{directory}/positions/")
            subprocess.run(
                ["aws", "s3", "cp", "--only-show-errors", f"{args.dataset}/{directory}/games/{shard}.jsonl.gz",
                 f"{root}/in/{directory}/games/{shard}.jsonl.gz"],
                check=True,
            )
        subprocess.run(
            [args.relabel, "--in", f"{root}/in", "--out", f"{root}/out", "--net", "default", "--nodes", str(budget),
             "--engine-sha", args.engine_sha],
            check=True,
        )
        return f"{root}/out"

    def read(path):
        with open(path, "rb") as f:
            return f.read()

    def manifest(path):
        with open(path) as f:
            m = json.load(f)
        m["relabel"].pop("seconds", None)
        return m

    out = relabel(sample, nodes, "same-nodes")
    identical, manifests, differ = 0, 0, []
    for rel in sample:
        directory, shard = rel.rsplit("/", 1)
        same = read(f"{out}/{directory}/positions/{shard}.txt.gz") == read(f"{data}/labels/{directory}/positions/{shard}.txt.gz")
        identical += same
        if not same:
            differ.append(rel)
        manifests += manifest(f"{out}/{directory}/manifests/{shard}.json") == manifest(f"{data}/labels/{directory}/manifests/{shard}.json")
    out = relabel(negative, max(nodes // 2, 1), "half-nodes")
    control = sum(
        read(f"{out}/{d}/positions/{s}.txt.gz") != read(f"{data}/labels/{d}/positions/{s}.txt.gz")
        for d, s in (rel.rsplit("/", 1) for rel in negative)
    )
    return {
        "seed": args.seed,
        "runs": len(runs),
        "sample": sample,
        "identical": identical,
        "manifests_equal": manifests,
        "differ": differ,
        "negative_control": negative,
        "negative_control_differ": control,
        "passed": identical == len(sample) and manifests == len(sample) and control == len(negative),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    for flag in ("--data", "--work", "--label-id", "--engine-sha", "--relabel", "--dataset", "--json"):
        parser.add_argument(flag, required=True)
    parser.add_argument("--seed", type=int, default=20261105)
    parser.add_argument("--per-run", type=int, default=6)
    parser.add_argument("--negatives", type=int, default=3)
    parser.add_argument("--skip-determinism", action="store_true")
    args = parser.parse_args()
    nodes = int(args.label_id.rsplit("-n", 1)[1])

    print("==> 1. inventory")
    all_shards, problems = inventory(args.data)
    for name, items in problems.items():
        print(f"FAIL {name}: {len(items)}, for example {items[:3]}")
    print(f"{len(all_shards)} source shards; {'FAIL' if problems else 'PASS'}")

    print("==> 2 and 3. every record, provenance and checksums")
    totals = collections.defaultdict(collections.Counter)
    failures = []
    net_shas = set()
    jobs = [(rel, args.data, args.label_id, args.engine_sha, nodes) for rel in all_shards if rel not in set(problems.get("source shards without a relabelled manifest", []))]
    with multiprocessing.Pool() as pool:
        for i, r in enumerate(pool.imap_unordered(check_shard, jobs, chunksize=16), 1):
            failures.extend([r["rel"], f] for f in r["fail"])
            if "net_sha" in r:
                net_shas.add(r["net_sha"])
            for key in (r["gen"], r["run"], "all"):
                t = totals[key]
                t["shards"] += 1
                t["largest"] = max(t["largest"], r.get("largest", 0))
                for name in SUMMED:
                    t[name] += r.get("totals", {}).get(name, 0)
            if i % 5000 == 0:
                print(f"  {i} of {len(jobs)} shards, {len(failures)} failures", flush=True)
    if len(net_shas) > 1:
        failures.append(["all", f"manifests name {len(net_shas)} nets: {sorted(net_shas)}"])
    for rel, fail in failures[:20]:
        print(f"FAIL {rel}: {fail}")
    print(f"{len(failures)} failures; {'FAIL' if failures else 'PASS'}")

    print("==> 4. plausibility, a guide against the noise floor in docs/datasets.md")
    rows = {key: report(key, totals[key]) for key in sorted(totals)}
    implausible = [
        key for key, row in rows.items()
        if key.startswith("gen=") and (row["sign_agreement_pct"] < MIN_SIGN_AGREEMENT or row["correlation"] < MIN_CORRELATION)
    ]
    print(f"{'FAIL, like a wrong point of view: ' + str(implausible) if implausible else 'PASS'}")

    det = None
    if args.skip_determinism:
        print("==> 5. determinism: SKIPPED, so this is not a pass")
    else:
        print(f"==> 5. determinism, {args.per_run} shards a run, seed {args.seed}")
        det = determinism(args, all_shards, nodes)
        print(f"{det['identical']} of {len(det['sample'])} positions files byte-identical across {det['runs']} runs; "
              f"{det['manifests_equal']} manifests equal apart from seconds; differ: {det['differ']}")
        print(f"negative control at half the nodes: {det['negative_control_differ']} of {len(det['negative_control'])} differ")
        print("PASS" if det["passed"] else "FAIL")

    passed = not problems and not failures and not implausible and (det is None or det["passed"])
    verdict = "FAIL" if not passed else "INCOMPLETE" if det is None else "PASS"
    with open(args.json, "w") as f:
        json.dump({"verdict": verdict, "label_id": args.label_id, "engine_sha": args.engine_sha, "net_sha256": sorted(net_shas),
                   "inventory_problems": problems, "failures": failures, "implausible": implausible, "rows": rows,
                   "determinism": det}, f, indent=1)
    print("==> not checked here: the relabel role's write policy, and the source's shard counts against docs/datasets.md")
    print(f"==> {verdict}")
    sys.exit(0 if verdict == "PASS" else 2 if verdict == "INCOMPLETE" else 1)


if __name__ == "__main__":
    main()
