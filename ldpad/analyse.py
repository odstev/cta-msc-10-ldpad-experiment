import statistics

def ci_95(xs):
    n = len(xs)
    if n < 2:
        return (xs[0], xs[0])
    m = statistics.mean(xs)
    s = statistics.stdev(xs)
    # two-sided 95% t-value for df = n-1 (approx for n=30 → 2.045)
    t = 2.045
    half = t * s / (n ** 0.5)
    return (m - half, m + half)

def summarise(name):
    path = f"results/{name}.txt"
    try:
        with open(path) as f:
            xs = [float(l.strip()) for l in f if l.strip()]
    except FileNotFoundError:
        print(f"{name}: file not found")
        return
    mean = statistics.mean(xs)
    lo, hi = ci_95(xs)
    print(f"{name:12s}  n={len(xs):3d}  mean={mean:8.4f}  95% CI=[{lo:.4f}, {hi:.4f}]  "
          f"min={min(xs):.4f}  max={max(xs):.4f}")

print("=== LDPAD — 30-run summary with 95% CI ===")
summarise("detections")
summarise("peak_risk")
summarise("p50")
summarise("p95")
summarise("p99")
