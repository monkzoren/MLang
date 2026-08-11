import threading
import queue

alpha = queue.Queue()
beta = queue.Queue()

def produce():
    for i in range(9):
        v = i + 1
        alpha.put(v * v)
    alpha.put(None)

def pump():
    while True:
        v = alpha.get()
        if v is None:
            beta.put(None)
            return
        beta.put(v * 2)

def drain():
    while True:
        v = beta.get()
        if v is None:
            return
        print(v)

threads = [threading.Thread(target=f) for f in (produce, pump, drain)]
for t in threads:
    t.start()
for t in threads:
    t.join()
