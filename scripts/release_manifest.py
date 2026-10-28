"""Build the stable updater manifest from one version-bound signed amd64 DEB."""
import argparse
import base64
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote


def manifest(version, tag, package, signature, repository):
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
    return {
        'version': version,
        'notes': f'Proton Medic {version}. See the GitHub release for changes and verification notes.',
        'pub_date': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
        'platforms': {'linux-x86_64-deb': {
            'signature': signature.strip(),
            'url': f'https://github.com/{repository}/releases/download/{tag}/{quote(package)}'
        }}
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--tag', required=True)
    parser.add_argument('--repository', default='relentlesspoetry-coder/proton-medic')
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    version = json.loads(Path('package.json').read_text())['version']
    if not args.package.is_file():
        raise SystemExit('Package does not exist')
    signature = Path(str(args.package) + '.sig').read_text()
    result = manifest(version, args.tag, args.package.name, signature, args.repository)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
