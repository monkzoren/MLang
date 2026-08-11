import threading
import queue

a = queue.Queue()
b = queue.Queue()
c = queue.Queue()

def produce():
    for i in range(5):
        a.put(i)
    a.put(None)

def add_one():
    while True:
        v = a.get()
        if v is None:
            b.put(None)
            return
        b.put(v + 1)

def times_ten():
    while True:
        v = b.get()
        if v is None:
            c.put(None)
            return
        c.put(v * 10)

def drain():
    out = []
    while True:
        v = c.get()
        if v is None:
            print(out)
            return
        out.append(v)

threads = [threading.Thread(target=f)
           for f in (produce, add_one, times_ten, drain)]
for t in threads:
    t.start()
for t in threads:
    t.join()
