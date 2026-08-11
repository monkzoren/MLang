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

| Self-repair — claude-haiku-4-5-20251001, ≤3 rounds | MLang | Python |
|---|---|---|
| seeded one-edit bugs | 40 | 40 |
| **healed (byte-exact output)** | **65%** | **100%** |
| healed in one round | 45% | 100% |
| median rounds to green | 1 | 1 |

| healed, by what the bug turned into | MLang | Python |
|---|---|---|
| caught before running | 1/2 | 35/35 |
| runtime fault, precise report | 15/22 | 2/2 |
| proven deadlock | 5/6 | — |
| silent wrong output | 5/10 | 1/1 |
| hang | — | 2/2 |

| One-token mutation becomes | MLang | Python |
|---|---|---|
| caught before running (load error) | 13.2% | 73.1% |
| caught at runtime, precise report | 50.4% | 18.7% |
| deadlock — proven and reported | 2.9% | 0.0% |
| **silent wrong output** | 28.5% | 6.0% |
| hang (killed at timeout) | 0.7% | 1.6% |
| no behavior change (equivalent mutant) | 4.3% | 0.6% |

1134 MLang mutants over 120 programs; 828 Python mutants over 29 ports. Same four operator classes per arm (swap / drop / transpose / rename), one edit per mutant, strings and comments masked. 9 of 13 Python hangs printed a thread traceback first — the process still never exited.
