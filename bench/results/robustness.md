| One-token mutation becomes | MLang | Python |
|---|---|---|
| caught before running (load error) | 13.2% | 72.5% |
| caught at runtime, precise report | 50.4% | 19.2% |
| deadlock — proven and reported | 2.8% | 0.0% |
| **silent wrong output** | 28.6% | 6.3% |
| hang (killed at timeout) | 0.7% | 1.4% |
| no behavior change (equivalent mutant) | 4.4% | 0.6% |

1124 MLang mutants over 119 programs; 797 Python mutants over 28 ports. Same four operator classes per arm (swap / drop / transpose / rename), one edit per mutant, strings and comments masked. 7 of 11 Python hangs printed a thread traceback first — the process still never exited.
