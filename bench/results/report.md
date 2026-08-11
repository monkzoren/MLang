| Self-repair — claude-haiku-4-5-20251001, ≤3 rounds | MLang | Python |
|---|---|---|
| seeded one-edit bugs | 80 | 80 |
| **healed (byte-exact output)** | **99%** | **100%** |
| healed in one round | 91% | 100% |
| median rounds to green | 1 | 1 |

| healed, by what the bug turned into | MLang | Python |
|---|---|---|
| caught before running | 5/5 | 54/54 |
| runtime fault, precise report | 37/37 | 23/23 |
| proven deadlock | 3/3 | — |
| silent wrong output | 34/35 | 2/2 |
| hang | — | 1/1 |

| One-token mutation becomes | MLang | Python |
|---|---|---|
| caught before running (load error) | 11.3% | 72.5% |
| caught at runtime, precise report | 49.8% | 19.2% |
| deadlock — proven and reported | 3.2% | 0.0% |
| **silent wrong output** | 31.0% | 6.3% |
| hang (killed at timeout) | 1.0% | 1.4% |
| no behavior change (equivalent mutant) | 3.6% | 0.6% |

893 MLang mutants over 94 programs; 797 Python mutants over 28 ports. Same four operator classes per arm (swap / drop / transpose / rename), one edit per mutant, strings and comments masked. 7 of 11 Python hangs printed a thread traceback first — the process still never exited.
