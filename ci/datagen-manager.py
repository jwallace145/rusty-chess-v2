#!/usr/bin/env python3
"""Keep N Datagen instances generating on spot until a generation reaches its target.

A campaign is one generation (the one datagen.toml names at main's commit), a
target number of positions and a number of instances N. Every Datagen run the
manager dispatches is one instance; each run gets its own seeds from its run
id, so a replacement never replays the games a reclaimed instance already
uploaded, and the shards already in S3 are the whole record of progress.
Progress is never tracked here: every poll counts the generation's manifests
in S3 and estimates its positions from a sample of them.

This only dispatches the Datagen workflow and reads S3 and GitHub. It never
launches EC2 itself, so the `datagen` role's limits (spot only, write only the
dataset prefix) still bound everything it starts.

  ci/datagen-manager.py status --generation 6
  ci/datagen-manager.py run    --generation 6 --target-positions 600000000 --max-instance-hours 20 [--dry-run]
  ci/datagen-manager.py stop   --generation 6     no new dispatches; running instances finish
  ci/datagen-manager.py cancel --generation 6     cancel the campaign's running Datagen runs

AWS credentials for reading S3 come from the environment (AWS_PROFILE). The
runbook: docs/datagen-runbook.md#the-datagen-manager. The shared helpers and
conventions are ci/relabel-manager.py's.
"""

import argparse
import base64
import datetime as dt
import fcntl
import importlib.util
import json
import math
import os
import re
import shutil
import signal
import subprocess
import sys
import time

_spec = importlib.util.spec_from_file_location('relabel_manager', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'relabel-manager.py'))
rm = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rm)
sh, gh_api, iso, hard_hours, REPO, DATASET = rm.sh, rm.gh_api, rm.iso, rm.hard_hours, rm.REPO, rm.DATASET

TITLE = re.compile(r'^Datagen(?: benchmark)?: (\d+) × (\d+) min, (\S+)(?: — (.*))?$')
LAUNCHED = '== datagen:'
DONE_SHARDS = re.compile(r'datagen-task: (\d+) shards, (\d+) positions in|datagen-task: interrupted; (\d+) complete shards')
NO_PROGRESS_LIMIT = 3  # launched runs in a row that upload nothing before the campaign pauses
DEFAULT_SHARD_POSITIONS = 24_300  # about: shards hold whole games up to shard_positions (25,000)
DEFAULT_RATE = 80_000_000  # positions an instance-hour, the workflow's figure, until a run of this campaign measures it
MIN_MINUTES = 30  # the shortest dispatch worth an instance's boot and build


# ---- pure logic (unit tested in ci/test-datagen-manager.py) ----------------

def parse_title(title):
    """Datagen: 1 × 280 min, native — datagen-manager gen 6 -> its parts, or None."""
    m = TITLE.match(title or '')
    if not m:
        return None
    return {'instances': int(m[1]), 'minutes': int(m[2]), 'target_cpu': m[3], 'note': m[4] or '',
            'benchmark': title.startswith('Datagen benchmark')}


def classify(conclusion, log):
    """A finished Generate job: finished, interrupted, no-capacity, cancelled or failed; launched or not; shards and positions uploaded."""
    launched = LAUNCHED in log
    shards, positions = 0, 0
    for m in DONE_SHARDS.finditer(log):
        if m[1]:
            shards, positions = int(m[1]), int(m[2])
        else:
            shards = int(m[3])
    if conclusion == 'success':
        outcome = 'finished'
    elif conclusion == 'cancelled':
        outcome = 'cancelled'
    elif rm.INTERRUPTED.search(log):
        outcome = 'interrupted'
    elif rm.NO_CAPACITY.search(log) and not launched:
        outcome = 'no-capacity'
    else:
        outcome = 'failed'
    return {'outcome': outcome, 'launched': launched, 'shards': shards, 'positions': positions}


def estimate_positions(shards, mean_per_shard):
    return int(round(shards * mean_per_shard))


def size_minutes(remaining, rate, instances, max_minutes):
    """Minutes to ask of `instances` launched together so they make about `remaining` positions
    at `rate` positions an instance-hour, with a tenth over for the estimate's error, within
    MIN_MINUTES..max_minutes. The last launches of a campaign are short, not the full stretch."""
    if remaining <= 0 or instances <= 0 or rate <= 0:
        return MIN_MINUTES
    hours = remaining / (rate * instances)
    return max(MIN_MINUTES, min(max_minutes, math.ceil(hours * 60 * 1.1)))


