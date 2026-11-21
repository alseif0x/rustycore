"""Check the exact source spelling of changed local-binding fingerprints.

The exhaustive inventory owns changes to row identities, multiplicity, or
unrecognised operations. A PR result covers the snapshot delta against its
accepted base. The independent post-merge audit recomputes the whole census.
"""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
import subprocess
import sys


SNAPSHOT = "tools/architecture/persistence-access-snapshot.json"
MANIFEST = Path(__file__).resolve().parents[1] / "full-validation/Cargo.toml"


def blob(revision, path):
    return subprocess.check_output(["git", "show", f"{revision}:{path}"])


def unique_keys(pairs):
    result = dict(pairs)
    if len(result) != len(pairs):
        raise ValueError("duplicate key in the persistence snapshot")
    return result


def rows(revision):
    data = json.loads(blob(revision, SNAPSHOT), object_pairs_hook=unique_keys)
    if set(data) != {"schema_version", "accesses"} or data["schema_version"] != 3:
        raise ValueError("unsupported inventory schema")
    return Counter(json.dumps(row, sort_keys=True) for row in data["accesses"])


def group(delta):
    groups = defaultdict(list)
    for raw in delta.elements():
        row = json.loads(raw)
        fingerprint = row.pop("fingerprint")
        groups[json.dumps(row, sort_keys=True)].append(fingerprint)
    return groups


def check(base):
    base = subprocess.check_output(["git", "rev-parse", "--verify", "--end-of-options", base + "^{commit}"], text=True).strip()
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    subprocess.run(["git", "merge-base", "--is-ancestor", base, head], check=True)
    before, after = rows(base), rows(head)
    removed, added = group(before - after), group(after - before)
    if not removed and not added:
        print("Snapshot rows identical to the accepted base.")
        return
    if removed.keys() != added.keys():
        raise ValueError("changed row identity or count requires exhaustive verification")
    cache = {}
    affected_slots = defaultdict(set)
    for identity, old_values in removed.items():
        row = json.loads(identity)
        path, owner, name = row["source"], row["enclosing"], row["symbol"]
        if (len(old_values) != 1 or len(added[identity]) != 1 or row["operation"] != "value_alias"
                or not owner.startswith("fn ") or not owner[3:].isidentifier() or not name.isidentifier()
                or not path.startswith("crates/") or ".." in Path(path).parts or not path.endswith(".rs")):
            raise ValueError("unsupported snapshot delta requires exhaustive verification")
        slot = path, owner, name
        for revision in (base, head):
            key = revision, path, owner, name
            if key not in cache:
                result = subprocess.run([
                    "cargo", "run", "--quiet", "--locked", "--manifest-path", str(MANIFEST),
                    "--", owner[3:], name,
                ], input=blob(revision, path), stdout=subprocess.PIPE)
                if result.returncode == 2:
                    raise ValueError("ambiguous or unsupported binding requires exhaustive verification")
                result.check_returncode()
                cache[key] = result.stdout.decode().removesuffix("\0").split("\0")
        previous, current = cache[(base, *slot)], cache[(head, *slot)]
        if len(previous) != len(current) or previous.count(old_values[0]) != 1:
            raise ValueError(f"binding multiplicity changed or ambiguous: {slot}")
        index = previous.index(old_values[0])
        if current[index] != added[identity][0]:
            raise RuntimeError(f"fingerprint differs from {head[:10]} {path} {owner}::{name}")
        affected_slots[slot].add(index)
    for slot, updated in affected_slots.items():
        changed_sites = {i for i, (old, new) in enumerate(zip(cache[(base, *slot)], cache[(head, *slot)])) if old != new}
        if changed_sites != updated:
            raise ValueError(f"additional initializer changes require exhaustive verification: {slot}")
    # Reconstruct the complete candidate snapshot from the base and real
    # initializers. Every target of a binding must follow the same source edit.
    rebuilt = Counter()
    for raw, count in before.items():
        row = json.loads(raw)
        key = row["source"], row["enclosing"], row["symbol"]
        if row["operation"] == "value_alias" and key in affected_slots:
            old = cache[(base, *key)]
            if old.count(row["fingerprint"]) != 1:
                raise ValueError(f"cannot associate the base row with one initializer: {key}")
            row["fingerprint"] = cache[(head, *key)][old.index(row["fingerprint"])]
            raw = json.dumps(row, sort_keys=True)
        rebuilt[raw] += count
    if rebuilt != after:
        raise RuntimeError("declared snapshot differs from the source-derived delta")
    print(f"Verified {sum((before - after).values())} changed fingerprints at both revisions; "
          f"{sum(after.values())} row identities and counts retained.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    try:
        check(parser.parse_args().base)
    except (KeyError, ValueError) as error:
        print(error, file=sys.stderr)
        sys.exit(2)
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        sys.exit(1)
