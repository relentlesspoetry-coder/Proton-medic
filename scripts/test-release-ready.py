#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import unittest

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

if __name__ == '__main__':
    unittest.main(verbosity=2)