def measured_rate(outcomes):
    """Positions an instance-hour from this campaign's finished runs (interrupted ones don't report
    positions), or the workflow's figure when none has finished."""
    pos = sum(o['positions'] for o in outcomes if o['outcome'] == 'finished' and o.get('hours', 0) > 0)
    hours = sum(o['hours'] for o in outcomes if o['outcome'] == 'finished' and o.get('hours', 0) > 0)
    return pos / hours if pos and hours else DEFAULT_RATE


def plan(est_positions, target, busy, n, paused, backoff_until, now, *, blocked, committed_hours, cap_hours,
         minutes, launches, max_launches):
    """How many instances to dispatch now, why the rest wait, and whether the campaign is done.

    Instances are interchangeable, so this is a count, not slots. The target
    counts only what is in S3: running instances' work isn't anticipated, so a
    campaign overshoots by up to N instances' last stretch, which is fine for
    a generation (more data) and bounded by --minutes.
    """
    if est_positions >= target:
        return 0, ('done' if not busy else f'target reached; {busy} still running'), busy == 0
    idle = max(0, n - busy)
    if idle == 0:
        return 0, f'{busy} running', False
    if paused:
        return 0, f'paused: {paused}', False
    if blocked:
        return 0, blocked, False
    if backoff_until > now:
        return 0, f'no capacity; retry in {math.ceil((backoff_until - now) / 60)} min', False
    dispatch, budget, why = 0, committed_hours, ''
    for _ in range(idle):
        if launches + dispatch >= max_launches:
            why = f'launch cap ({max_launches}) reached'
            break
        if budget + hard_hours(minutes) > cap_hours:
            why = f'budget: {budget:.1f} h committed + {hard_hours(minutes)} h > {cap_hours} h'
            break
        dispatch += 1
        budget += hard_hours(minutes)
    return dispatch, why, False


def after_outcome(result, no_progress, at_target=False):
    """What a finished run means: (pause reason or None, retry delay in s, no-progress count).
    A run cancelled once the target was reached is the manager's own doing, not a reason to pause."""
    o = result['outcome']
    if o == 'failed':
        return 'the task failed; read its log before relaunching', 0, no_progress
    if o == 'cancelled':
        return (None, 0, no_progress) if at_target else ('its run was cancelled', 0, no_progress)
    if o == 'no-capacity':
        return None, 600, no_progress
    no_progress = 0 if result['shards'] > 0 else no_progress + 1
    if no_progress >= NO_PROGRESS_LIMIT:
        return f'{no_progress} launches in a row uploaded nothing', 0, no_progress
    return None, 0, no_progress


# ---- the outside world -----------------------------------------------------

def notify(args, text):
    if args.notify and shutil.which('osascript'):
        subprocess.run(['osascript', '-e', f'display notification {json.dumps(text)} with title "datagen-manager"'],
                       capture_output=True)


def generation_at(sha):
    src = base64.b64decode(gh_api(f'repos/{REPO}/contents/datagen.toml?ref={sha}')['content']).decode()
    m = re.search(r'^generation\s*=\s*(\d+)', src, re.M)
    if not m:
        raise RuntimeError(f'no generation in datagen.toml at {sha}')
    return int(m[1])


