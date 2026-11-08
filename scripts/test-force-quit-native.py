#!/usr/bin/env python3
"""Exercise the real GTK close dialog under dbus-run-session and Xvfb.

Uses a disposable HOME, synthetic PE, and TERM-resistant fake Wine only. Never
points at a real Steam library. Requires xdotool and Pillow (portable-smoke image).
No window manager is needed: send WM_DELETE_WINDOW, never XDestroyWindow.
"""
import argparse
import ctypes
import ctypes.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


def wait_for(check, message, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(0.05)
    raise AssertionError(message)


def identity(pid):
    try:
        fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
        return fields[0], int(fields[19])
    except FileNotFoundError:
        return None


def alive(pid, start):
    current = identity(pid)
    return current is not None and current[0] not in ('Z', 'X') and current[1] == start


class ClientData(ctypes.Union):
    _fields_ = [('b', ctypes.c_char * 20), ('s', ctypes.c_short * 10),
                ('l', ctypes.c_long * 5)]


class ClientMessage(ctypes.Structure):
    _fields_ = [('type', ctypes.c_int), ('serial', ctypes.c_ulong),
                ('send_event', ctypes.c_int), ('display', ctypes.c_void_p),
                ('window', ctypes.c_ulong), ('message_type', ctypes.c_ulong),
                ('format', ctypes.c_int), ('data', ClientData)]


class XEvent(ctypes.Union):
    _fields_ = [('client', ClientMessage), ('pad', ctypes.c_long * 24)]


class Desktop:
    def __init__(self):
        self.lib = ctypes.CDLL(ctypes.util.find_library('X11'))
        self.lib.XOpenDisplay.argtypes = [ctypes.c_char_p]
        self.lib.XOpenDisplay.restype = ctypes.c_void_p
        self.lib.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
        self.lib.XInternAtom.restype = ctypes.c_ulong
        self.lib.XSendEvent.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int,
                                       ctypes.c_long, ctypes.POINTER(XEvent)]
        self.lib.XFlush.argtypes = [ctypes.c_void_p]
        self.lib.XCloseDisplay.argtypes = [ctypes.c_void_p]
        self.display = self.lib.XOpenDisplay(None)
        if not self.display:
            raise RuntimeError('No X display; run under xvfb-run')

    def close(self, window):
        event = XEvent()
        event.client.type = 33  # ClientMessage
        event.client.display = self.display
        event.client.window = int(window)
        event.client.message_type = self.lib.XInternAtom(self.display, b'WM_PROTOCOLS', 0)
        event.client.format = 32
        event.client.data.l[0] = self.lib.XInternAtom(self.display, b'WM_DELETE_WINDOW', 0)
        if not self.lib.XSendEvent(self.display, int(window), 0, 0, ctypes.byref(event)):
            raise RuntimeError('Could not send WM_DELETE_WINDOW')
        self.lib.XFlush(self.display)

    def dispose(self):
        self.lib.XCloseDisplay(self.display)


def xdo(*args, check=True):
    return subprocess.run(['xdotool', *map(str, args)], check=check,
                          text=True, capture_output=True)


def windows(title):
    result = xdo('search', '--onlyvisible', '--name', title, check=False)
    if result.returncode not in (0, 1):
        raise RuntimeError(result.stderr)
    return result.stdout.split()


def capture_failure(output):
    """Best-effort diagnostics must never replace the original test failure."""
    from PIL import ImageGrab
    try:
        ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(output/'force-quit-failure.png')
    except Exception as error:
        print(f'Could not capture failure screenshot: {error}', file=sys.stderr)
    try:
        entries = []
        for window in windows('.*'):
            title = xdo('getwindowname', window, check=False)
            entries.append(f'{window}: {title.stdout.strip() or title.stderr.strip()}')
        listing = '\n'.join(entries) or '(no visible windows)'
        (output/'force-quit-windows.txt').write_text(listing+'\n')
        print(f'Visible windows at failure:\n{listing}', file=sys.stderr)
    except Exception as error:
        print(f'Could not list failure windows: {error}', file=sys.stderr)
    try:
        with (output/'force-quit.log').open('rb') as log:
            log.seek(0, os.SEEK_END)
            log.seek(max(0, log.tell()-16000))
            tail = log.read().decode('utf-8', errors='replace')
        print(f'Recent native log (last 16000 bytes):\n{tail}', file=sys.stderr, flush=True)
    except Exception as error:
        print(f'Could not read native log: {error}', file=sys.stderr)


