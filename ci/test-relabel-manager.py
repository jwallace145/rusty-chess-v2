#!/usr/bin/env python3
"""Unit tests for ci/relabel-manager.py's decisions. No AWS or GitHub: python3 ci/test-relabel-manager.py"""

import importlib.util
import math
import os
import unittest

spec = importlib.util.spec_from_file_location('manager', os.path.join(os.path.dirname(__file__), 'relabel-manager.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

RUN = 'gen=3/run=20261031-41180888933-1/task=t0'
SOURCE = [f'{RUN}/t0-w{w:03d}-s{s:05d}' for w in range(7) for s in range(4)]


def plan(left, busy=(), **kw):
    opts = dict(blocked=None, committed_hours=0, cap_hours=math.inf, minutes=280, launches=0, max_launches=math.inf)
    opts.update(kw)
    return m.plan(left, {s: 1 for s in busy}, kw.pop('paused', {}), kw.pop('backoff', {}), 1000.0,
                  **{k: v for k, v in opts.items() if k not in ('paused', 'backoff')})


class Slots(unittest.TestCase):
    def test_worker_strips_only_the_sequence(self):
        self.assertEqual(m.worker_of(f'{RUN}/t0-w017-s00005'), f'{RUN}/t0-w017')

    def test_every_nth_worker_of_the_sorted_list(self):
        slots = m.slot_of_workers(SOURCE, 3)
        self.assertEqual([slots[f'{RUN}/t0-w{w:03d}'] for w in range(7)], [0, 1, 2, 0, 1, 2, 0])

    def test_remaining_counts_shards_not_done(self):
        done = [s for s in SOURCE if '-w000-' in s or s.endswith('w001-s00000')]
        self.assertEqual(m.remaining_by_slot(SOURCE, done, 3), [8, 7, 8])
        self.assertEqual(m.remaining_by_slot(SOURCE, SOURCE, 3), [0, 0, 0])

    def test_the_split_ignores_what_is_done(self):
        # A top-up must give each worker the same slot, whatever is finished.
        self.assertEqual(m.slot_of_workers(SOURCE, 2), m.slot_of_workers(list(reversed(SOURCE)), 2))


class Titles(unittest.TestCase):
    def test_a_slot_dispatch(self):
        t = m.parse_title('Relabel: gen 3,4,5, 3 × 280 min, 5000 nodes, slots 2 — relabel-manager x slot 2')
        self.assertEqual(t, {'generations': '3,4,5', 'instances': 3, 'minutes': 280, 'nodes': 5000, 'slots': [2]})

    def test_an_old_full_run(self):
        t = m.parse_title('Relabel: gen 0,1,2, 2 × 220 min, 5000 nodes — top-up of 41788545675, ninth attempt')
        self.assertEqual((t['instances'], t['slots']), (2, None))

    def test_not_a_relabel_title(self):
        self.assertIsNone(m.parse_title('Train: net7rl012 512 hidden'))


class Classify(unittest.TestCase):
    def test_reclaimed(self):
        log = ('== relabel: x, generations 3, task 0 of 3\nrelabel-task: interrupted; 1112 complete shards are in S3\n'
               'warning: interrupted by spot reclamation at 2026-11-05T06:19:04Z: no result; re-run the task\n')
        self.assertEqual(m.classify('failure', log), {'outcome': 'interrupted', 'launched': True, 'shards': 1112})

    def test_no_capacity_launches_nothing(self):
        log = ('warning: no spot c8a.16xlarge capacity in us-east-1b (InsufficientInstanceCapacity); trying the next zone\n'
               'error: no spot c8a.16xlarge capacity in us-east-1a, us-east-1b; re-run later\n')
        self.assertEqual(m.classify('failure', log)['outcome'], 'no-capacity')

    def test_finished(self):
        log = '== relabel: x\nrelabel-task: 463 shards, 11499291 records in 731s\n'
        self.assertEqual(m.classify('success', log), {'outcome': 'finished', 'launched': True, 'shards': 463})

    def test_a_task_failure_is_not_retried_as_capacity(self):
        log = '== relabel: x\nrelabel-task: relabel exited 1 on the batch ending at worker 4\n'
        self.assertEqual(m.classify('failure', log)['outcome'], 'failed')

    def test_capacity_words_after_launch_are_a_failure(self):
        log = '== relabel: x\nsomething about no-capacity\n'
        self.assertEqual(m.classify('failure', log)['outcome'], 'failed')

    def test_colour_codes_are_stripped_from_logs(self):
        self.assertEqual(m.ANSI.sub('', '\x1b[36;1mwarning: interrupted by spot reclamation\x1b[0m'),
                         'warning: interrupted by spot reclamation')

    def test_cancelled(self):
        self.assertEqual(m.classify('cancelled', '')['outcome'], 'cancelled')


class Plan(unittest.TestCase):
    def test_idle_slots_with_work_are_dispatched(self):
        self.assertEqual(plan([5, 0, 7])[0], [0, 2])

    def test_complete_only_when_nothing_runs(self):
        self.assertTrue(plan([0, 0, 0])[2])
        self.assertFalse(plan([0, 0, 0], busy=[1])[2])

    def test_running_and_paused_slots_wait(self):
        todo, waits, _ = plan([5, 5, 5], busy=[0], paused={1: 'x'})
        self.assertEqual((todo, waits[0], waits[1]), ([2], 'running', 'paused: x'))

    def test_blocked_dispatches_nothing(self):
        todo, waits, _ = plan([5, 5, 5], blocked='main moved')
        self.assertEqual((todo, set(waits.values())), ([], {'main moved'}))

    def test_no_capacity_backs_off(self):
        todo, waits, _ = plan([5, 5], backoff={0: 1300.0})
        self.assertEqual((todo, waits[0]), ([1], 'no capacity; retry in 5 min'))

    def test_the_budget_counts_each_launch_at_its_hard_limit(self):
        # 280 minutes is a 6-hour hard limit: 10 h used leaves room for one more under 17.
        todo, waits, _ = plan([5, 5, 5], committed_hours=10, cap_hours=17)
        self.assertEqual(todo, [0])
        self.assertTrue(waits[1].startswith('budget'))

    def test_the_launch_cap(self):
        todo, waits, _ = plan([5, 5], launches=39, max_launches=40)
        self.assertEqual((todo, waits[1]), ([0], 'launch cap (40) reached'))

    def test_hard_hours_match_the_workflow(self):
        self.assertEqual([m.hard_hours(x) for x in (10, 60, 220, 280, 300)], [2, 2, 5, 6, 6])


class AfterOutcome(unittest.TestCase):
    def res(self, outcome, launched=True):
        return {'outcome': outcome, 'launched': launched, 'shards': 0}

    def test_reclaimed_with_progress_relaunches(self):
        self.assertEqual(m.after_outcome(self.res('interrupted'), 900, 400, 0), (None, 0, 0))

    def test_out_of_minutes_relaunches(self):
        self.assertEqual(m.after_outcome(self.res('finished'), 900, 100, 0), (None, 0, 0))

    def test_no_capacity_waits_ten_minutes(self):
        self.assertEqual(m.after_outcome(self.res('no-capacity', False), 900, 900, 0), (None, 600, 0))

    def test_a_failure_pauses(self):
        self.assertIsNotNone(m.after_outcome(self.res('failed'), 900, 900, 0)[0])

    def test_a_cancel_pauses(self):
        self.assertIsNotNone(m.after_outcome(self.res('cancelled'), 900, 800, 0)[0])

    def test_finishing_without_progress_pauses(self):
        self.assertIn('mismatch', m.after_outcome(self.res('finished'), 900, 900, 0)[0])

    def test_reclaims_without_progress_pause_on_the_third(self):
        self.assertEqual(m.after_outcome(self.res('interrupted'), 900, 900, 1), (None, 0, 2))
        self.assertIsNotNone(m.after_outcome(self.res('interrupted'), 900, 900, 2)[0])

    def test_the_last_shard_clears_the_slot(self):
        self.assertEqual(m.after_outcome(self.res('interrupted'), 10, 0, 2), (None, 0, 0))


if __name__ == '__main__':
    unittest.main(verbosity=1)
