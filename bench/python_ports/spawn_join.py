import threading
import queue

r = queue.Queue()
t = threading.Thread(target=lambda: r.put(42))
t.start()
t.join()
print(r.get())
