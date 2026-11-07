#!/usr/bin/env python3
"""Keep a Relabel campaign's slots busy on spot until its label set is complete.

A campaign is one label set (this commit's default net at --nodes), a list of
generations and a fixed number of slots N. Slot I is the Relabel task "I of N":
it owns every Nth self-play worker, and a task skips shards already relabelled,
so a slot relaunched after a spot reclamation resumes exactly its own leftovers.
The work left is never tracked here: every poll recomputes it from S3 (source
manifests minus relabelled manifests), so it stays right while instances run.

This only dispatches the Relabel workflow (one slot a dispatch) and reads S3 and
GitHub. It never launches EC2 itself, so the `relabel` role's limits (spot only,
read selfplay/, write only selfplay/relabel/) still bound everything it starts.

  ci/relabel-manager.py status --generations 3,4,5
  ci/relabel-manager.py run    --generations 3,4,5 --max-instance-hours 40 [--dry-run]
  ci/relabel-manager.py stop   --generations 3,4,5     no new dispatches; running tasks finish
  ci/relabel-manager.py cancel --generations 3,4,5     cancel the campaign's running Relabel runs

AWS credentials for reading S3 come from the environment (AWS_PROFILE). The
design and the runbook: docs/relabel.md#the-relabel-manager.
"""

import argparse
import base64
import datetime as dt
import fcntl
import json
import math
import os
import re
import shutil
import signal
import subprocess
import sys
import time

REPO = 'jwallace145/rusty-chess-v2'
DATASET = 's3://jwall-engineering-rusty-chess-nnue/data/selfplay'
TITLE = re.compile(r'^Relabel: gen ([0-9,]+), (\d+) × (\d+) min, (\d+) nodes(?:, slots ([0-9,]+))?')
INTERRUPTED = re.compile(r'warning: interrupted by spot reclamation')
NO_CAPACITY = re.compile(r'error: no (?:spot |on-demand )?\S* ?capacity|no-capacity')
LAUNCHED = '== relabel:'
DONE_SHARDS = re.compile(r'relabel-task: (\d+) shards, \d+ records|relabel-task: interrupted; (\d+) complete shards')
NO_PROGRESS_LIMIT = 3  # launched runs in a row that relabel nothing before a slot pauses


# ---- pure logic (unit tested in ci/test-relabel-manager.py) ----------------

def worker_of(shard):
    """gen=N/run=R/task=T/t0-w017-s00005 -> gen=N/run=R/task=T/t0-w017."""
    return re.sub(r'-s[0-9]+$', '', shard)


def slot_of_workers(source, n):
    """Slot of every worker, as ci/relabel-task.sh assigns them: every Nth of the sorted list."""
    return {w: i % n for i, w in enumerate(sorted({worker_of(s) for s in source}))}


def remaining_by_slot(source, done, n):
    slots = slot_of_workers(source, n)
    left = [0] * n
    for s in set(source) - set(done):
        left[slots[worker_of(s)]] += 1
    return left


def parse_title(title):
    m = TITLE.match(title or '')
    if not m:
        return None
    return {'generations': m[1], 'instances': int(m[2]), 'minutes': int(m[3]), 'nodes': int(m[4]),
            'slots': [int(x) for x in m[5].split(',')] if m[5] else None}


def classify(conclusion, log):
    """A finished slot job: finished, interrupted, no-capacity, cancelled or failed; launched or not; shards done."""
    launched = LAUNCHED in log
    shards = 0
    for m in DONE_SHARDS.finditer(log):
        shards = int(m[1] or m[2])
    if conclusion == 'success':
        outcome = 'finished'
    elif conclusion == 'cancelled':
        outcome = 'cancelled'
    elif INTERRUPTED.search(log):
        outcome = 'interrupted'
    elif NO_CAPACITY.search(log) and not launched:
        outcome = 'no-capacity'
    else:
        outcome = 'failed'
    return {'outcome': outcome, 'launched': launched, 'shards': shards}


