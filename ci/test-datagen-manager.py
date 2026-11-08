#!/usr/bin/env python3
"""Unit tests for ci/datagen-manager.py's decisions. No AWS or GitHub: python3 ci/test-datagen-manager.py"""

import importlib.util
import math
import os
import unittest

spec = importlib.util.spec_from_file_location('manager', os.path.join(os.path.dirname(__file__), 'datagen-manager.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


def plan(est, target, busy, n=3, **kw):
    opts = dict(paused=None, backoff_until=0.0, now=1000.0, blocked=None, committed_hours=0, cap_hours=math.inf,
                minutes=280, launches=0, max_launches=math.inf)
    opts.update(kw)
    return m.plan(est, target, busy, n, opts.pop('paused'), opts.pop('backoff_until'), opts.pop('now'), **opts)


class Titles(unittest.TestCase):
    def test_a_manager_dispatch(self):
        t = m.parse_title('Datagen: 1 × 280 min, native — datagen-manager gen 6')
        self.assertEqual(t, {'instances': 1, 'minutes': 280, 'target_cpu': 'native', 'note': 'datagen-manager gen 6',
                             'benchmark': False})

    def test_a_manual_run_and_a_benchmark(self):
        t = m.parse_title('Datagen: 3 × 150 min, native — gen 5: net6k4 (v0.42.0), ~500M positions for a net on gens 0-5')
        self.assertEqual((t['instances'], t['minutes'], t['benchmark']), (3, 150, False))
        self.assertTrue(m.parse_title('Datagen benchmark: 1 × 10 min, x86-64-v3')['benchmark'])

    def test_not_a_datagen_title(self):
        self.assertIsNone(m.parse_title('Relabel: gen 3,4,5, 3 × 280 min, 5000 nodes'))


class Classify(unittest.TestCase):
    def test_finished(self):
        log = '== datagen: run x task t0, generation 6, 64 workers, 280 minutes, seed 1\ndatagen-task: 1412 shards, 34291011 positions in 17011s\n'
        self.assertEqual(m.classify('success', log), {'outcome': 'finished', 'launched': True, 'shards': 1412, 'positions': 34291011})

    def test_reclaimed(self):
        log = ('== datagen: run x task t0, generation 6, 64 workers, 280 minutes, seed 1\n'
               'datagen-task: interrupted; 388 complete shards are in S3\n'
               'warning: interrupted by spot reclamation at 2026-11-09T03:00:00Z: no result; re-run the task\n')
        self.assertEqual(m.classify('failure', log), {'outcome': 'interrupted', 'launched': True, 'shards': 388, 'positions': 0})

    def test_no_capacity_launches_nothing(self):
        log = 'warning: no spot c8a.16xlarge capacity in us-east-1b; trying the next zone\nerror: no spot c8a.16xlarge capacity in us-east-1a, us-east-1b; re-run later\n'
        self.assertEqual(m.classify('failure', log)['outcome'], 'no-capacity')

    def test_a_task_failure(self):
        self.assertEqual(m.classify('failure', '== datagen: run x\ndatagen exited 1\n')['outcome'], 'failed')

    def test_cancelled(self):
        self.assertEqual(m.classify('cancelled', '')['outcome'], 'cancelled')


class Plan(unittest.TestCase):
    def test_idle_instances_are_dispatched(self):
        self.assertEqual(plan(0, 600_000_000, 0)[0], 3)
        self.assertEqual(plan(0, 600_000_000, 2)[0], 1)
        self.assertEqual(plan(0, 600_000_000, 3)[0], 0)

    def test_done_only_when_the_target_is_in_s3_and_nothing_runs(self):
        self.assertEqual(plan(600_000_000, 600_000_000, 0)[2], True)
        dispatch, why, done = plan(650_000_000, 600_000_000, 2)
        self.assertEqual((dispatch, done), (0, False))
        self.assertIn('target reached', why)

    def test_paused_blocked_and_backoff_wait(self):
        self.assertEqual(plan(0, 1, 0, paused='x')[0], 0)
        self.assertIn('main moved', plan(0, 1, 0, blocked='main moved')[1])
        self.assertIn('no capacity', plan(0, 1, 0, backoff_until=1300.0)[1])

    def test_the_budget_counts_each_launch_at_its_hard_limit(self):
        # 10 h used, 17 h cap, 6 h a launch: one fits, the second does not.
        dispatch, why, _ = plan(0, 1, 0, committed_hours=10, cap_hours=17)
        self.assertEqual(dispatch, 1)
        self.assertTrue(why.startswith('budget'))

    def test_the_launch_cap(self):
        dispatch, why, _ = plan(0, 1, 0, launches=29, max_launches=30)
        self.assertEqual((dispatch, why), (1, 'launch cap (30) reached'))

    def test_estimate(self):
        self.assertEqual(m.estimate_positions(24_000, 24_300), 583_200_000)


class AfterOutcome(unittest.TestCase):
    def res(self, outcome, shards=100):
        return {'outcome': outcome, 'launched': True, 'shards': shards, 'positions': 0}

    def test_reclaimed_and_finished_relaunch(self):
        self.assertEqual(m.after_outcome(self.res('interrupted'), 0), (None, 0, 0))
        self.assertEqual(m.after_outcome(self.res('finished'), 2), (None, 0, 0))

    def test_no_capacity_waits_ten_minutes(self):
        self.assertEqual(m.after_outcome(self.res('no-capacity', 0), 0), (None, 600, 0))

    def test_failures_and_cancels_pause(self):
        self.assertIsNotNone(m.after_outcome(self.res('failed'), 0)[0])
        self.assertIsNotNone(m.after_outcome(self.res('cancelled'), 0)[0])

    def test_three_empty_launches_pause(self):
        self.assertEqual(m.after_outcome(self.res('finished', 0), 1), (None, 0, 2))
        self.assertIsNotNone(m.after_outcome(self.res('interrupted', 0), 2)[0])


if __name__ == '__main__':
    unittest.main(verbosity=1)
