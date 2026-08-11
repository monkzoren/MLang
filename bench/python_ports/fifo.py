import threading
import queue

c = queue.Queue()

def send():
    c.put(1)
    c.put(2)
    c.put(3)

def recv():
    print(c.get())
    print(c.get())
    print(c.get())

threads = [threading.Thread(target=send), threading.Thread(target=recv)]
for t in threads:
    t.start()
for t in threads:
    t.join()
