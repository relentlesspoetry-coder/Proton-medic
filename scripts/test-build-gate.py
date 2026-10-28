#!/usr/bin/env python3
"""Read-only integration tests for the Zorin native build preflight.

These tests intentionally DO NOT invoke --build: they do not require cargo,
frontend npm dependencies, or modify real Steam installations.
"""
import pathlib
import subprocess
import unittest
import hashlib
import os
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = ROOT / 'scripts/build-zorin.sh'
GUARDED = [ROOT / 'src/App.tsx', ROOT / 'src/style.css',
           ROOT / 'src-tauri/src/repairs.rs', ROOT / 'src-tauri/src/auto_select.rs']

class BuildGateTests(unittest.TestCase):
    def digest(self):
        return [hashlib.sha256(p.read_bytes()).hexdigest() for p in GUARDED]

    def run_gate(self, mode):
        return subprocess.run(['bash', str(SCRIPT), mode], cwd=ROOT,
                              capture_output=True, text=True, timeout=15, check=False)

    def test_help_is_non_destructive(self):
        old = self.digest()
        proc = self.run_gate('--help')
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn('Zorin', proc.stdout)
        self.assertEqual(self.digest(), old)

    def test_preflight_reports_status_without_building(self):
        old = self.digest()
        proc = self.run_gate('--check')
        self.assertIn(proc.returncode, (0,2), proc.stderr)
        self.assertNotIn('[RUN] Tauri', proc.stdout)
        self.assertNotIn('[RUN] Full native', proc.stdout)
        self.assertEqual(self.digest(), old)

    def test_unrecognized_argument_is_rejected(self):
        proc = self.run_gate('--delete-games')
        self.assertEqual(proc.returncode, 64)

    def test_locked_tauri_rust_version_requirement(self):
        with tempfile.TemporaryDirectory(prefix='proton-medic-toolchain-') as tmp:
            rustc = pathlib.Path(tmp) / 'rustc'
            for version, expected in [('1.89.0', '[BLOCKED] Rust version'),
                                      ('1.90.0', '[PASS] Rust version suitable')]:
                rustc.write_text('#!/bin/sh\necho "rustc ' + version + ' (fixture)"\n')
                rustc.chmod(0o700)
                env = dict(os.environ, PATH=tmp + os.pathsep + os.environ['PATH'])
                proc = subprocess.run(['bash', str(SCRIPT), '--check'], cwd=ROOT,
                                      env=env, capture_output=True, text=True,
                                      timeout=15, check=False)
                self.assertIn(expected, proc.stdout)
                if version == '1.89.0':
                    self.assertEqual(proc.returncode, 2)

    def test_validation_wrapper_runs_new_gate(self):
        wrapper = (ROOT/'scripts/validate-native.sh').read_text()
        self.assertIn('/build-zorin.sh" --build', wrapper)
        self.assertNotIn('sudo', wrapper)

if __name__ == '__main__':
    unittest.main(verbosity=2)
