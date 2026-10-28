#!/usr/bin/env python3
"""Dependency-free executable Steam fixture protocol checks.

IMPORTANT: These checks exercise throwaway Steam-like directories and actual
fake-launcher subprocesses. They do NOT execute Rust / Tauri code and are not
a replacement for `cargo test` or a native build.
"""
import os
import pathlib
import subprocess
import tempfile
import time
import unittest


class DisposableSteamFixture(unittest.TestCase):
    def setUp(self):
        self._temporary = tempfile.TemporaryDirectory(prefix='proton-medic-fixture-')
        self.root = pathlib.Path(self._temporary.name)
        self.home = self.root / 'home'
        self.steam = self.home / '.local' / 'share' / 'Steam'
        self.game = self.steam / 'steamapps' / 'common' / 'Fixture Adventure'
        self.game.mkdir(parents=True)
        (self.steam / 'steamapps' / 'appmanifest_98765.acf').write_text(
            '"AppState" { "appid" "98765" "name" "Fixture Adventure" '
            '"installdir" "Fixture Adventure" }', encoding='utf-8')
        (self.steam / 'steamapps' / 'libraryfolders.vdf').write_text(
            '"libraryfolders" { "0" { "path" "'+str(self.steam)+'" } }', encoding='utf-8')
        self.log = self.home / 'steam-98765.log'
        self.launcher = self.root / 'fake-steam'
        self.events = self.root / 'launch-events.txt'

    def tearDown(self):
        self._temporary.cleanup()

    def make_launcher(self, extra='exit 0'):
        # Fixed test-only shell file: the production Rust launcher uses explicit
        # argv and never permits a browser-supplied executable path.
        self.launcher.write_text(
            '#!/bin/sh\n'
            'printf "%s\\n" "$1" "$2" > "'+str(self.events)+'"\n'
            +extra+'\n', encoding='utf-8')
        self.launcher.chmod(0o700)

    def launch(self):
        return subprocess.run([str(self.launcher), '-applaunch', '98765'],
                              stdin=subprocess.DEVNULL, capture_output=True,
                              check=False, timeout=3, cwd=str(self.root))

    def test_manifest_is_disposable_and_not_real_steam(self):
        self.assertTrue((self.steam/'steamapps/appmanifest_98765.acf').is_file())
        self.assertTrue(self.game.is_dir())
        self.assertTrue(str(self.steam).startswith(str(self.root)))
        self.assertNotEqual(self.steam, pathlib.Path.home()/'.local/share/Steam')

    def test_launcher_receives_only_expected_game_arguments(self):
        self.make_launcher()
        result=self.launch()
        self.assertEqual(result.returncode,0)
        self.assertEqual(self.events.read_text(),'-applaunch\n98765\n')

    def test_launcher_exit_failure_is_observable(self):
        self.make_launcher('exit 7')
        self.assertEqual(self.launch().returncode,7)

    def test_fresh_proton_error_log_is_created(self):
        self.make_launcher('printf "vk_error_device_lost\\n" > "'+str(self.log)+'"')
        before=time.time()
        self.assertEqual(self.launch().returncode,0)
        self.assertGreaterEqual(self.log.stat().st_mtime, before-1)
        self.assertIn('vk_error_device_lost', self.log.read_text())

    def test_clean_log_is_not_a_playability_proof(self):
        self.make_launcher('printf "startup complete\\n" > "'+str(self.log)+'"')
        self.assertEqual(self.launch().returncode,0)
        self.assertNotIn('vk_error_device_lost', self.log.read_text())
        # A launcher exit of zero is deliberately not treated as game success.
        self.assertTrue(self.log.exists())

    def test_stale_log_is_untouched_by_launcher_without_logging(self):
        self.log.write_text('old failure\n', encoding='utf-8')
        timestamp=time.time()-3600
        os.utime(self.log,(timestamp,timestamp))
        self.make_launcher()
        self.assertEqual(self.launch().returncode,0)
        self.assertEqual(self.log.read_text(),'old failure\n')
        self.assertLess(self.log.stat().st_mtime,time.time()-3500)

    def test_fake_launch_cannot_modify_any_original_user_game(self):
        sentinel=self.root/'saves'/'save.dat'
        sentinel.parent.mkdir()
        sentinel.write_bytes(b'precious save data')
        self.make_launcher('printf "vk_error_device_lost\\n" > "'+str(self.log)+'"')
        self.assertEqual(self.launch().returncode,0)
        self.assertEqual(sentinel.read_bytes(),b'precious save data')


if __name__=='__main__':
    unittest.main(verbosity=2)
