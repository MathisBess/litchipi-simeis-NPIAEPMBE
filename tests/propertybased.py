import sys
import time
import random
import math


def create_property_based_test(f, regressions=[], time_test=10):
    tstart = time.time()
    i = 0
    while (time.time() - tstart) < time_test:
        if i < len(regressions):
            seed = regressions[i]
        else:
            seed = random.randrange(0, 2**64)
        random.seed(seed)
        try:
            f()
            print("Test", f.__name__, i, "OK")
        except AssertionError as err:
            print("Test", f.__name__, "failed with seed", seed)
            print(err)
            sys.exit(1)
        i += 1


### Example


def get_dist(a, b):
    return math.sqrt(((a[0] - b[0]) ** 2) + ((a[1] - b[1]) ** 2) + ((a[2] - b[2]) ** 2))


def addition():
    x = random.randrange(0, 10000)
    y = random.randrange(0, 10000)
    z = random.randrange(0, 10000)

    assert x + y == y + x, "L'addition doit être commutative"
    assert (x + y) + z == x + (y + z), "L'addition doit être associative"
    assert x + 0 == x, "0 doit être l'élément neutre"


def distance():
    x1 = random.randrange(-100, 100)
    y1 = random.randrange(-100, 100)
    z1 = random.randrange(-100, 100)
    a = (x1, y1, z1)

    x2 = random.randrange(-100, 100)
    y2 = random.randrange(-100, 100)
    z2 = random.randrange(-100, 100)
    b = (x2, y2, z2)

    dist_ab = get_dist(a, b)
    dist_ba = get_dist(b, a)

    assert dist_ab >= 0, "La distance doit toujours être positive ou nulle"
    assert dist_ab == dist_ba, "La distance doit être symétrique (A->B == B->A)"
    if a == b:
        assert dist_ab == 0, "La distance entre un point et lui-même doit être 0"


if __name__ == "__main__":
    time_test_param = int(sys.argv[1]) if len(sys.argv) > 1 else 3
    create_property_based_test(addition, time_test=time_test_param)
    create_property_based_test(
        distance, regressions=[4480881574280375424], time_test=time_test_param
    )
