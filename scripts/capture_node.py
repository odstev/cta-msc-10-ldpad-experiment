#!/usr/bin/env python3
"""Collect whole-node CPU/memory readings; does not measure the LDPAD process alone."""
import argparse,datetime,subprocess,time
from pathlib import Path

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output',required=True);p.add_argument('--node',required=True)
    p.add_argument('--samples',type=int,default=4);p.add_argument('--interval',type=float,default=15)
    p.add_argument('--kubectl',action='store_true');a=p.parse_args()
    if a.samples<1 or a.interval<=0:p.error('samples and interval must be positive')
    base=['kubectl'] if a.kubectl else ['sudo','k3s','kubectl']
    # Authenticate before starting timestamps, so a prompt does not distort sampling.
    if not a.kubectl:subprocess.run(['sudo','-v'],check=True)
    with Path(a.output).open('x') as f:
        for i in range(a.samples):
            stamp=datetime.datetime.now().astimezone().isoformat(timespec='seconds')
            row=subprocess.check_output(base+['top','node',a.node,'--no-headers'],text=True).strip()
            if len(row.splitlines())!=1 or len(row.split())!=5:raise ValueError('Unexpected metrics output: '+row)
            text=stamp+'\n'+row+'\n';f.write(text);f.flush();print(text,end='',flush=True)
            if i+1<a.samples:time.sleep(a.interval)
    print('Saved whole-node readings:',a.output)
if __name__=='__main__':main()
