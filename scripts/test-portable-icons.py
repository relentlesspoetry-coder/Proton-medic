#!/usr/bin/env python3
"""Decode native dialog icons with only the AppImage's GTK libraries and data.

No display or system GTK is needed. Missing shared-mime-info makes GTK's own
PNG resource fail to decode, aborting native dialogs even when WebKit renders.
"""
import ctypes
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def check(appdir):
    gtk = ctypes.CDLL(str(appdir / 'usr/lib/libgtk-3.so.0'))
    gtk.gtk_init_check(None, None)  # Registers GTK resources; no display required.
    pixbuf = ctypes.CDLL(str(appdir / 'usr/lib/libgdk_pixbuf-2.0.so.0'))
    gobject = ctypes.CDLL(str(appdir / 'usr/lib/libgobject-2.0.so.0'))
    gobject.g_object_unref.argtypes = [ctypes.c_void_p]

    class GError(ctypes.Structure):
        _fields_ = [('domain', ctypes.c_uint), ('code', ctypes.c_int),
                    ('message', ctypes.c_char_p)]

    def decode(function, source):
        function.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.POINTER(GError))]
        function.restype = ctypes.c_void_p
        error = ctypes.POINTER(GError)()
        image = function(os.fsencode(source), ctypes.byref(error))
        if not image:
            message = error.contents.message.decode() if error else 'Unknown decoder error'
            raise RuntimeError(f'{source}: {message}')
        gobject.g_object_unref(image)

    # Exercise the exact resource that previously aborted ensure_surface_for_gicon.
    decode(pixbuf.gdk_pixbuf_new_from_resource,
           '/org/gtk/libgtk/icons/48x48/status/image-missing.png')
    assert (appdir / 'usr/share/icons/hicolor/index.theme').is_file()
    theme = appdir / 'usr/share/icons/Adwaita'
    assert (theme / 'index.theme').is_file()
    warnings = list(theme.rglob('dialog-warning-symbolic.svg'))
    assert warnings, 'Bundled warning icon is missing'
    decode(pixbuf.gdk_pixbuf_new_from_file, warnings[0])
    print('PASS: bundled GTK fallback PNG and warning SVG decode without host data')


def main():
    if sys.argv[1] == '--appdir':
        check(Path(sys.argv[2]).resolve())
        return
    image = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix='pm-icon-smoke-') as tmp:
        subprocess.run([str(image), '--appimage-extract'], cwd=tmp,
                       stdout=subprocess.DEVNULL, check=True, timeout=120)
        appdir = Path(tmp) / 'squashfs-root'
        env = dict(os.environ, LD_LIBRARY_PATH=str(appdir / 'usr/lib'),
                   XDG_DATA_DIRS=str(appdir / 'usr/share'),
                   XDG_DATA_HOME=str(Path(tmp) / 'empty-data'),
                   GDK_PIXBUF_MODULE_FILE=str(appdir / 'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache'))
        env.pop('DISPLAY', None)
        env.pop('WAYLAND_DISPLAY', None)
        subprocess.run([sys.executable, str(Path(__file__).resolve()), '--appdir', str(appdir)],
                       env=env, check=True, timeout=30)


if __name__ == '__main__':
    main()
