"""Record the Python ports' goldens (bench/python_ports/expected.json).

Each port is a natural Python translation of one conformance case; its
recorded output is the healing target for the Python arm, exactly as
conformance/expected.json is for the MLang arm. Re-run after editing a
port and review the diff.
"""

import json
import os
import sys

import common


def main():
    check = "--check" in sys.argv[1:]
    manifest = json.load(open(os.path.join(common.BENCH, "python_ports",
                                           "manifest.json")))
    expected = {}
    for name, meta in sorted(manifest.items()):
        src = open(os.path.join(common.BENCH, "python_ports",
                                meta["file"]), encoding="utf-8").read()
        r = common.run_python(src, meta["stdin"])
        assert not r["hang"], f"{name}: port hangs"
        assert r["exit"] == 0, f"{name}: port exits {r['exit']}: {r['stderr']}"
        assert r["stderr"] == "", f"{name}: port writes stderr"
        expected[name] = {"exit": 0, "stdout": r["stdout"], "stderr": ""}
        print(f"✓ {name}: {len(r['stdout'])} bytes")
    out = os.path.join(common.BENCH, "python_ports", "expected.json")
    if check:
        recorded = json.load(open(out))
        assert expected == recorded, \
            "ports diverge from recorded goldens — rerun record_ports.py " \
            "and review the diff"
        print(f"{len(expected)} port goldens verified")
    else:
        with open(out, "w") as f:
            json.dump(expected, f, indent=1, sort_keys=True)
            f.write("\n")
        print(f"recorded {len(expected)} port goldens")


if __name__ == "__main__":
    main()
