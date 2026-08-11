| Self-repair — claude-haiku-4-5-20251001, ≤3 rounds | MLang | Python |
|---|---|---|
| seeded one-edit bugs | 80 | 80 |
| **healed (byte-exact output)** | **98%** | **100%** |
| healed in one round | 90% | 100% |
| median rounds to green | 1 | 1 |

| healed, by what the bug turned into | MLang | Python |
|---|---|---|
| caught before running | 8/8 | 54/54 |
| runtime fault, precise report | 36/36 | 23/23 |
| proven deadlock | 3/3 | — |
| silent wrong output | 31/33 | 2/2 |
| hang | — | 1/1 |

| One-token mutation becomes | MLang | Python |
|---|---|---|
| caught before running (load error) | 13.2% | 72.5% |
| caught at runtime, precise report | 50.4% | 19.2% |
| deadlock — proven and reported | 2.8% | 0.0% |
| **silent wrong output** | 28.6% | 6.3% |
| hang (killed at timeout) | 0.7% | 1.4% |
| no behavior change (equivalent mutant) | 4.4% | 0.6% |

1124 MLang mutants over 119 programs; 797 Python mutants over 28 ports. Same four operator classes per arm (swap / drop / transpose / rename), one edit per mutant, strings and comments masked. 7 of 11 Python hangs printed a thread traceback first — the process still never exited.
