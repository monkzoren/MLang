import threading
import queue

alpha = queue.Queue()
beta = queue.Queue()

def produce():
    for i in range(1, 10):
        alpha.put(i * i)
    alpha.put(None)

def pump():
    while (v := alpha.get()) is not None:
        if v == 25:
            raise ValueError("boom")
        beta.put(v * 2)
    beta.put(None)

def drain():
    while (v := beta.get()) is not None:
        print(v)

threads = [threading.Thread(target=f) for f in (produce, pump, drain)]
for t in threads:
    t.start()
for t in threads:
    t.join()
