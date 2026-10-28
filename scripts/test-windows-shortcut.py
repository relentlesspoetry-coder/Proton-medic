#!/usr/bin/env python3
"""Run the packaged native launch path with a disposable profile and fake Wine."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
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
                              env=self.env, text=True, capture_output=True, timeout=10)

    def test_external_executable_keeps_prefix_without_a_desktop_session(self):
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt, dict(args=[str(self.exe)], prefix=str(self.prefix), cwd=str(self.exe.parent)))
        saved = json.loads((self.profile/'profile.json').read_text())
        self.assertEqual(saved['state'], 'game-exited')
        self.assertEqual(saved['exitCode'], 0)
        self.assertTrue((self.profile/'logs'/saved['runId']/'stderr.log').is_file())

    def test_missing_executable_fails_without_starting_runtime(self):
        self.record['gameExe'] = str(self.home/'missing.exe')
        self.save()
        result = self.launch()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.receipt.exists())

if __name__ == '__main__':
    unittest.main(argv=['test-windows-shortcut'], verbosity=2)
