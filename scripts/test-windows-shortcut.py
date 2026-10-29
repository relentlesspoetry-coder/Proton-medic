#!/usr/bin/env python3
"""Run the packaged native launch path with a disposable profile and fake Wine."""
import fcntl
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

parser = argparse.ArgumentParser()
parser.add_argument('binary', type=Path)
args = parser.parse_args()
BINARY = args.binary.resolve()

class WindowsShortcutTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='pm-native-shortcut-')
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        self.root = self.home / 'data/proton-medic/windows-games'
        self.profile = self.root / '123-456'
        self.prefix = self.profile / 'compatdata/pfx'
        (self.prefix / 'drive_c').mkdir(parents=True)
        self.exe = self.home / 'Games/Black Myth Wukong/b1.exe'
        self.exe.parent.mkdir(parents=True)
        pe = bytearray(256)
        pe[:2] = b'MZ'
        pe[60:64] = (128).to_bytes(4, 'little')
        pe[128:134] = b'PE\0\0\x64\x86'
        self.exe.write_bytes(pe)
        self.receipt = self.home / 'invocation.json'
        runtime = self.home / 'bin/wine'
        runtime.parent.mkdir()
        runtime.write_text(
            '#!/usr/bin/python3\nimport json,os,sys,time\nfrom pathlib import Path\n'
            f"Path({str(self.receipt)!r}).write_text(json.dumps(dict(args=sys.argv[1:], prefix=os.environ.get('WINEPREFIX'), cwd=os.getcwd())))\n"
            f"Path({str(self.home / 'runtime-environment.json')!r}).write_text(json.dumps(dict(os.environ)))\n"
            'time.sleep(0.1)\n')
        runtime.chmod(0o700)
        self.record = dict(id='123-456', name='Fixture Wukong', runtimeId='wine:' + str(runtime),
                           installer='', gameExe=str(self.exe), state='ready-to-test', exitCode=None,
                           message='Fixture', runId=None, bootId=None)
        self.save()
        self.env = dict(os.environ, HOME=str(self.home), XDG_DATA_HOME=str(self.home/'data'),
                        XDG_CONFIG_HOME=str(self.home/'config'), XDG_CACHE_HOME=str(self.home/'cache'),
                        PATH=str(runtime.parent) + ':/usr/bin:/bin')
        self.env.pop('DISPLAY', None)
        self.env.pop('WAYLAND_DISPLAY', None)

    def save(self):
        (self.profile / 'profile.json').write_text(json.dumps(self.record))

    def launch(self):
        return subprocess.run([str(BINARY), '--launch-windows-game', '123-456', '--profile-root', str(self.root)],
                              env=self.env, text=True, capture_output=True, timeout=60)

    def start_held_runtime(self):
        runtime = self.home/'bin/wine'
        runtime.write_text('#!/usr/bin/python3\nimport time\nfrom pathlib import Path\n'
                           f'Path({str(self.receipt)!r}).write_text("started")\n'
                           f'while not Path({str(self.home / "release-runtime")!r}).exists(): time.sleep(0.05)\n')
        process = subprocess.Popen([str(BINARY), '--launch-windows-game', '123-456', '--profile-root', str(self.root)],
                                   env=self.env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        def finish():
            (self.home/'release-runtime').touch()
            process.communicate(timeout=30)
        self.addCleanup(finish)
        deadline = time.monotonic() + 30
        while not self.receipt.exists():
            if process.poll() is not None:
                self.fail(process.communicate()[1])
            if time.monotonic() >= deadline:
                self.fail('Fixture runtime did not start')
            time.sleep(0.05)
        return process

    def test_detached_child_keeps_profile_prefix_and_update_locked(self):
        runtime = self.home/'bin/wine'
        child = self.home/'detached.py'
        release = self.home/'release-detached'
        child.write_text('import time\nfrom pathlib import Path\n'
                         f'Path({str(self.receipt)!r}).write_text("started")\n'
                         f'while not Path({str(release)!r}).exists(): time.sleep(0.02)\n')
        runtime.write_text('#!/usr/bin/python3\nimport subprocess\n'
                          f'subprocess.Popen(["/usr/bin/python3", {str(child)!r}], start_new_session=True, '
                          'stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)\n')
        process = subprocess.Popen([str(BINARY), '--launch-windows-game', '123-456', '--profile-root', str(self.root)],
                                   env=self.env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 30
            while not self.receipt.exists() and process.poll() is None and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertTrue(self.receipt.exists(), 'Detached runtime did not start')
            time.sleep(0.7)
            self.assertIsNone(process.poll(), 'Supervisor exited while detached game was active')
            self.assertEqual(json.loads((self.profile/'profile.json').read_text())['state'], 'game-running')
            for path in [self.prefix/'.proton-medic.lock', self.home/'data/proton-medic/locks/activity.lock']:
                with path.open('r+') as lock:
                    with self.assertRaises(BlockingIOError):
                        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        finally:
            release.touch()
            output, error = process.communicate(timeout=30)
        self.assertEqual(process.returncode, 0, error)
        self.assertEqual(json.loads((self.profile/'profile.json').read_text())['state'], 'game-exited')

    def test_two_profiles_cannot_launch_the_same_prefix_concurrently(self):
        first = self.start_held_runtime()
        second = self.root/'456-789'
        second.mkdir()
        record = dict(self.record, id='456-789', prefixPath=str(self.prefix))
        (second/'profile.json').write_text(json.dumps(record))
        result = subprocess.run([str(BINARY), '--launch-windows-game', '456-789', '--profile-root', str(self.root)],
                                env=self.env, text=True, capture_output=True, timeout=30)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('prefix is in use', result.stderr)
        self.assertEqual(json.loads((second/'profile.json').read_text())['state'], 'ready-to-test')
        self.assertIsNone(first.poll())
        (self.home/'release-runtime').touch()
        first.communicate(timeout=30)
        retry = subprocess.run([str(BINARY), '--launch-windows-game', '456-789', '--profile-root', str(self.root)],
                               env=self.env, text=True, capture_output=True, timeout=30)
        self.assertEqual(retry.returncode, 0, retry.stderr)

    def test_shortcut_blocks_updates_until_its_runtime_finishes(self):
        process = self.start_held_runtime()
        with (self.home/'data/proton-medic/locks/activity.lock').open('r+') as lock:
            with self.assertRaises(BlockingIOError):
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            (self.home/'release-runtime').touch()
            process.communicate(timeout=30)
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)

    def test_active_update_blocks_shortcut_launch_without_changing_profile(self):
        locks = self.home/'data/proton-medic/locks'
        locks.mkdir(parents=True)
        before = (self.profile/'profile.json').read_bytes()
        with (locks/'activity.lock').open('w+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            result = self.launch()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('app update is active', result.stderr)
            self.assertFalse(self.receipt.exists())
            self.assertEqual((self.profile/'profile.json').read_bytes(), before)
        self.assertEqual(self.launch().returncode, 0)

    def test_external_executable_keeps_prefix_without_a_desktop_session(self):
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt, dict(args=[str(self.exe)], prefix=str(self.prefix), cwd=str(self.exe.parent)))
        saved = json.loads((self.profile/'profile.json').read_text())
        self.assertEqual(saved['state'], 'game-exited')
        self.assertEqual(saved['exitCode'], 0)
        self.assertGreater(saved['lastLaunchedAt'], 0)
        self.assertLessEqual(saved['lastLaunchedAt'], int(time.time()*1000))
        self.assertTrue((self.profile/'logs'/saved['runId']/'stderr.log').is_file())
        history = json.loads((self.profile/'logs'/saved['runId']/'history.json').read_text())
        self.assertEqual(history['id'], saved['runId'])
        self.assertEqual(history['kind'], 'game')
        self.assertEqual(history['state'], 'game-exited')
        self.assertEqual(history['exitCode'], 0)
        self.assertGreaterEqual(history['endedAt'], history['startedAt'])
        self.assertTrue(history['runtime'])
        self.assertFalse((self.profile/'logs'/saved['runId']/'monitor-sample.json').exists())

    def test_failed_runtime_start_keeps_the_previous_recent_game_date(self):
        (self.home/'bin/wine').write_text('#!/missing-proton-medic-fixture-interpreter\n')
        self.record['lastLaunchedAt'] = 1700000000000
        self.save()
        result = self.launch()
        self.assertNotEqual(result.returncode, 0)
        saved = json.loads((self.profile/'profile.json').read_text())
        self.assertEqual(saved['state'], 'start-failed')
        self.assertEqual(saved['lastLaunchedAt'], 1700000000000)
        self.assertFalse(self.receipt.exists())

    def test_opt_in_monitor_includes_shortcut_child_processes(self):
        runtime = self.home / 'bin/wine'
        runtime.write_text('#!/usr/bin/python3\nimport subprocess,sys\nchild=subprocess.Popen([sys.executable,"-c","import time; data=bytearray(16*1024*1024); time.sleep(5)"])\nchild.wait()\n')
        process = subprocess.Popen([str(BINARY), '--launch-windows-game', '123-456', '--profile-root', str(self.root)], env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.addCleanup(lambda: process.wait(timeout=15))
        deadline = time.monotonic() + 10
        sample = None
        while time.monotonic() < deadline and process.poll() is None:
            saved = json.loads((self.profile/'profile.json').read_text())
            if saved.get('runId'):
                logs = self.profile/'logs'/saved['runId']
                temp = logs/'request-fixture.tmp'
                temp.write_text(json.dumps(dict(expiresAt=int(time.time()*1000)+6000)))
                temp.replace(logs/'monitor-request.json')
                try:
                    sample = json.loads((logs/'monitor-sample.json').read_text())
                except FileNotFoundError:
                    pass
                if sample and sample['processCount'] >= 2 and sample['cpuPercent'] is not None:
                    break
            time.sleep(0.1)
        self.assertIsNotNone(sample)
        self.assertGreaterEqual(sample['processCount'], 2)
        self.assertGreater(sample['memoryBytes'], 8*1024*1024)
        self.assertGreaterEqual(sample['peakMemoryBytes'], sample['memoryBytes'])
        self.assertIsNotNone(sample['cpuPercent'])
        self.assertGreaterEqual(sample['cpuPercent'], 0)
        self.assertFalse(sample['partial'])
        self.assertEqual(sample['runId'], saved['runId'])
        self.assertEqual(process.wait(timeout=15), 0)

    def test_saved_arguments_and_working_directory_reach_the_native_launcher_literally(self):
        working = self.home / 'Game Working Directory'
        working.mkdir()
        marker = self.home / 'must-not-exist'
        arguments = ['-dx12', 'two words', f'$(touch {marker})', f'; touch {marker}']
        self.record.update(launchArguments=arguments, workingDirectory=str(working))
        self.save()
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(self.receipt.read_text()), dict(args=[str(self.exe), *arguments], prefix=str(self.prefix), cwd=str(working)))
        self.assertFalse(marker.exists())

    def test_missing_working_directory_preserves_the_profile_without_starting_runtime(self):
        self.record['workingDirectory'] = str(self.home / 'missing-directory')
        self.save()
        before = (self.profile / 'profile.json').read_bytes()
        result = self.launch()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.receipt.exists())
        self.assertEqual((self.profile / 'profile.json').read_bytes(), before)

    def test_native_version_probe_needs_no_desktop_session(self):
        result = subprocess.run([str(BINARY), '--proton-medic-version'], env=self.env,
                                text=True, capture_output=True, timeout=60)
        expected = json.loads((Path(__file__).resolve().parents[1]/'package.json').read_text())['version']
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, expected + '\n')
        self.assertFalse(self.receipt.exists())

    def test_game_does_not_inherit_portable_mount_environment(self):
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        env = json.loads((self.home / 'runtime-environment.json').read_text())
        self.assertNotIn('APPDIR', env)
        self.assertNotIn('APPIMAGE', env)
        for key in ['LD_LIBRARY_PATH', 'PYTHONHOME', 'PYTHONPATH', 'GTK_PATH', 'GIO_MODULE_DIR']:
            self.assertNotIn('/tmp/appimage_', env.get(key, ''))
            self.assertNotIn('/tmp/.mount_', env.get(key, ''))

    def test_missing_executable_fails_without_starting_runtime(self):
        self.record['gameExe'] = str(self.home/'missing.exe')
        self.save()
        result = self.launch()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.receipt.exists())

if __name__ == '__main__':
    unittest.main(argv=['test-windows-shortcut'], verbosity=2)
