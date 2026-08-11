import queue

def pour(items, ch):
    for v in items:
        ch.put(v)
    ch.put(None)

def drain(ch):
    out = []
    while True:
        v = ch.get()
        if v is None:
            return out
        out.append(v)

a = queue.Queue()
pour([1, 2, 3], a)
print(drain(a))
b = queue.Queue()
pour([], b)
print(drain(b))
c = queue.Queue()
pour("xy", c)
print(drain(c))
