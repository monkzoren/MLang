| One-token mutation becomes | MLang | Python |
|---|---|---|
| caught before running (load error) | 10.7% | 77.8% |
| caught at runtime, precise report | 49.4% | 6.0% |
| deadlock — proven and reported | 14.4% | 0.0% |
| **silent wrong output** | 16.0% | 4.7% |
| hang (killed at timeout) | 0.0% | 10.7% |
| no behavior change (equivalent mutant) | 9.5% | 0.9% |

243 MLang mutants over 1 programs; 234 Python mutants over 1 ports. Same four operator classes per arm (swap / drop / transpose / rename), one edit per mutant, strings and comments masked. 23 of 25 Python hangs printed a thread traceback first — the process still never exited.
