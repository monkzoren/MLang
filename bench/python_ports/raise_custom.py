try:
    raise ValueError([1, 2])
except ValueError as e:
    print(e.args[0])
