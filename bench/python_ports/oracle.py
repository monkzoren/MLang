import queue
import sys
import threading

MAPPERS = 3
lines_q = queue.Queue()
words_q = queue.Queue()
requests = queue.Queue()
replies = queue.Queue()
answers = queue.Queue()

doc_lines = 0


def mapper():
    while True:
        line = lines_q.get()
        if line is None:
            words_q.put(None)
            return
        for word in line.lower().split():
            words_q.put(word)


def counter():
    counts = {}
    done = 0
    while done < MAPPERS:
        w = words_q.get()
        if w is None:
            done += 1
        else:
            counts[w] = counts.get(w, 0) + 1
    while True:
        req = requests.get()
        if req[0] == "count":
            word = req[1]
            replies.put(f"{word}: {counts.get(word, 0)}")
        elif req[0] == "top":
            ranked = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
            replies.put("\n".join(f"{w} × {n}" for w, n in ranked[:req[1]]))
        elif req[0] == "stats":
            total = sum(counts.values())
            replies.put(f"lines {doc_lines} · words {total} "
                        f"· distinct {len(counts)}")
        else:
            answers.put("goodbye, operator")
            answers.put(None)
            return


def printer():
    while True:
        a = answers.get()
        if a is None:
            return
        print(a)


def handle(line):
    try:
        parts = line.split()
        cmd = parts[0]
        if cmd == "count":
            requests.put(("count", parts[1]))
        elif cmd == "top":
            requests.put(("top", int(parts[1])))
        elif cmd == "stats":
            requests.put(("stats",))
        else:
            raise ValueError("unknown command")
        answers.put(replies.get())
    except Exception:
        answers.put("✗ " + line)


threads = [threading.Thread(target=mapper) for _ in range(MAPPERS)]
threads += [threading.Thread(target=counter), threading.Thread(target=printer)]
for t in threads:
    t.start()

stdin = (line.rstrip("\n") for line in sys.stdin)
for line in stdin:
    if line == ".":
        break
    lines_q.put(line)
    doc_lines += 1
for _ in range(MAPPERS):
    lines_q.put(None)

for line in stdin:
    if line == "q":
        break
    worker = threading.Thread(target=handle, args=(line,))
    worker.start()
    worker.join()
requests.put(("halt",))
for t in threads:
    t.join()