class Campaign:
    def __init__(self, args):
        self.args = args
        self.n = args.instances
        self.gen = args.generation
        self.sha = gh_api(f'repos/{REPO}/commits/main')['sha']
        at_main = generation_at(self.sha)
        if at_main != self.gen:
            raise RuntimeError(f'datagen.toml at main ({self.sha[:7]}) says generation {at_main}, not {self.gen}')
        self.root = f'{args.dataset}/v1'
        self.dir = os.path.expanduser(f'{args.state_dir}/gen{self.gen}-{self.sha[:7]}')
        os.makedirs(self.dir, exist_ok=True)
        self.log_path = f'{self.dir}/log.jsonl'
        self.stop_path = f'{self.dir}/STOP'
        self.note = f'datagen-manager gen {self.gen}'
        self.tracked = {}      # run id -> {'shards': shards in S3 at dispatch, 'adopted': bool}
        self.used_hours = 0.0
        self.launches = 0
        self.dispatches = 0
        self.paused, self.backoff, self.no_progress = None, 0.0, 0
        self.main_moved = False
        self.outcomes = []       # for the measured rate
        self.target_reached = False
        self.mean_per_shard = DEFAULT_SHARD_POSITIONS
        self.restore()

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
                self.tracked[e['run']] = {'shards': e['shards'], 'adopted': e['event'] == 'adopt'}
                self.dispatches += e['event'] == 'dispatch'
            elif e['event'] == 'outcome':
                self.tracked.pop(e['run'], None)
                self.used_hours += e.get('hours', 0)
                self.launches += e.get('launched', False)
                self.outcomes.append(e)

    def say(self, text, alert=False):
        print(f'{dt.datetime.now():%H:%M:%S} {text}', flush=True)
        if alert:
            notify(self.args, text)

    def shards(self):
        """The generation's shards in S3, as gen=N/run=R/task=T/<shard>."""
        return rm.list_shards(self.root, str(self.gen))

    def manifest(self, shard):
        bucket = self.root.split('/')[2]
        key_prefix = self.root.split('/', 3)[3]
        gen_run_task, name = shard.rsplit('/', 1)
        return json.loads(sh(['aws', 's3', 'cp', '--only-show-errors', f's3://{bucket}/{key_prefix}/{gen_run_task}/manifests/{name}.json', '-']))

    def sample_manifests(self, shards, k=12):
        """Mean positions a shard from an evenly spaced sample, and each sampled manifest's engine commit."""
        if not shards:
            return DEFAULT_SHARD_POSITIONS, set()
        picks = sorted(shards)[::max(1, len(shards) // k)][:k]
        manifests = [self.manifest(s) for s in picks]
        mean = sum(m['positions'] for m in manifests) / len(manifests)
        shas = {(m.get('provenance') or {}).get('engine_sha') for m in manifests} - {None}
        return mean, shas

    def check_existing_commit(self, shas):
        """A generation's data should come from one commit: refuse to add to one made at another."""
        others = {s for s in shas if s != self.sha}
        if others:
            raise RuntimeError(f'generation {self.gen} has shards made at {", ".join(s[:7] for s in others)}, but main is '
                               f'{self.sha[:7]}; a generation comes from one commit (start it over, or bump the generation)')

    def progress(self):
        shards = self.shards()
        self.mean_per_shard, shas = self.sample_manifests(shards)
        return len(shards), estimate_positions(len(shards), self.mean_per_shard), shas

    def runs(self):
        """Active Datagen runs: this campaign's (run id -> jobs still running) and anyone else's."""
        mine, foreign = {}, []
        for r in gh_api(f'repos/{REPO}/actions/workflows/datagen.yml/runs?per_page=30')['workflow_runs']:
            if r['status'] == 'completed':
                continue
            t = parse_title(r['display_title'])
            if t and self.note in t['note'] and r['head_sha'] == self.sha and not t['benchmark']:
                mine[r['id']] = t['instances']
            else:
                foreign.append(r['id'])
        return mine, foreign

    def generate_jobs(self, run):
        return [j for j in gh_api(f'repos/{REPO}/actions/runs/{run}/jobs?per_page=100')['jobs'] if j['name'].startswith('Generate (')]

    def settle(self, shards_now):
        """Classify every tracked run that has finished."""
        for run, info in list(self.tracked.items()):
            r = gh_api(f'repos/{REPO}/actions/runs/{run}')
            if r['status'] != 'completed':
                continue
            jobs = self.generate_jobs(run)
            if not jobs:
                res, hours = {'outcome': 'failed', 'launched': False, 'shards': 0, 'positions': 0}, 0.0
            else:
                job = jobs[0]
                log = gh_api(f'repos/{REPO}/actions/jobs/{job["id"]}/logs', raw=True)
                res = classify(job['conclusion'], log)
                start, end = iso(job['started_at']), iso(job['completed_at'])
                hours = (end - start) / 3600 if res['launched'] and start and end else 0.0
            self.used_hours += hours
            self.launches += res['launched']
            reason, delay, self.no_progress = after_outcome(res, self.no_progress, self.target_reached)
            self.outcomes.append({'hours': hours, **res})
            self.record('outcome', run=run, hours=round(hours, 3), shards_before=info['shards'], shards_after=shards_now, **res)
            self.say(f'run {run} {res["outcome"]}{" after launching" if res["launched"] else ""}: {res["shards"]} shards, '
                     f'{res["positions"]} positions ({hours:.2f} h)', alert=res['outcome'] != 'no-capacity')
            if reason:
                self.paused = reason
                self.record('pause', reason=reason)
                self.say(f'paused: {reason}', alert=True)
            if delay:
                self.backoff = time.time() + delay
            del self.tracked[run]

    def committed_hours(self, busy):
        return self.used_hours + busy * hard_hours(self.args.minutes)

    def rate(self):
        return measured_rate(self.outcomes)

    def cancel_running(self, mine):
        """The target is in S3: stop the instances still generating. Complete shards are kept; each
        worker's shard in progress is lost, which is the price of not overshooting by a whole run."""
        for run in sorted(mine):
            sh(['gh', 'run', 'cancel', str(run), '--repo', REPO])
            self.record('cancel-at-target', run=run)
            self.say(f'target reached: cancelled run {run}; its complete shards are kept')

    def dispatch(self, shards_now, minutes):
        since = time.time()
        out = sh(['gh', 'workflow', 'run', 'Datagen', '--repo', REPO, '--ref', 'main', '-f', 'instances=1',
                  '-f', f'minutes={minutes}', '-f', 'target-cpu=native', '-f', 'upload=true', '-f', f'note={self.note}'], timeout=60)
        m = re.search(r'/actions/runs/(\d+)', out)
        run = int(m[1]) if m else None
        for _ in range(12):
            if run:
                break
            time.sleep(5)
            for r in gh_api(f'repos/{REPO}/actions/workflows/datagen.yml/runs?per_page=10')['workflow_runs']:
                t = parse_title(r['display_title'])
                if t and self.note in t['note'] and iso(r['created_at']) >= since - 5 and r['id'] not in self.tracked:
                    run = r['id']
        if not run:
            raise RuntimeError('dispatched a run but found no run id for it')
        self.dispatches += 1
        self.tracked[run] = {'shards': shards_now, 'adopted': False}
        self.record('dispatch', run=run, shards=shards_now, sha=self.sha, generation=self.gen, minutes=minutes)
        self.say(f'dispatched run {run} for {minutes} min ({shards_now} shards in S3 so far) https://github.com/{REPO}/actions/runs/{run}')


def campaign_args(p):
    p.add_argument('--generation', type=int, required=True, help='must equal datagen.toml\'s generation at main')
    p.add_argument('--instances', type=int, default=3, choices=(1, 2, 3), help='instances kept running')
    p.add_argument('--dataset', default=DATASET)
    p.add_argument('--state-dir', default='~/.rusty-chess/datagen-manager')


def show(c, shards, est, target, busy, why):
    pct = 100 * est / target if target else 0
    print(f'  generation {c.gen}, main {c.sha[:7]}: {shards} shards, about {est / 1e6:.1f}M positions '
          f'({pct:.0f}% of {target / 1e6:.0f}M; {c.mean_per_shard:.0f} a shard from a sample), {busy} of {c.n} instances running'
          + (f'; {why}' if why else ''), flush=True)
    print(f'  instance time {c.used_hours:.1f} h finished (about ${c.used_hours * 0.89:.0f}–{c.used_hours * 1.33:.0f} on spot, '
          f'an estimate), {c.launches} launches, {c.dispatches} dispatches', flush=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest='cmd', required=True)
    for name in ('status', 'stop', 'cancel'):
        campaign_args(sub.add_parser(name))
    r = sub.add_parser('run')
    campaign_args(r)
    r.add_argument('--target-positions', type=int, required=True, help='stop dispatching once S3 holds about this many')
    r.add_argument('--max-instance-hours', type=float, required=True,
                   help='budget: hours used plus each running instance\'s hard limit may not pass this')
    r.add_argument('--max-launches', type=int, default=30, help='instances started, at most')
    r.add_argument('--minutes', type=int, default=280, help='the most minutes an instance is asked for (Datagen\'s input, at most 300); '
                   'each dispatch is sized to the positions still needed, down to 30')
    r.add_argument('--let-finish', action='store_true', help='once the target is in S3, let running instances finish '
                   'instead of cancelling them')
    r.add_argument('--poll', type=int, default=300, help='seconds between polls')
    r.add_argument('--dry-run', action='store_true', help='read and decide, never dispatch')
    r.add_argument('--yes', action='store_true', help='skip the confirmation')
    r.add_argument('--notify', action='store_true', help='macOS notifications for outcomes and pauses')
    r.add_argument('--once', action='store_true', help='one poll, then exit (with --dry-run, a rehearsal)')
    for p in (sub.choices['status'], sub.choices['stop'], sub.choices['cancel']):
        p.set_defaults(notify=False, minutes=280, target_positions=0)
    sub.choices['status'].add_argument('--target-positions', type=int, default=600_000_000)
    sub.choices['cancel'].add_argument('--yes', action='store_true')
    args = ap.parse_args()

    c = Campaign(args)
    if args.cmd == 'stop':
        open(c.stop_path, 'w').close()
        print(f'{c.stop_path} written: the manager stops dispatching at its next poll; running instances finish their '
              f'minutes. Delete the file before the next `run`.')
        return 0
    if args.cmd == 'cancel':
        mine, _ = c.runs()
        if not mine:
            print('no running runs in this campaign')
            return 0
        print(f'cancel runs {sorted(mine)}? Their instances stop; complete shards are kept.')
        if not args.yes and input('type "cancel" to confirm: ').strip() != 'cancel':
            return 1
        for run in sorted(mine):
            sh(['gh', 'run', 'cancel', str(run), '--repo', REPO])
            print(f'cancelled {run}')
        return 0

    shards, est, shas = c.progress()
    mine, foreign = c.runs()
    busy = sum(mine.values())
    if args.cmd == 'status':
        _, why, finished = plan(est, args.target_positions, busy, c.n, None, 0, time.time(), blocked=None,
                                committed_hours=0, cap_hours=math.inf, minutes=280, launches=0, max_launches=math.inf)
        show(c, shards, est, args.target_positions, busy, why)
        if foreign:
            print(f'  other Datagen runs active: {foreign}')
        if shas - {c.sha}:
            print(f'  note: shards made at {", ".join(s[:7] for s in shas - {c.sha})}, not main ({c.sha[:7]})')
        if finished:
            print('  target reached')
        return 0

    c.check_existing_commit(shas)
    lock = open(f'{c.dir}/lock', 'w')
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        print(f'another manager is running this campaign ({c.dir})')
        return 1
    show(c, shards, est, args.target_positions, busy, '')
    print(f'  budget {args.max_instance_hours} instance-hours (each launch may use up to {hard_hours(args.minutes)}), '
          f'at most {args.max_launches} launches, {args.minutes} minutes an instance, polling every {args.poll} s'
          f'{", DRY RUN: nothing is dispatched" if args.dry_run else ""}')
    print(f'  log: {c.log_path}\n  stop: Ctrl-C, or `ci/datagen-manager.py stop --generation {c.gen}`')
    if os.path.exists(c.stop_path):
        print(f'  {c.stop_path} exists from an earlier stop; delete it to run')
        return 1
    if not args.dry_run and not args.yes and input(f'type "gen {c.gen}" to start: ').strip() != f'gen {c.gen}':
        return 1
    c.record('start', sha=c.sha, generation=c.gen, instances=c.n, minutes=args.minutes, target=args.target_positions,
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
            shards, est, shas = c.progress()
            mine, foreign = c.runs()
            for run in mine:
                if run not in c.tracked:
                    c.tracked[run] = {'shards': shards, 'adopted': True}
                    c.record('adopt', run=run, shards=shards)
                    c.say(f'following run {run}, which this manager didn\'t start')
            c.settle(shards)
            mine, foreign = c.runs()
            busy = sum(mine.values())
            if not c.main_moved and gh_api(f'repos/{REPO}/commits/main')['sha'] != c.sha:
                c.main_moved = True
                c.record('main-moved')
                c.say(f'main moved off {c.sha[:7]}: no more dispatches, since a generation comes from one commit. '
                      f'Running instances finish.', alert=True)
            stop = stopping or os.path.exists(c.stop_path)
            blocked = ('stopping' if stop else 'main moved' if c.main_moved
                       else f'other Datagen runs active: {foreign}' if foreign else None)
            todo, why, finished = plan(est, args.target_positions, busy, c.n, c.paused, c.backoff, time.time(),
                                       blocked=blocked, committed_hours=c.committed_hours(busy), cap_hours=args.max_instance_hours,
                                       minutes=args.minutes, launches=c.launches + busy, max_launches=args.max_launches)
            if finished:
                c.record('complete', shards=shards, est_positions=est, used_hours=round(c.used_hours, 3))
                c.say(f'complete: generation {c.gen} holds {shards} shards, about {est / 1e6:.1f}M positions. Audit it before '
                      f'training on it (ci/datagen-full-audit.sh).', alert=True)
                return 0
            if est >= args.target_positions and not c.target_reached:
                c.target_reached = True
                c.record('target-reached', shards=shards, est_positions=est)
                if mine and not args.let_finish and not args.dry_run:
                    c.cancel_running(mine)
            minutes = size_minutes(args.target_positions - est, c.rate(), todo, args.minutes) if todo else 0
            for _ in range(todo):
                if args.dry_run:
                    why = f'would dispatch {todo} for {minutes} min (dry run)'
                    break
                c.dispatch(shards, minutes)
                busy += 1
            show(c, shards, est, args.target_positions, busy, why)
            if (stop or c.main_moved) and not busy:
                c.say('stopped; nothing of this campaign is running')
                return 0
            if not busy and not todo and (c.paused or why.startswith(('budget', 'launch cap'))):
                c.say(f'nothing can run: {why}', alert=True)
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
