"""Build the stable updater manifest from version-bound signed Linux packages."""
import argparse
import base64
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote


def manifest(version, tag, package, signature, repository, appimage=None, appimage_signature=None):
    if not re.fullmatch(r'\d+\.\d+\.\d+', version) or tag != f'v{version}':
        raise ValueError('Release tag must match the stable application version')
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repository):
        raise ValueError('Invalid GitHub repository')
    expected = f'proton-medic_{version}_amd64.deb'
    if package != expected:
        raise ValueError(f'Expected package name: {expected}')
    decoded = base64.b64decode(signature.strip(), validate=True).decode('utf-8')
    # Actual cryptographic verification happens in Tauri. This catches publishing
    # an old signature alongside a new manifest before a release is uploaded.
    if not any(line.startswith('trusted comment:') and
               re.search(r'(?:^|\s)version:' + re.escape(version) + r'(?:\s|$)', line)
               for line in decoded.splitlines()):
        raise ValueError('Signature must be bound to the release version')
    result = {
        'version': version,
        'notes': f'Proton Medic {version}. See the GitHub release for changes.',
        'pub_date': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
        'platforms': {'linux-x86_64-deb': {
            'signature': signature.strip(),
            'url': f'https://github.com/{repository}/releases/download/{tag}/{quote(package)}'
        }}
    }
    if appimage is not None:
        expected_image = f'proton-medic_{version}_amd64.AppImage'
        if appimage != expected_image:
            raise ValueError(f'Expected AppImage name: {expected_image}')
        # Reuse the same version-binding validation as the DEB signature.
        validated = manifest(version, tag, expected, appimage_signature, repository)
        platform = validated['platforms']['linux-x86_64-deb']
        platform['url'] = f'https://github.com/{repository}/releases/download/{tag}/{quote(appimage)}'
        result['platforms']['linux-x86_64-appimage'] = platform
    elif appimage_signature is not None:
        raise ValueError('AppImage signature provided without an AppImage')
    return result



if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--tag', required=True)
    parser.add_argument('--repository', default='relentlesspoetry-coder/proton-medic')
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--appimage', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    version = json.loads(Path('package.json').read_text())['version']
    if not args.package.is_file():
        raise SystemExit('Package does not exist')
    signature = Path(str(args.package) + '.sig').read_text()
    if args.appimage and not args.appimage.is_file():
        raise SystemExit('AppImage does not exist')
    result = manifest(version, args.tag, args.package.name, signature, args.repository,
                      args.appimage.name if args.appimage else None,
                      Path(str(args.appimage) + '.sig').read_text() if args.appimage else None)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
