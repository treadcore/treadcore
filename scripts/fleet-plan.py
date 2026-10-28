#!/usr/bin/env python3
"""treadcore's planner for orrery's fleet-queue (LAB-REPORT-BRAIN.md 110).

    python3 -I scripts/fleet-plan.py QDIR plan N     up to N new jobs, as JSON
    python3 -I scripts/fleet-plan.py QDIR collect    what the finished jobs found
    python3 -I scripts/fleet-plan.py QDIR judged ID  (on_done of a judge job)
    python3 -I scripts/fleet-plan.py QDIR families   the families' verdicts so far

orrery's keeper (`tools/fleet-queue.py QDIR start`) calls `plan` whenever the
queue runs low, and `collect` when it stops. This file decides what the fleet
does next, from QDIR/plan.json and from the results already in QDIR/results.
Three kinds of work share the one queue:

- weights: the scripted brain's named weights (weights.rs), a census trial
  each. scripts/suggest.py picks them from the queue's own ledger, so the
  first trials screen every weight at half and double and later ones
  follow each weight's fitted curve. A trial whose census reads past two
  errors above the defaults, with no map past two errors below, is read
  twice more on held-out seeds. Nothing is promoted: candidates wait for
  `make families` and Paul.
- net: the learned tank. A round trains the best net so far (START=) on a
  recording, on the GPU machine, then judges the candidate against the best
  in one census machine's census, paired by seed. A candidate above the
  best outside two errors becomes the queue's best; the official net
  (nets/map7.bin) changes only when Paul says.
- families: a candidate's weights over every family in scenarios/families/,
  each held to its floor (LR 112; what `make families` did). The candidates
  are the defaults first, then the queue's own trials that passed both
  held-out reads, then plan.json's converge runs, newest generation first.
  A job runs a batch of them side by side, one a core.

Every host first builds the queue's commit in its own worktree
(~/fleet/q-NAME/wt), so every census of the queue runs the same source.
"""
import base64
import glob
import json
import os
import re
import shutil
import statistics as st
import subprocess
import sys

TC = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def load(p, default=None):
    try:
        with open(p) as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def save(p, v):
    tmp = p + ".tmp"
    with open(tmp, "w") as f:
        json.dump(v, f, indent=1)
    os.replace(tmp, p)


