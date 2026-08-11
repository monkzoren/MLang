| One-token mutation becomes | MLang | Python |
|---|---|---|
| caught before running (load error) | 13.2% | 73.1% |
| caught at runtime, precise report | 50.4% | 18.7% |
| deadlock — proven and reported | 2.9% | 0.0% |
| **silent wrong output** | 28.5% | 6.0% |
| hang (killed at timeout) | 0.7% | 1.6% |
| no behavior change (equivalent mutant) | 4.3% | 0.6% |

1134 MLang mutants over 120 programs; 828 Python mutants over 29 ports. Same four operator classes per arm (swap / drop / transpose / rename), one edit per mutant, strings and comments masked. 9 of 13 Python hangs printed a thread traceback first — the process still never exited.
