#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import Mock, patch
import sys
import urllib.error
sys.path.insert(0, str(Path(__file__).parent))
import release_tag

spec = importlib.util.spec_from_file_location('release_ready', Path(__file__).with_name('release_ready.py'))
ready = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ready)

class ReleaseReadyTests(unittest.TestCase):
    def decision(self, tag='v0.6.8', existing=None, latest='v0.6.7', automatic=True, target='same', main='same'):
        return ready.should_publish(tag, existing, latest, automatic, target, main)[0]

    def test_new_successful_main_version_is_publishable(self):
        self.assertTrue(self.decision())

    def test_repeated_validation_does_not_republish(self):
        self.assertFalse(self.decision(existing={'draft': False, 'prerelease': False}))

    def test_older_or_equal_versions_cannot_replace_latest(self):
        self.assertFalse(self.decision(tag='v0.6.7'))
        self.assertFalse(self.decision(tag='v0.6.6'))
        self.assertTrue(self.decision(tag='v0.10.0', latest='v0.9.9'))

    def test_superseded_commit_cannot_publish_automatically(self):
        self.assertFalse(self.decision(target='older', main='newer'))
        self.assertTrue(self.decision(automatic=False, target='tagged', main='newer'))

    def test_malformed_and_prerelease_tags_are_rejected(self):
        for tag in ['main', 'v0.6.8-beta', 'v00.6.8', 'v0.6.8\nextra']:
            with self.assertRaises(ValueError):
                self.decision(tag=tag)

    def test_existing_draft_is_not_silently_overwritten(self):
        with self.assertRaises(ValueError):
            self.decision(existing={'draft': True, 'prerelease': False})

class ReleaseTagTests(unittest.TestCase):
    sha = 'a' * 40
    annotated = 'b' * 40

    def reference(self, sha=None, kind='commit'):
        return {'object': {'type': kind, 'sha': sha or self.sha}}

    def ensure(self, get, post):
        release_tag.ensure_tag('owner/repo', 'v0.6.44', self.sha, get, post)

    def test_missing_tag_is_created_and_verified(self):
        get, post = Mock(side_effect=[None, self.reference()]), Mock()
        self.ensure(get, post)
        post.assert_called_once_with('repos/owner/repo/git/refs', {'ref': 'refs/tags/v0.6.44', 'sha': self.sha})

    def test_retry_reuses_matching_lightweight_tag(self):
        get, post = Mock(return_value=self.reference()), Mock()
        self.ensure(get, post)
        post.assert_not_called()

    def test_matching_annotated_tag_is_peeled(self):
        get, post = Mock(side_effect=[self.reference(self.annotated, 'tag'), self.reference()]), Mock()
        self.ensure(get, post)
        get.assert_any_call('repos/owner/repo/git/tags/' + self.annotated)
        post.assert_not_called()

    def test_nested_annotated_tag_is_peeled(self):
        get = Mock(side_effect=[self.reference(self.annotated, 'tag'), self.reference('c' * 40, 'tag'), self.reference()])
        self.ensure(get, Mock())

    def test_mismatching_tags_are_never_moved(self):
        for rows in [[self.reference('c' * 40)], [self.reference(self.annotated, 'tag'), self.reference('c' * 40)]]:
            post = Mock()
            with self.assertRaisesRegex(ValueError, 'different commit'):
                self.ensure(Mock(side_effect=rows), post)
            post.assert_not_called()

    def test_creation_race_must_verify_the_winner(self):
        for sha in [self.sha, 'c' * 40]:
            post = Mock(side_effect=urllib.error.HTTPError('url', 422, 'exists', {}, None))
            get = Mock(side_effect=[None, self.reference(sha)])
            if sha == self.sha:
                self.ensure(get, post)
            else:
                with self.assertRaisesRegex(ValueError, 'different commit'):
                    self.ensure(get, post)
            self.assertEqual(post.call_count, 1)

    def test_missing_annotation_and_cycles_fail_closed(self):
        for rows in [[self.reference(self.annotated, 'tag'), None], [self.reference(self.annotated, 'tag'), self.reference(self.annotated, 'tag')]]:
            with self.assertRaises(ValueError):
                self.ensure(Mock(side_effect=rows), Mock())

    def test_read_network_errors_never_create_a_tag(self):
        for error in [urllib.error.URLError('offline'), urllib.error.HTTPError('url', 403, 'forbidden', {}, None)]:
            post = Mock()
            with self.assertRaises(type(error)):
                self.ensure(Mock(side_effect=error), post)
            post.assert_not_called()

    def test_only_http_404_means_missing(self):
        with patch.dict('os.environ', {'GH_TOKEN': 'test'}):
            for code in [404, 403, 429, 500]:
                with patch('urllib.request.urlopen', side_effect=urllib.error.HTTPError('url', code, 'failure', {}, None)):
                    if code == 404:
                        self.assertIsNone(ready.github_get('repos/owner/repo/git/ref/tags/v0.6.44'))
                    else:
                        with self.assertRaises(urllib.error.HTTPError):
                            ready.github_get('repos/owner/repo/git/ref/tags/v0.6.44')

    def test_failed_creation_is_not_retried_or_assumed_successful(self):
        for error in [urllib.error.URLError('timeout'), urllib.error.HTTPError('url', 500, 'failure', {}, None)]:
            get, post = Mock(return_value=None), Mock(side_effect=error)
            with self.assertRaises(type(error)):
                self.ensure(get, post)
            self.assertEqual(post.call_count, 1)
            self.assertEqual(get.call_count, 1)

if __name__ == '__main__':
    unittest.main(verbosity=2)
