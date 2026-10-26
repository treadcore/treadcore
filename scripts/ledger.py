#!/usr/bin/env python3
"""The census ledger (LR 92): every census a machine has read, one row
each, in one table, so a night's search, the weight ledger and Paul
read one record instead of build/ folders and NOTES files.

    scripts/ledger.py [build/census] > build/ledger.tsv
    scripts/ledger.py --merge a.tsv b.tsv ... > ledger.tsv

A row: the host, the census's stamp, the commit it was built from (a +
for uncommitted source; empty before 26 October), the frozen binary's
hash, seeds, first seed, seconds, map, respawns, the trial's weights and
the baseline's (`k=v;...`, `caps=...` as written), the every-map
difference a seat, its standard error, z, and each map's difference and
error (`map:d:se|...`). A census with no pooled.txt (running, or
stopped) is left out. --merge joins ledgers from several machines,
one row a host and stamp; a census belongs to its machine, so rows
are joined, never pooled (CENSUS.md section 5).
"""
import csv
import glob
import hashlib
import os
import re
import socket
import sys

COLS = ['host', 'stamp', 'commit', 'binary', 'seeds', 'seed0', 'secs', 'map', 'respawns',
        'trial', 'base', 'd', 'se', 'z', 'maps']
LINE = re.compile(r'^(\S+(?: map)?)\s+([-+][\d.]+) \+/- ([\d.]+) over\s+(\d+) seeds')


def weights(path):
    """A weights file as `k=v;...`, comments and blanks out."""
    try:
        rows = [l.split('#', 1)[0].strip() for l in open(path)]
    except OSError:
        return ''
    return ';'.join(re.sub(r'\s*=\s*', '=', r) for r in rows if r)


def env(path):
    out = {}
    try:
        for l in open(path):
            if '=' in l:
                k, v = l.rstrip('\n').split('=', 1)
                out[k] = v.strip("'\"")
    except OSError:
        pass
    return out


def binary(path):
    try:
        h = hashlib.sha256()
        with open(path, 'rb') as f:
            for b in iter(lambda: f.read(1 << 20), b''):
                h.update(b)
        return h.hexdigest()[:12]
    except OSError:
        return ''


def row(d, host):
    try:
        lines = open(os.path.join(d, 'pooled.txt')).read().splitlines()
    except OSError:
        return None
    every, maps = None, []
    for l in lines:
        m = LINE.match(l)
        if not m:
            continue
        name, dd, se = m.group(1), float(m.group(2)), float(m.group(3))
        if name == 'every map':
            every = (dd, se, int(m.group(4)))
        else:
            maps.append(f'{name}:{dd:+.2f}:{se:.2f}')
    if every is None:
        return None
    e = env(os.path.join(d, 'run.env'))
    dd, se, n = every
    return {
        'host': e.get('HOST', host), 'stamp': os.path.basename(d.rstrip('/')),
        'commit': e.get('COMMIT', ''), 'binary': binary(os.path.join(d, 'census-bin')),
        'seeds': e.get('SEEDS', str(n)), 'seed0': e.get('SEED0', ''), 'secs': e.get('SECS', ''),
        'map': e.get('MAP', ''), 'respawns': e.get('RESPAWNS', ''),
        # A net in the trial seat (LR 96) is the trial; its weights file is
        # the defaults' and says nothing.
        'trial': f"net={e['NET']}" if e.get('NET') else weights(os.path.join(d, 'trial.weights')),
        'base': weights(os.path.join(d, 'base.weights')),
        'd': f'{dd:+.2f}', 'se': f'{se:.2f}', 'z': f'{dd / se:+.2f}' if se > 0 else '0.00',
        'maps': '|'.join(maps),
    }


def write(rows):
    w = csv.DictWriter(sys.stdout, COLS, delimiter='\t', lineterminator='\n')
    w.writeheader()
    for r in sorted(rows, key=lambda r: (r['stamp'], r['host'])):
        w.writerow(r)


def main(argv):
    if argv[:1] == ['--merge']:
        rows = {}
        for f in argv[1:]:
            for r in csv.DictReader(open(f), delimiter='\t'):
                rows[(r['host'], r['stamp'])] = r
        write(rows.values())
        return
    root = argv[0] if argv else 'build/census'
    host = socket.gethostname().split('.')[0]
    write(r for r in (row(d, host) for d in glob.glob(os.path.join(root, '*/'))) if r)


if __name__ == '__main__':
    main(sys.argv[1:])
