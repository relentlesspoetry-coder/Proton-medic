#!/usr/bin/env python3
"""Select a validated release version without moving existing tags."""
import json
import os
from pathlib import Path
import re
import tomllib
import urllib.error
import urllib.request


def version_tuple(tag):
    match = re.fullmatch(r'v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)', tag)
    if not match:
        raise ValueError(f'Expected a stable vX.Y.Z tag: {tag}')
    return tuple(map(int, match.groups()))


def should_publish(tag, existing, latest, automatic, target_sha, main_sha):
    version = version_tuple(tag)
    if existing:
        if existing.get('draft') or existing.get('prerelease'):
            raise ValueError('An existing draft or prerelease must be resolved before publishing this version')
        return False, 'Version already published'
    if latest and version <= version_tuple(latest):
        return False, 'Version is not newer than the latest stable release'
    if automatic and target_sha != main_sha:
        return False, 'A newer source commit is awaiting validation'
    return True, 'New validated version ready for publication'


def github_get(path):
    request = urllib.request.Request('https://api.github.com/' + path, headers={
        'Authorization': 'Bearer ' + os.environ['GH_TOKEN'],
        'Accept': 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
    })
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise


def main():
    version = json.loads(Path('package.json').read_text())['version']
    tag = os.environ.get('REQUESTED_TAG') or 'v' + version
    version_tuple(tag)
    if tag != 'v' + version:
        raise ValueError('Release tag does not match the application version')
    if json.loads(Path('src-tauri/tauri.conf.json').read_text())['version'] != version:
        raise ValueError('Tauri version does not match')
    if tomllib.loads(Path('src-tauri/Cargo.toml').read_text())['package']['version'] != version:
        raise ValueError('Cargo version does not match')
    if not Path(f'RELEASE-{tag}.md').is_file():
        raise ValueError('Release notes are missing')
    target_sha = os.environ['TARGET_SHA']
    if not re.fullmatch(r'[0-9a-f]{40}', target_sha):
        raise ValueError('Invalid source commit')
    repository = os.environ['RELEASE_REPOSITORY']
    automatic = os.environ.get('RELEASE_AUTOMATIC') == 'true'
    existing = github_get(f'repos/{repository}/releases/tags/{tag}')
    latest = github_get(f'repos/{repository}/releases/latest')
    main_commit = github_get(f'repos/{repository}/commits/main') if automatic else None
    publish, reason = should_publish(tag, existing, latest['tag_name'] if latest else None,
                                     automatic, target_sha, main_commit['sha'] if main_commit else target_sha)
    print(f'{tag}: {reason}')
    with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
        output.write(f'tag={tag}\nsha={target_sha}\npublish={str(publish).lower()}\n')


if __name__ == '__main__':
    main()