class Plan:
    def __init__(self, qdir):
        self.q = os.path.abspath(qdir)
        self.name = os.path.basename(self.q.rstrip("/"))
        self.cfg = load(os.path.join(self.q, "plan.json"), {})
        self.hosts = load(os.path.join(self.q, "config.json"), {}).get("hosts", [])
        self.pd = os.path.join(self.q, "plan")
        os.makedirs(self.pd, exist_ok=True)
        self.state = load(os.path.join(self.pd, "state.json"), {"weights_n": 0, "gen": 0, "held_n": 0, "round": 0, "held_read": []})
        self.state.setdefault("families_n", 0)
        self.state.setdefault("families_queued", [])
        self.home = "fleet/q-%s" % self.name  # under each host's $HOME

    def keep(self):
        save(os.path.join(self.pd, "state.json"), self.state)

    def ids(self, *subs):
        out = set()
        for sub in subs:
            for f in glob.glob(os.path.join(self.q, sub, "**", "*.json"), recursive=True):
                out.add(re.sub(r"^\d+-", "", os.path.basename(f))[:-5])
        return out

    # ---- preparing the hosts ------------------------------------------------
    def prepare_jobs(self):
        commit = self.cfg["commit"]
        jobs = []
        known = self.ids("queue", "running", "done", "failed", "stopped")
        for h in self.hosts:
            jid = "prep-%s" % h["name"]
            if jid in known:
                continue
            cmd = (
                "set -e; [ -f ~/.cargo/env ] && . ~/.cargo/env; export PATH=$HOME/.cargo/bin:$PATH\n"
                "Q=~/%s; mkdir -p $Q/out\n"
                "cd ~/code/treadcore && git fetch -q origin\n"
                "git worktree remove --force $Q/wt 2>/dev/null || true; git worktree prune\n"
                "git worktree add -q --detach $Q/wt %s\n"
                "cd $Q/wt && export CARGO_TARGET_DIR=$HOME/fleet/target && . scripts/common.sh && census_bin $Q >/dev/null\n"
                "CENSUS_DUMP=$Q/defaults.toml CENSUS_SEEDS=0 $Q/census-bin --ignored brain_census >/dev/null 2>&1\n"
                "sha256sum $Q/census-bin | cut -c1-16\n"
            ) % (self.home, commit)
            job = {"id": jid, "kind": "prepare", "title": "build %s at %s on %s" % ("the census binary", commit[:7], h["name"]),
                   "needs": "host:" + h["name"], "cmd": cmd, "timeout_s": 3600}
            if "gpu" in h.get("caps", []):
                # The GPU machine also builds the trainer at the queue's commit,
                # and keeps the net each round continues.
                job["cmd"] += (
                    "cd $Q/wt/tools/bc && CARGO_TARGET_DIR=$HOME/code/treadcore/tools/bc/target cargo build -q --release\n"
                    "cp $HOME/code/treadcore/tools/bc/target/release/treadcore-bc $Q/treadcore-bc\n"
                )
            if h["name"] == self.cfg["census_host"]:
                job["fetch"] = self.home + "/defaults.toml"
            jobs.append(job)
        return jobs

    def prepared(self, host):
        return "prep-%s" % host in self.ids("done")

    # ---- the weights -------------------------------------------------------------
    def census_cmd(self, trial_text, seeds, seed0, out_id):
        w = self.cfg["weights"]
        b = base64.b64encode(trial_text.encode()).decode()
        return (
            "set -e; [ -f ~/.cargo/env ] && . ~/.cargo/env; export PATH=$HOME/.cargo/bin:$PATH\n"
            "Q=~/%s; mkdir -p $Q/trials; echo %s | base64 -d > $Q/trials/%s.weights\n"
            "cd $Q/wt && export CARGO_TARGET_DIR=$HOME/fleet/target\n"
            "TRIAL=$Q/trials/%s.weights SEEDS=%d SEED0=%d SECS=%d MAP=%s RESPAWNS=3 JOBS=$(nproc 2>/dev/null || sysctl -n hw.ncpu) scripts/census.sh 2>&1 | tail -3\n"
            "d=$(ls -td build/census/*/ | head -1); s=$(basename $d)\n"
            "rm -rf $Q/out/%s; mkdir -p $Q/out/%s/$s; cp $d/*.txt $d/*.env $d/*.weights $d/g-*.log $Q/out/%s/$s/; rm -rf $d\n"
        ) % (self.home, b, out_id, out_id, seeds, seed0, w["secs"], w["map"], out_id, out_id, out_id)

    def census_dirs(self):
        """Every fetched census of the queue: results/ID/fleet/q-NAME/out/ID/STAMP."""
        return sorted(glob.glob(os.path.join(self.q, "results", "*", self.home, "out", "*", "*", "")))

    def ledger(self):
        root = os.path.join(self.pd, "census")
        shutil.rmtree(root, ignore_errors=True)
        os.makedirs(root)
        for d in self.census_dirs():
            # The stamp, and the job: two hosts can finish in the same second.
            jid = d.rstrip("/").split(os.sep)[-2]
            os.symlink(d, os.path.join(root, "%s-%s" % (os.path.basename(d.rstrip("/")), jid)))
        out = os.path.join(self.pd, "ledger.tsv")
        with open(out, "w") as f:
            subprocess.run([sys.executable, "-I", os.path.join(TC, "scripts", "ledger.py"), root], stdout=f, check=True)
        return out

    def defaults(self):
        p = glob.glob(os.path.join(self.q, "results", "prep-" + self.cfg["census_host"], self.home, "defaults.toml"))
        return p[0] if p else None

    @staticmethod
    def passes(d):
        ok, below = False, False
        try:
            lines = open(os.path.join(d, "pooled.txt")).read().splitlines()
        except OSError:
            return None
        for l in lines:
            m = re.match(r"^(\S+(?: map)?)\s+([-+][\d.]+) \+/- ([\d.]+)", l)
            if not m:
                continue
            dd, se = float(m.group(2)), float(m.group(3))
            if m.group(1) == "every map":
                ok = se > 0 and dd > 2 * se
            elif se > 0 and dd < -2 * se:
                below = True
        return ok and not below

    def weight_jobs(self, n):
        w = self.cfg["weights"]
        defaults = self.defaults()
        cpu = [h for h in self.hosts if "cpu" in h.get("caps", ["cpu"]) and self.prepared(h["name"])]
        if not defaults or not cpu or n <= 0:
            return []
        jobs = []
        after = []  # any CPU host may take it; each waits for nothing but its host's build
        # Held-out reads first, for trials that passed and have not been read.
        for d in self.census_dirs():
            jid = d.rstrip("/").split(os.sep)[-2]
            if not jid.startswith("w") or jid in self.state["held_read"] or not self.passes(d):
                continue
            trial = open(os.path.join(d, "trial.weights")).read()
            for k in range(2):
                hid = "h%04d" % self.state["held_n"]
                seed0 = w["held_seed0"] + 200000 * self.state["held_n"]
                self.state["held_n"] += 1
                jobs.append({"id": hid, "kind": "weights", "title": "held-out read %d of %s (%d seeds)" % (k + 1, jid, w["held_seeds"]),
                             "needs": "cpu", "after_host": "prep-{host}", "cmd": self.census_cmd(trial, w["held_seeds"], seed0, hid),
                             "fetch": "%s/out/%s" % (self.home, hid), "timeout_s": 6 * 3600})
            self.state["held_read"].append(jid)
        # Then new trials from the fit.
        want = n - len(jobs)
        if want > 0:
            g = self.state["gen"]
            out = os.path.join(self.pd, "gen%d" % g)
            shutil.rmtree(out, ignore_errors=True)
            r = subprocess.run([sys.executable, "-I", os.path.join(TC, "scripts", "suggest.py"), self.ledger(), defaults, out,
                                "--n", str(want), "--seed", str(g)], capture_output=True, text=True)
            if r.returncode != 0:
                raise SystemExit("suggest.py: " + r.stderr[-400:])
            self.state["gen"] += 1
            for f in sorted(glob.glob(os.path.join(out, "*.weights")))[:want]:
                text = open(f).read()
                jid = "w%04d" % self.state["weights_n"]
                seed0 = w["seed0"] + 10000 * self.state["weights_n"]
                self.state["weights_n"] += 1
                what = os.path.basename(f)[:-8]
                moved = "; ".join(l.split("#")[0].strip() for l in text.splitlines() if "=" in l.split("#")[0])[:80]
                jobs.append({"id": jid, "kind": "weights", "title": "%s: %s" % (what, moved or "the defaults"),
                             "needs": "cpu", "after_host": "prep-{host}", "cmd": self.census_cmd(text, w["seeds"], seed0, jid),
                             "fetch": "%s/out/%s" % (self.home, jid), "timeout_s": 6 * 3600})
        return jobs

    # ---- the net -------------------------------------------------------------------
    def best_net(self):
        p = os.path.join(self.pd, "best.bin")
        if not os.path.exists(p):
            shutil.copy(os.path.join(TC, "nets", "bc2.bin"), p)
            save(os.path.join(self.pd, "best.json"), {"name": "bc2", "round": 0})
        return p

    def net_jobs(self):
        n = self.cfg.get("net")
        if not n:
            return []
        gpu = [h for h in self.hosts if "gpu" in h.get("caps", [])]
        if not gpu or not self.prepared(gpu[0]["name"]) or not self.prepared(self.cfg["census_host"]):
            return []
        r = self.state["round"]
        # One round at a time: the next only when this one is judged (or failed).
        if r > 0 and "judge-%03d" % r not in self.ids("done", "failed", "stopped"):
            return []
        rounds = n["rounds"]
        if r >= len(rounds):
            return []
        spec = rounds[r]
        r += 1
        self.state["round"] = r
        store_q = "~/" + self.home
        tid, jid = "train-%03d" % r, "judge-%03d" % r
        best = self.best_net()
        data = (
            "D=%s/rounds/%03d; rm -rf $D; mkdir -p $D/data\n"
            "for f in ~/fleet/%s/shards/*.bin; do s=$(basename $f .bin); [ $s -ge %d ] && [ $s -lt %d ] && ln -s $f $D/data/; done; true\n"
            "ln -s ~/bc/rec60.bin $D/data/9000000.bin\n"
            "for s in 9001140 9001164 9001188 9001212 9001236; do ln -s ~/fleet/rec1200/shards/$s.bin $D/data/; done\n"
        ) % (store_q, r, spec["run"], spec["lo"], spec["lo"] + spec["n"])
        train = {
            "id": tid, "kind": "net", "title": "round %d: train the best on %s seeds %d to %d" % (r, spec["run"], spec["lo"], spec["lo"] + spec["n"] - 1),
            "needs": "gpu", "after": ["prep-" + gpu[0]["name"]],
            "send": [{"src": best, "dst": "%s/rounds/%03d.start.bin" % (store_q, r)}],
            "cmd": data + "cd $D && HELD=9000054-9000059,9001140-9001259 START=%s/rounds/%03d.start.bin BC_DEVICE=cuda %s/treadcore-bc $D/data $D/cand.bin %s > $D/train.log 2>&1\n"
                   "grep -v 'DEINIT\\|^thread\\|^note\\|^$' $D/train.log | tail -14\n" % (store_q, r, store_q, n["train_args"]),
            "fetch": "%s/rounds/%03d" % (self.home, r), "timeout_s": 4 * 3600,
        }
        c = n["census"]
        census_one = (
            "cd $Q/wt && export CARGO_TARGET_DIR=$HOME/fleet/target\n"
            "SEEDS=%d SEED0=%d SECS=%d MAP=%s RESPAWNS=3 JOBS=$(nproc) CENSUS_NET=$Q/judge/%03d/%%s.bin scripts/census.sh 2>&1 | tail -2\n"
            "d=$(ls -td build/census/*/ | head -1); mkdir -p $Q/judge/%03d/%%s; cp $d/*.txt $d/*.env $d/g-*.log $Q/judge/%03d/%%s/; rm -rf $d\n"
        ) % (c["seeds"], c["seed0"], c["secs"], c["map"], r, r, r)
        judge = {
            "id": jid, "kind": "net", "title": "round %d: judge the candidate against the best, a census each" % r,
            "needs": "host:" + self.cfg["census_host"], "after": [tid, "prep-" + self.cfg["census_host"]],
            "send": [{"src": os.path.join(self.q, "results", tid, self.home, "rounds", "%03d" % r, "cand.bin"), "dst": "~/%s/judge/%03d/cand.bin" % (self.home, r)},
                     {"src": best, "dst": "~/%s/judge/%03d/best.bin" % (self.home, r)}],
            "cmd": "set -e; [ -f ~/.cargo/env ] && . ~/.cargo/env; export PATH=$HOME/.cargo/bin:$PATH; Q=~/%s\n" % self.home
                   + (census_one % ("cand", "cand", "cand")) + (census_one % ("best", "best", "best")),
            "fetch": "%s/judge/%03d" % (self.home, r),
            "on_done": "python3 -I %s %s judged %s" % (os.path.join(TC, "scripts", "fleet-plan.py"), self.q, jid),
            "timeout_s": 6 * 3600,
        }
        return [train, judge]

    def judged(self, jid):
        r = int(jid.split("-")[1])
        base = os.path.join(self.q, "results", jid, self.home, "judge", "%03d" % r)

        def seeds(who):
            out = {}
            for f in glob.glob(os.path.join(base, who, "g-*.log")):
                s0 = int(re.search(r"g-(\d+)", f).group(1))
                for l in open(f):
                    l = re.sub(r"\x1b\[[0-9;]*m", "", l)
                    if l.startswith("brain_census: difference"):
                        for i, v in enumerate(l.split("a seed:")[1].split()):
                            out[s0 + i] = float(v)
            return out

        a, b = seeds("cand"), seeds("best")
        k = sorted(set(a) & set(b))
        if len(k) < 2:
            print("judge %s: too few paired seeds (%d)" % (jid, len(k)))
            return
        d = [a[s] - b[s] for s in k]
        m, se = st.mean(d), st.stdev(d) / len(d) ** 0.5
        verdict = {"round": r, "seeds": len(k), "cand": st.mean(a[s] for s in k), "best": st.mean(b[s] for s in k),
                   "diff": m, "se": se, "better": m > 2 * se}
        tlog = os.path.join(self.q, "results", "train-%03d" % r, self.home, "rounds", "%03d" % r, "train.log")
        agree = {}
        try:
            text = open(tlog).read()
            part = text.split("seeds 9000054-9000059")[1] if "seeds 9000054-9000059" in text else ""
            for h in ("drive", "gun", "mortar", "drone", "lead"):
                mm = re.search(r"^\s+%s\s+([\d.]+)%%" % h, part, re.M)
                if mm:
                    agree[h] = float(mm.group(1))
        except OSError:
            pass
        verdict["held_out"] = agree
        save(os.path.join(self.pd, "round-%03d.json" % r), verdict)
        if verdict["better"]:
            cand = os.path.join(self.q, "results", "train-%03d" % r, self.home, "rounds", "%03d" % r, "cand.bin")
            shutil.copy(cand, os.path.join(self.pd, "best.bin"))
            save(os.path.join(self.pd, "best.json"), {"name": "round %d" % r, "round": r, "census_vs_previous": [m, se], "held_out": agree})
        print("judge %s: candidate less best %+.2f +/- %.2f over %d seeds: %s" % (jid, m, se, len(k), "the new best" if verdict["better"] else "the best stays"))

    # ---- the families ---------------------------------------------------------------
    @staticmethod
    def weights_set(text):
        """A .weights file as SCENARIO_SET: name=value ... (comments dropped)."""
        out = []
        for l in text.splitlines():
            l = l.split("#")[0].strip()
            if "=" in l:
                k, v = (x.strip() for x in l.split("=", 1))
                out.append("%s=%s" % (k, v))
        return " ".join(out)

    def held_pairs(self):
        """Each trial's held-out reads: {trial: [census dir, ...]}, from the jobs' titles."""
        title = {}
        for f in glob.glob(os.path.join(self.q, "done", "**", "*.json"), recursive=True):
            j = load(f, {})
            m = re.match(r"held-out read \d of (w\d+)", j.get("title", ""))
            if m:
                title[j["id"]] = m.group(1)
        out = {}
        for d in self.census_dirs():
            hid = d.rstrip("/").split(os.sep)[-2]
            if hid in title:
                out.setdefault(title[hid], []).append(d)
        return out

    def family_candidates(self):
        """[(name, SCENARIO_SET)] in the order they are read: the defaults, the
        queue's trials that passed both held-out reads, then each converge run's
        CANDIDATE lines, newest generation first."""
        f = self.cfg.get("families") or {}
        out = [("defaults", "")]
        if f.get("queue", True):
            for jid, ds in sorted(self.held_pairs().items()):
                if len(ds) == 2 and all(self.passes(d) for d in ds):
                    trial = glob.glob(os.path.join(self.q, "results", jid, self.home, "out", jid, "*", "trial.weights"))
                    if trial:
                        out.append(("%s/%s" % (self.name, jid), self.weights_set(open(trial[0]).read())))
        for run in f.get("converge", []):
            run = os.path.join(TC, run) if not os.path.isabs(run) else run
            rows = []
            for l in open(os.path.join(run, "candidates.txt")):
                m = re.match(r"^(gen(\d+)/\S+) .*CANDIDATE", l)
                if m and os.path.exists(os.path.join(run, m.group(1) + ".weights")):
                    rows.append((int(m.group(2)), m.group(1)))
            for g, name in sorted(rows, key=lambda r: (-r[0], r[1])):
                text = open(os.path.join(run, name + ".weights")).read()
                out.append(("%s/%s" % (os.path.basename(run.rstrip("/")), name), self.weights_set(text)))
        return out

    def family_jobs(self, n):
        f = self.cfg.get("families")
        cpu = [h for h in self.hosts if "cpu" in h.get("caps", ["cpu"]) and self.prepared(h["name"])]
        if not f or not cpu or n <= 0:
            return []
        todo = [c for c in self.family_candidates() if c[0] not in self.state["families_queued"]]
        batch = int(f.get("batch", 8))
        jobs = []
        while todo and len(jobs) < n:
            part, todo = todo[:batch], todo[batch:]
            fid = "f%04d" % self.state["families_n"]
            self.state["families_n"] += 1
            lines = ["%s\t%s" % (name, s) for name, s in part]
            b = base64.b64encode(("\n".join(lines) + "\n").encode()).decode()
            # The test fails when a family is under its floor: that is a
            # verdict, kept in the output, not a failed job (the breaker).
            cmd = (
                "set -e; Q=~/%s; O=$Q/out/%s; rm -rf $O; mkdir -p $O; echo %s | base64 -d > $O/candidates.tsv\n"
                "N=$(nproc 2>/dev/null || sysctl -n hw.ncpu)\n"
                "i=0; while IFS=$'\\t' read -r name set; do\n"
                "  i=$((i+1)); ( printf '%%s\\n%%s\\n' \"$name\" \"$set\" > $O/$i.set\n"
                "    t0=$(date +%%s); SCENARIO_SET=\"$set\" $Q/census-bin --ignored every_family_holds_its_floor --nocapture > $O/$i.log 2>&1 && r=holds || r=UNDER\n"
                "    echo \"$r $(( $(date +%%s) - t0 ))\" > $O/$i.verdict ) &\n"
                "  while [ $(jobs -rp | wc -l) -ge $N ]; do sleep 2; done\n"
                "done < $O/candidates.tsv; wait\n"
                "for v in $O/*.verdict; do k=$(basename $v .verdict); echo \"$(cat $v) $(head -1 $O/$k.set)\"; done\n"
            ) % (self.home, fid, b)
            jobs.append({"id": fid, "kind": "families", "title": "families: %s" % ", ".join(name for name, _ in part)[:110],
                         "needs": "cpu", "after_host": "prep-{host}", "cmd": cmd,
                         "fetch": "%s/out/%s" % (self.home, fid), "timeout_s": 12 * 3600})
            self.state["families_queued"] += [name for name, _ in part]
        return jobs

    def families(self):
        """Every families verdict fetched so far, into plan/families.json."""
        out = {}
        for d in sorted(glob.glob(os.path.join(self.q, "results", "f*", self.home, "out", "f*", ""))):
            for sf in glob.glob(os.path.join(d, "*.set")):
                k = sf[:-4]
                name, _, sset = open(sf).read().partition("\n")
                try:
                    verdict, secs = open(k + ".verdict").read().split()
                except (OSError, ValueError):
                    continue
                fams = []
                for l in open(k + ".log", errors="replace"):
                    m = re.match(r"^family (\S+) \[.*\]: held (\d+) of (\d+), ([\d.]+)% \+/- ([\d.]+)%, floor (\S+), (.+) \(([\d.]+) s\)", l)
                    if m:
                        fams.append({"family": m.group(1)[:-5], "held": int(m.group(2)), "runs": int(m.group(3)),
                                     "pct": float(m.group(4)), "se": float(m.group(5)), "floor": m.group(6), "verdict": m.group(7)})
                out[name] = {"set": sset.strip(), "verdict": verdict, "seconds": int(secs), "families": fams,
                             "job": d.rstrip("/").split(os.sep)[-1]}
        save(os.path.join(self.pd, "families.json"), out)
        held = [k for k, v in out.items() if v["verdict"] == "holds"]
        print("families: %d candidates read, %d hold every floor: %s" % (len(out), len(held), ", ".join(held) or "none"))
        return out

    # ---- the keeper's two calls --------------------------------------------------------
    def plan(self, n):
        jobs = self.prepare_jobs()
        jobs += self.net_jobs()
        # The families first, a share of the queue: a candidate waits on them.
        jobs += self.family_jobs(max(0, (n - len(jobs) + 1) // 2))
        jobs += self.weight_jobs(max(0, n - len(jobs)))
        self.keep()
        print(json.dumps(jobs))

    def collect(self):
        rows = []
        for d in self.census_dirs():
            jid = d.rstrip("/").split(os.sep)[-2]
            p = self.passes(d)
            trial = ""
            try:
                trial = "; ".join(l.split("#")[0].strip() for l in open(os.path.join(d, "trial.weights")) if "=" in l.split("#")[0])
            except OSError:
                pass
            every = ""
            try:
                every = next(l for l in open(os.path.join(d, "pooled.txt")) if l.startswith("every map")).strip()
            except (OSError, StopIteration):
                pass
            rows.append({"job": jid, "trial": trial, "every_map": every, "passes": p})
        rounds = [load(f) for f in sorted(glob.glob(os.path.join(self.pd, "round-*.json")))]
        summary = {"weights": rows, "rounds": rounds, "best": load(os.path.join(self.pd, "best.json")), "families": self.families()}
        save(os.path.join(self.pd, "summary.json"), summary)
        passed = [r["job"] for r in rows if r["passes"]]
        print("collected: %d weight censuses (%d past two errors: %s); %d net rounds judged" % (len(rows), len(passed), ", ".join(passed) or "none", len(rounds)))


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    p = Plan(sys.argv[1])
    if sys.argv[2] == "plan":
        p.plan(int(sys.argv[3]) if len(sys.argv) > 3 else 4)
    elif sys.argv[2] == "collect":
        p.collect()
    elif sys.argv[2] == "families":
        p.families()
    elif sys.argv[2] == "judged":
        p.judged(sys.argv[3])
    else:
        sys.exit(__doc__)
