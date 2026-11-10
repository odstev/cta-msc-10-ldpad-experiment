#!/usr/bin/env python3
"""Check the package's retrospective SHA-256 inventory; not a preregistration."""
from pathlib import Path
import hashlib
ROOT=Path(__file__).resolve().parents[1]
lines=(ROOT/'protocol'/'input_sha256.txt').read_text().splitlines()
checked=0
for line in lines:
 if not line or line.startswith('#'):continue
 digest,rel=line.split('  ',1)
 path=ROOT/rel
 if not path.is_file():raise SystemExit('Missing evidence input: '+rel)
 observed=hashlib.sha256(path.read_bytes()).hexdigest()
 if observed!=digest:raise SystemExit('Evidence input hash mismatch: '+rel)
 checked+=1
seeds=(ROOT/'protocol'/'seeds.txt').read_text().splitlines()
if seeds!=(ROOT/'seeds.txt').read_text().splitlines() or seeds!=[str(i) for i in range(30)]:
 raise SystemExit('Seed lists do not match the recorded 0--29 detector seeds')
print('Validated',checked,'retrospectively inventoried package inputs and the seed list.')