def hard_hours(minutes):
    """The instance's hard limit, as the Relabel workflow sets it."""
    return min(6, (minutes + 59) // 60 + 1)


def plan(left, busy, paused, backoff_until, now, *, blocked, committed_hours, cap_hours, minutes,
         launches, max_launches):
    """Which slots to dispatch now, and why the others wait. Returns (dispatch, waits, finished)."""
    dispatch, waits = [], {}
    if sum(left) == 0 and not busy:
        return [], {}, True
    budget = committed_hours
    for i, n in enumerate(left):
        if n == 0:
            waits[i] = 'done'
        elif i in busy:
            waits[i] = 'running'
        elif i in paused:
            waits[i] = f'paused: {paused[i]}'
        elif blocked:
            waits[i] = blocked
        elif backoff_until.get(i, 0) > now:
            waits[i] = f'no capacity; retry in {math.ceil((backoff_until[i] - now) / 60)} min'
        elif launches + len(dispatch) >= max_launches:
            waits[i] = f'launch cap ({max_launches}) reached'
        elif budget + hard_hours(minutes) > cap_hours:
            waits[i] = f'budget: {budget:.1f} h committed + {hard_hours(minutes)} h > {cap_hours} h'
        else:
            dispatch.append(i)
            budget += hard_hours(minutes)
    return dispatch, waits, False


def after_outcome(result, left_before, left_after, no_progress):
    """What a finished slot run means for that slot: (pause reason or None, retry delay in s, no-progress count)."""
    progressed = left_after < left_before
    o = result['outcome']
    if o == 'failed':
        return 'the task failed; read its log before relaunching', 0, no_progress
    if o == 'cancelled':
        return 'its run was cancelled', 0, no_progress
    if o == 'no-capacity':
        return None, 600, no_progress
    if left_after == 0:
        return None, 0, 0
    if o == 'finished' and not progressed:
        return 'finished without relabelling anything although work is left (slot mismatch?)', 0, no_progress
    no_progress = 0 if progressed else no_progress + 1
    if no_progress >= NO_PROGRESS_LIMIT:
        return f'{no_progress} launches in a row relabelled nothing', 0, no_progress
    return None, 0, no_progress


# ---- the outside world -----------------------------------------------------

def sh(args, timeout=120, input=None):
    p = subprocess.run(args, capture_output=True, text=True, timeout=timeout, input=input)
    if p.returncode != 0:
        raise RuntimeError(f'{" ".join(args[:4])}…: exit {p.returncode}: {(p.stderr or p.stdout).strip()[:300]}')
    return p.stdout


ANSI = re.compile(r'\x1b\[[0-9;]*[A-Za-z]')


def gh_api(path, raw=False):
    if not raw:
        return json.loads(sh(['gh', 'api', '-H', 'Accept: application/vnd.github+json', path]))
    # Job logs carry terminal colours, which newer gh refuses to print without the flag.
    try:
        out = sh(['gh', 'api', '--allow-escape-sequences', path])
    except RuntimeError as e:
        if 'unknown flag' not in str(e):
            raise
        out = sh(['gh', 'api', path])
    return ANSI.sub('', out)


def iso(t):
    return dt.datetime.fromisoformat(t.replace('Z', '+00:00')).timestamp() if t else None


def list_shards(root, generations):
    """Every shard with a manifest under root, as gen=N/run=R/task=T/<shard>."""
    key_prefix = root.split('/', 3)[3] + '/'
    out = []
    for g in generations.split(','):
        p = subprocess.run(['aws', 's3', 'ls', '--recursive', f'{root}/gen={g}/'], capture_output=True, text=True, timeout=600)
        if p.returncode == 1 and not p.stdout and not p.stderr.strip():
            continue  # `aws s3 ls` exits 1, silently, on an empty prefix
        if p.returncode != 0:
            raise RuntimeError(f'aws s3 ls {root}/gen={g}/: exit {p.returncode}: {p.stderr.strip()[:300]}')
        for line in p.stdout.splitlines():
            key = line.split()[-1]
            if '/manifests/' in key and key.endswith('.json'):
                out.append(key[len(key_prefix):].replace('/manifests/', '/')[:-len('.json')])
    return out


def default_net(sha):
    src = base64.b64decode(gh_api(f'repos/{REPO}/contents/crates/search/src/nnue.rs?ref={sha}')['content']).decode()
    m = re.search(r'pub const NETS: \[EmbeddedNet; \d+\] = \[EmbeddedNet \{\s*id: "([^"]+)"', src)
    if not m:
        raise RuntimeError(f'no default net in nnue.rs at {sha}')
    return m[1]


def notify(args, text):
    if args.notify and shutil.which('osascript'):
        subprocess.run(['osascript', '-e', f'display notification {json.dumps(text)} with title "relabel-manager"'],
                       capture_output=True)


class Campaign:
    def __init__(self, args):
        self.args = args
        self.n = args.instances
        self.gens = ','.join(str(g) for g in sorted({int(g) for g in args.generations.split(',')}))
        self.sha = gh_api(f'repos/{REPO}/commits/main')['sha']
        self.net = default_net(self.sha)
        self.label = getattr(args, 'label_id', None) or f'{self.net}-n{args.nodes}'
        self.dir = os.path.expanduser(f'{args.state_dir}/{self.label}-g{self.gens.replace(",", "")}-of{self.n}')
        os.makedirs(self.dir, exist_ok=True)
        self.log_path = f'{self.dir}/log.jsonl'
        self.stop_path = f'{self.dir}/STOP'
        self.source = None
        self.tracked = {}      # (run id, slot) -> {'left': remaining at dispatch, 'adopted': bool}
        self.used_hours = 0.0  # finished runs that launched an instance, start to end of the job
        self.launches = 0
        self.dispatches = 0
        self.paused, self.backoff, self.no_progress = {}, {}, {}
        self.main_moved = False
        self.restore()

    # The log is the campaign's record: every dispatch and outcome, for docs/datasets.md.
    def record(self, event, **kw):
        kw = {'time': dt.datetime.now(dt.timezone.utc).isoformat(timespec='seconds'), 'event': event, **kw}
        with open(self.log_path, 'a') as f:
            f.write(json.dumps(kw) + '\n')

    def restore(self):
        if not os.path.exists(self.log_path):
            return
        for line in open(self.log_path):
            e = json.loads(line)
            if e['event'] in ('dispatch', 'adopt'):
                self.tracked[(e['run'], e['slot'])] = {'left': e['left'], 'adopted': e['event'] == 'adopt'}
                self.dispatches += e['event'] == 'dispatch'
            elif e['event'] == 'outcome':
                self.tracked.pop((e['run'], e['slot']), None)
                self.used_hours += e.get('hours', 0)
                self.launches += e.get('launched', False)

    def say(self, text, alert=False):
        print(f'{dt.datetime.now():%H:%M:%S} {text}', flush=True)
        if alert:
            notify(self.args, text)

    def remaining(self):
        if self.source is None:
            self.source = list_shards(f'{self.args.dataset}/v1', self.gens)
            if not self.source:
                raise RuntimeError(f'no source shards for generations {self.gens}')
        source = set(self.source)
        done = [d for d in list_shards(f'{self.args.dataset}/relabel/{self.label}', self.gens) if d in source]
        return remaining_by_slot(self.source, done, self.n), len(done)

    def check_existing_commit(self, done_count):
        """A label set made at two commits fails its audit: refuse to add to one made at another."""
        if done_count == 0:
            return
        root = f'{self.args.dataset}/relabel/{self.label}'
        bucket = root.split('/')[2]
        key_prefix = root.split('/', 3)[3]
        for g in self.gens.split(','):
            for shard in list_shards(root, g)[:3]:
                gen_run_task, name = shard.rsplit('/', 1)
                key = f'{key_prefix}/{gen_run_task}/manifests/{name}.json'
                m = json.loads(sh(['aws', 's3', 'cp', '--only-show-errors', f's3://{bucket}/{key}', '-']))
                if m['relabel']['engine_sha'] != self.sha:
                    raise RuntimeError(f'{key} was relabelled at {m["relabel"]["engine_sha"][:7]}, but main is '
                                       f'{self.sha[:7]}; a label set must come from one commit')

    def runs(self):
        """Active Relabel runs: this campaign's, by slot, and anyone else's."""
        mine, foreign = {}, []
        data = gh_api(f'repos/{REPO}/actions/workflows/relabel.yml/runs?per_page=30')
        for r in data['workflow_runs']:
            if r['status'] == 'completed':
                continue
            t = parse_title(r['display_title'])
            ours = t and t['generations'] == self.gens and t['instances'] == self.n and t['nodes'] == self.args.nodes \
                and r['head_sha'] == self.sha
            if not ours:
                foreign.append(r['id'])
                continue
            jobs = self.slot_jobs(r['id'])
            slots = [s for s, j in jobs.items() if j['status'] != 'completed'] if jobs else (t['slots'] or list(range(self.n)))
            for s in slots:
                mine[s] = r['id']
        return mine, foreign

    def slot_jobs(self, run):
        jobs = gh_api(f'repos/{REPO}/actions/runs/{run}/jobs?per_page=100')['jobs']
        out = {}
        for j in jobs:
            m = re.match(r'Relabel \(t(\d+)\)', j['name'])
            if m:
                out[int(m[1])] = j
        return out

    def settle(self, left):
        """Classify every tracked slot run that has finished, and decide what it means for its slot."""
        for (run, slot), info in list(self.tracked.items()):
            r = gh_api(f'repos/{REPO}/actions/runs/{run}')
            if r['status'] != 'completed':
                continue
            job = self.slot_jobs(run).get(slot)
            if job is None:
                res, hours = {'outcome': 'failed', 'launched': False, 'shards': 0}, 0.0
            else:
                log = gh_api(f'repos/{REPO}/actions/jobs/{job["id"]}/logs', raw=True)
                res = classify(job['conclusion'], log)
                start, end = iso(job['started_at']), iso(job['completed_at'])
                hours = (end - start) / 3600 if res['launched'] and start and end else 0.0
            self.used_hours += hours
            self.launches += res['launched']
            reason, delay, self.no_progress[slot] = after_outcome(res, info['left'], left[slot], self.no_progress.get(slot, 0))
            self.record('outcome', run=run, slot=slot, hours=round(hours, 3), left_before=info['left'], left_after=left[slot], **res)
            self.say(f'slot {slot}: run {run} {res["outcome"]}{" after launching" if res["launched"] else ""}, '
                     f'{res["shards"]} shards; {left[slot]} left in the slot ({hours:.2f} h)', alert=res['outcome'] != 'no-capacity')
            if reason:
                self.paused[slot] = reason
                self.record('pause', slot=slot, reason=reason)
                self.say(f'slot {slot} paused: {reason}', alert=True)
            if delay:
                self.backoff[slot] = time.time() + delay
            del self.tracked[(run, slot)]

    def committed_hours(self, busy):
        """Hours spent plus, for each running slot, its whole hard limit: the most it can still cost."""
        return self.used_hours + len(busy) * hard_hours(self.args.minutes)

    def dispatch(self, slot, left):
        a = self.args
        note = f'relabel-manager {self.label} slot {slot}'
        since = time.time()
        out = sh(['gh', 'workflow', 'run', 'Relabel', '--repo', REPO, '--ref', 'main', '-f', f'generations={self.gens}',
                  '-f', f'instances={self.n}', '-f', f'slots={slot}', '-f', f'minutes={a.minutes}', '-f', f'nodes={a.nodes}',
                  '-f', f'note={note}'], timeout=60)
        m = re.search(r'/actions/runs/(\d+)', out)
        run = int(m[1]) if m else None
        for _ in range(12):
            if run:
                break
            time.sleep(5)
            for r in gh_api(f'repos/{REPO}/actions/workflows/relabel.yml/runs?per_page=10')['workflow_runs']:
                t = parse_title(r['display_title'])
                if t and t['slots'] == [slot] and iso(r['created_at']) >= since - 5 and note in r['display_title']:
                    run = r['id']
        if not run:
            raise RuntimeError(f'dispatched slot {slot} but found no run for it')
        self.dispatches += 1
        self.tracked[(run, slot)] = {'left': left, 'adopted': False}
        self.record('dispatch', run=run, slot=slot, left=left, sha=self.sha, label=self.label, minutes=a.minutes)
        self.say(f'slot {slot}: dispatched run {run} ({left} shards left in the slot) '
                 f'https://github.com/{REPO}/actions/runs/{run}')


def campaign_args(p):
    p.add_argument('--generations', required=True, help='e.g. 3,4,5')
    p.add_argument('--instances', type=int, default=3, choices=(1, 2, 3), help='slots N: the campaign keeps this for its life')
    p.add_argument('--nodes', type=int, default=5000)
    p.add_argument('--dataset', default=DATASET)
    p.add_argument('--state-dir', default='~/.rusty-chess/relabel-manager')


def show(c, left, done, busy, waits):
    total = sum(left)
    print(f'  {c.label}, generations {c.gens}, {c.n} slots, main {c.sha[:7]}: '
          f'{len(c.source) - total} of {len(c.source)} shards done, {total} left', flush=True)
    for i in range(c.n):
        state = f'running run {busy[i]}' if i in busy else waits.get(i, '')
        print(f'    slot {i}: {left[i]:6d} left  {state}', flush=True)
    print(f'  instance time {c.used_hours:.1f} h finished (about ${c.used_hours * 0.89:.0f}–{c.used_hours * 1.33:.0f} on spot, '
          f'an estimate), {c.launches} launches, {c.dispatches} dispatches', flush=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest='cmd', required=True)
    for name in ('status', 'stop', 'cancel'):
        campaign_args(sub.add_parser(name))
    sub.choices['status'].add_argument('--label-id', help="read another label set, such as an older net's (status only)")
    r = sub.add_parser('run')
    campaign_args(r)
    r.add_argument('--max-instance-hours', type=float, required=True,
                   help='budget: hours used plus each running slot\'s hard limit may not pass this')
    r.add_argument('--max-launches', type=int, default=40, help='instances started, at most')
    r.add_argument('--minutes', type=int, default=280, help='minutes per instance (Relabel\'s input)')
    r.add_argument('--poll', type=int, default=300, help='seconds between polls')
    r.add_argument('--dry-run', action='store_true', help='read and decide, never dispatch')
    r.add_argument('--yes', action='store_true', help='skip the confirmation')
    r.add_argument('--notify', action='store_true', help='macOS notifications for outcomes and pauses')
    r.add_argument('--once', action='store_true', help='one poll, then exit (with --dry-run, a rehearsal)')
    for p in (sub.choices['status'], sub.choices['stop'], sub.choices['cancel']):
        p.set_defaults(notify=False, minutes=280)
    sub.choices['cancel'].add_argument('--yes', action='store_true')
    args = ap.parse_args()

    c = Campaign(args)
    if args.cmd == 'stop':
        open(c.stop_path, 'w').close()
        print(f'{c.stop_path} written: the manager stops dispatching at its next poll; running tasks finish their '
              f'minutes. Delete the file before the next `run`.')
        return 0
    if args.cmd == 'cancel':
        busy, _ = c.runs()
        runs = sorted(set(busy.values()))
        if not runs:
            print('no running runs in this campaign')
            return 0
        print(f'cancel runs {runs}? Their instances stop; complete shards are kept.')
        if not args.yes and input('type "cancel" to confirm: ').strip() != 'cancel':
            return 1
        for run in runs:
            sh(['gh', 'run', 'cancel', str(run), '--repo', REPO])
            print(f'cancelled {run}')
        return 0

    left, done = c.remaining()
    busy, foreign = c.runs()
    if args.cmd == 'status':
        _, waits, finished = plan(left, busy, {}, {}, time.time(), blocked=None, committed_hours=0, cap_hours=math.inf,
                                  minutes=args.minutes, launches=0, max_launches=math.inf)
        show(c, left, done, busy, waits)
        if foreign:
            print(f'  other Relabel runs active: {foreign}')
        if finished:
            print('  complete')
        return 0

    c.check_existing_commit(done)
    lock = open(f'{c.dir}/lock', 'w')
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        print(f'another manager is running this campaign ({c.dir})')
        return 1
    show(c, left, done, busy, {})
    print(f'  budget {args.max_instance_hours} instance-hours (each launch may use up to {hard_hours(args.minutes)}), '
          f'at most {args.max_launches} launches, {args.minutes} minutes an instance, polling every {args.poll} s'
          f'{", DRY RUN: nothing is dispatched" if args.dry_run else ""}')
    print(f'  log: {c.log_path}\n  stop: Ctrl-C, or `ci/relabel-manager.py stop --generations {c.gens}`')
    if os.path.exists(c.stop_path):
        print(f'  {c.stop_path} exists from an earlier stop; delete it to run')
        return 1
    if not args.dry_run and not args.yes and input(f'type the label id ({c.label}) to start: ').strip() != c.label:
        return 1
    c.record('start', sha=c.sha, label=c.label, generations=c.gens, instances=c.n, minutes=args.minutes,
             cap_hours=args.max_instance_hours, max_launches=args.max_launches, dry_run=args.dry_run)

    stopping = []
    def on_signal(sig, _):
        if stopping:
            sys.exit(130)
        stopping.append(sig)
        c.say('stopping: no new dispatches (Ctrl-C again to quit at once)')
    signal.signal(signal.SIGINT, on_signal)
    signal.signal(signal.SIGTERM, on_signal)

    while True:
        try:
            left, done = c.remaining()
            busy, foreign = c.runs()
            for s, run in busy.items():
                if (run, s) not in c.tracked:
                    c.tracked[(run, s)] = {'left': left[s], 'adopted': True}
                    c.record('adopt', run=run, slot=s, left=left[s])
                    c.say(f'slot {s}: following run {run}, which this manager didn\'t start')
            c.settle(left)
            busy, foreign = c.runs()
            if not c.main_moved and gh_api(f'repos/{REPO}/commits/main')['sha'] != c.sha:
                c.main_moved = True
                c.record('main-moved')
                c.say(f'main moved off {c.sha[:7]}: no more dispatches, since a label set must come from one commit. '
                      f'Running tasks finish; start a new campaign (a new label set) from the new main.', alert=True)
            stop = stopping or os.path.exists(c.stop_path)
            blocked = ('stopping' if stop else 'main moved' if c.main_moved
                       else f'other Relabel runs active: {foreign}' if foreign else None)
            todo, waits, finished = plan(left, busy, c.paused, c.backoff, time.time(), blocked=blocked,
                                         committed_hours=c.committed_hours(busy), cap_hours=args.max_instance_hours,
                                         minutes=args.minutes, launches=c.launches + len(busy), max_launches=args.max_launches)
            if finished:
                c.record('complete', used_hours=round(c.used_hours, 3))
                c.say(f'complete: every shard of generations {c.gens} is in {c.label}. Audit it before training on it '
                      f'(ci/label-set-audit.sh).', alert=True)
                return 0
            for s in todo:
                if args.dry_run:
                    waits[s] = 'would dispatch (dry run)'
                else:
                    c.dispatch(s, left[s])
                    busy[s] = 'new'
            show(c, left, done, busy, waits)
            if (stop or c.main_moved) and not busy:
                c.say('stopped; nothing of this campaign is running')
                return 0
            if not busy and not todo and all(w.startswith(('paused', 'done', 'budget', 'launch cap')) for w in waits.values()):
                c.say('nothing can run: every slot with work left is paused or out of budget', alert=True)
                return 1
            if args.once:
                return 0
        except (RuntimeError, subprocess.TimeoutExpired, json.JSONDecodeError, KeyError) as e:
            c.say(f'poll failed, trying again next poll: {e}')
        for _ in range(args.poll):
            if stopping and not c.tracked:
                break
            time.sleep(1)


if __name__ == '__main__':
    sys.exit(main())
