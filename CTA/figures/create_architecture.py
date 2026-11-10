#!/usr/bin/env python3
"""Draw the proposed LDPAD architecture at readable single-column size."""
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch

out = Path(__file__).resolve().parent
fig = plt.figure(figsize=(3.35, 4.4), facecolor='white')
ax = fig.add_axes([0, 0, 1, 1])
ax.set(xlim=(0, 1), ylim=(0, 1))
ax.axis('off')

labels = [
    'Healthcare workload and telemetry',
    'Layers 1–3: Access, API and\nlocal orchestration',
    'Layer 4: Static posture auditing',
    'Layer 5: Configuration drift and\nthreat intelligence',
    'Layer 6: Runtime anomaly detection',
    'Risk fusion and policy mode\nNormal  |  Moderate  |  Strict',
    'Layer 7: Encryption',
    'Layer 8: Feedback and event log',
]
left, width, height = 0.14, 0.83, 0.084
ys = [0.89 - i*0.12 for i in range(len(labels))]
for y, label in zip(ys, labels):
    ax.add_patch(FancyBboxPatch((left, y), width, height,
        boxstyle='round,pad=0.01,rounding_size=0.015',
        facecolor='white', edgecolor='black', linewidth=0.8))
    ax.text(left + width/2, y + height/2, label,
            ha='center', va='center', fontsize=7.4,
            color='black', linespacing=1.15, fontweight='normal')

center = left + width/2
for upper, lower in zip(ys, ys[1:]):
    ax.annotate('', xy=(center, lower + height + 0.008),
        xytext=(center, upper - 0.008),
        arrowprops=dict(arrowstyle='-|>', lw=0.8, color='black',
                        mutation_scale=8))

# The dashed path shows that feedback updates the dynamic-posture layer.
last_y = ys[-1] + height/2
target_y = ys[3] + height/2
ax.plot([left - 0.01, 0.055, 0.055], [last_y, last_y, target_y],
        color='black', lw=0.75, linestyle=(0, (3, 2)))
ax.annotate('', xy=(left - 0.01, target_y), xytext=(0.055, target_y),
    arrowprops=dict(arrowstyle='-|>', lw=0.75, linestyle=(0, (3, 2)),
                    color='black', mutation_scale=8))

fig.savefig(out/'architecture_single_column.pdf', facecolor='white')
fig.savefig(out/'architecture_single_column.png', dpi=220, facecolor='white')
plt.close(fig)