DIALOG = '^Force quit Proton Medic\\?$'


def click_button(window, confirm):
    geometry = dict(line.split('=', 1) for line in
                    xdo('getwindowgeometry', '--shell', window).stdout.splitlines())
    # rfd's GTK OkCancelCustom adds OK then Cancel. The rightmost button
    # is Cancel whether GTK expands the footer or packs buttons at the end.
    x = int(geometry['WIDTH']) - 45
    y = int(geometry['HEIGHT']) - 20
    xdo('windowraise', window)
    xdo('windowfocus', '--sync', window)
    xdo('mousemove', '--window', window, x, y)
    if not confirm:
        xdo('click', '1')
        return
    # Focus Cancel by pressing it, then drag outside before releasing so it
    # does not activate. Navigate left to OK. This avoids guessing OK's width,
    # relying on an unspecified default button, or requiring an AT-SPI daemon.
    xdo('mousedown', '1')
    xdo('mousemove', '--window', window, 10, 10)
    xdo('mouseup', '1')
    xdo('key', 'Left', 'space')



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path, help='Directory for logs and dialog screenshots')
    args = parser.parse_args()
    if not shutil.which('xdotool'):
        parser.error('xdotool is required; use the portable-smoke Docker image')
    from PIL import ImageGrab
    binary = args.binary.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    desktop = Desktop()
    try:
        with tempfile.TemporaryDirectory(prefix='pm-force-quit-native-') as tmp:
            home = Path(tmp)
            root = home/'data/proton-medic/windows-games'
            profile = root/'123-456'
            (profile/'compatdata/pfx/drive_c').mkdir(parents=True)
            exe = home/'Games/Fixture/game.exe'
            exe.parent.mkdir(parents=True)
            pe = bytearray(256)
            pe[:2] = b'MZ'
            pe[60:64] = (128).to_bytes(4, 'little')
            pe[128:134] = b'PE\0\0\x64\x86'
            exe.write_bytes(pe)
            receipt, release = home/'runtime.json', home/'release-runtime'
            wine = home/'bin/wine'
            wine.parent.mkdir()
            wine.write_text('#!/usr/bin/python3\nimport json,os,signal,time\nfrom pathlib import Path\n'
                            'signal.signal(signal.SIGTERM,signal.SIG_IGN)\n'
                            'fields=Path(f"/proc/{os.getpid()}/stat").read_text().rsplit(")",1)[1].split()\n'
                            f'Path({str(receipt)!r}).write_text(json.dumps([os.getpid(),int(fields[19])]))\n'
                            'deadline=time.monotonic()+120\n'
                            f'while not Path({str(release)!r}).exists() and time.monotonic()<deadline: time.sleep(0.02)\n')
            wine.chmod(0o700)
            (profile/'profile.json').write_text(json.dumps(dict(
                id='123-456', name='Force Quit Fixture', runtimeId='wine:'+str(wine),
                installer='', gameExe=str(exe), state='ready-to-test', exitCode=None,
                message='Fixture', runId=None, bootId=None)))
            gtk = home/'config/gtk-3.0/settings.ini'
            gtk.parent.mkdir(parents=True)
            gtk.write_text('[Settings]\ngtk-dialogs-use-header=false\ngtk-font-name=Sans 10\n')
            env = dict(os.environ, HOME=str(home), XDG_DATA_HOME=str(home/'data'),
                       XDG_CONFIG_HOME=str(home/'config'), XDG_CACHE_HOME=str(home/'cache'),
                       GDK_BACKEND='x11', GTK_THEME='Adwaita', LANG='C.UTF-8',
                       LIBGL_ALWAYS_SOFTWARE='1', PATH=str(wine.parent)+':/usr/bin:/bin')
            # Do not permit a caller's custom library/config paths to discover real games.
            for key in ('STEAM_COMPAT_DATA_PATH', 'STEAM_COMPAT_CLIENT_INSTALL_PATH', 'WINEPREFIX'):
                env.pop(key, None)
            processes = []
            with (args.output/'force-quit.log').open('w') as log:
                try:
                    shortcut = subprocess.Popen([str(binary), '--launch-windows-game', '123-456',
                                                 '--profile-root', str(root)], env=env, stdout=log, stderr=log)
                    processes.append(shortcut)
                    def runtime_started():
                        if not receipt.exists():
                            return False
                        try:
                            return json.loads(receipt.read_text())
                        except json.JSONDecodeError:
                            return False
                    pid, start = wait_for(runtime_started, 'Fake Wine did not start')
                    def supported():
                        saved = json.loads((profile/'profile.json').read_text())
                        token = saved.get('runId')
                        marker = profile/'logs'/str(token)/'force-stop-supported'
                        return token and marker.exists() and marker.read_text() == token
                    wait_for(supported, 'Fixture supervisor does not support force stop')
                    unrelated = subprocess.Popen(['/usr/bin/sleep', '120'])
                    processes.append(unrelated)
                    gui = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
                    processes.append(gui)
                    def preserved():
                        assert gui.poll() is None, f'GUI exited without confirmation (status {gui.returncode})'
                        assert shortcut.poll() is None, f'Shortcut exited without confirmation (status {shortcut.returncode})'
                        assert alive(pid, start), 'Owned runtime stopped without confirmation'
                        assert unrelated.poll() is None, 'Unrelated process was stopped'
                    def visible_while_alive(title):
                        preserved()
                        return windows(title)
                    main_window = wait_for(lambda: visible_while_alive('^Proton Medic$'),
                                           'Main window did not appear')[0]
                    time.sleep(1)
                    def dialog():
                        preserved()
                        desktop.close(main_window)
                        found = wait_for(lambda: visible_while_alive(DIALOG), 'Force Quit dialog did not appear')
                        assert len(found) == 1, 'Multiple confirmation dialogs appeared'
                        assert found[0] != main_window, 'Dialog search matched the main GUI'
                        return found[0]
                    first = dialog()
                    for _ in range(3):
                        desktop.close(main_window)
                    time.sleep(0.5)
                    assert windows(DIALOG) == [first], 'Repeated close created another dialog'
                    preserved()
                    ImageGrab.grab(xdisplay=os.environ['DISPLAY']).save(args.output/'force-quit-dialog.png')
                    click_button(first, confirm=False)
                    wait_for(lambda: not windows(DIALOG), 'Keep Open did not dismiss dialog')
                    time.sleep(0.3)
                    preserved()
                    second = dialog()
                    desktop.close(second)
                    wait_for(lambda: not windows(DIALOG), 'Dialog titlebar dismissal failed')
                    time.sleep(0.3)
                    preserved()
                    third = dialog()
                    click_button(third, confirm=True)
                    wait_for(lambda: gui.poll() is not None, 'GUI did not exit after Force Quit', 20)
                    wait_for(lambda: shortcut.poll() is not None, 'Shortcut did not exit after Force Quit', 10)
                    assert not alive(pid, start), 'TERM-resistant owned runtime survived'
                    assert unrelated.poll() is None, 'Force Quit killed an unrelated process'
                    assert not windows('^Proton Medic$') and not windows(DIALOG), 'GUI windows survived'
                    saved = json.loads((profile/'profile.json').read_text())
                    assert not saved['state'].endswith('running'), 'Profile remained running'
                    print('PASS: Keep Open, dismissal, single dialog, forced runtime shutdown, unrelated process safety')
                except Exception:
                    log.flush()
                    capture_failure(args.output)
                    raise
                finally:
                    release.touch()
                    for process in reversed(processes):
                        if process.poll() is None:
                            process.terminate()
                        try:
                            process.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait(timeout=5)
    finally:
        desktop.dispose()


if __name__ == '__main__':
    main()
