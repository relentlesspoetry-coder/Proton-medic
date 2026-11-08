#!/usr/bin/env python3
"""Create or verify a release tag without moving an existing reference."""
import json
import os
import re
import urllib.error
import urllib.request
from release_ready import github_get, version_tuple


def github_post(path, value):
    request = urllib.request.Request('https://api.github.com/' + path,
        data=json.dumps(value).encode(), method='POST', headers={
            'Authorization': 'Bearer ' + os.environ['GH_TOKEN'],
            'Accept': 'application/vnd.github+json',
            'Content-Type': 'application/json',
            'X-GitHub-Api-Version': '2022-11-28',
        })
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def verify_target(repository, reference, target_sha, get):
    obj = reference.get('object', {})
    visited = set()
    for _ in range(16):
        sha = obj.get('sha', '')
        if not re.fullmatch(r'[0-9a-f]{40}', sha):
            raise ValueError('Release tag has an invalid target')
        if obj.get('type') == 'commit':
            if sha != target_sha:
                raise ValueError('Existing release tag points to a different commit; refusing to move it')
            return
        if obj.get('type') != 'tag' or sha in visited:
            raise ValueError('Release tag does not resolve to a commit')
        visited.add(sha)
        annotated = get(f'repos/{repository}/git/tags/{sha}')
        if not annotated:
            raise ValueError('Annotated release tag object is unavailable')
        obj = annotated.get('object', {})
    raise ValueError('Release tag nesting limit exceeded')


def ensure_tag(repository, tag, target_sha, get=github_get, post=github_post):
    version_tuple(tag)
    if not re.fullmatch(r'[0-9a-f]{40}', target_sha):
        raise ValueError('Invalid release commit')
    path = f'repos/{repository}/git/ref/tags/{tag}'
    reference = get(path)  # Only a confirmed HTTP 404 means absent.
    if reference is None:
        try:
            post(f'repos/{repository}/git/refs', {'ref': f'refs/tags/{tag}', 'sha': target_sha})
        except urllib.error.HTTPError as error:
            # Another run may have created the tag. Never assume a 422 is success.
            if error.code != 422:
                raise
        reference = get(path)
        if reference is None:
            raise ValueError('Release tag creation was not confirmed')
    verify_target(repository, reference, target_sha, get)


if __name__ == '__main__':
    ensure_tag(os.environ['RELEASE_REPOSITORY'], os.environ['RELEASE_TAG'], os.environ['RELEASE_SHA'])
    print('Release tag verified at the validated source commit')
