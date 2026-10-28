import base64
import unittest
from release_manifest import manifest


class ReleaseManifestTests(unittest.TestCase):
    def make(self, **overrides):
        values = dict(version='0.6.5', tag='v0.6.5', package='proton-medic_0.6.5_amd64.deb',
                      signature=base64.b64encode(b'trusted comment: timestamp:1 version:0.6.5\n').decode(),
                      repository='relentlesspoetry-coder/proton-medic')
        values.update(overrides)
        return manifest(**values)

    def test_uses_exact_deb_platform_and_tagged_asset(self):
        value = self.make()
        self.assertEqual(list(value['platforms']), ['linux-x86_64-deb'])
        self.assertTrue(value['platforms']['linux-x86_64-deb']['url'].endswith('/v0.6.5/proton-medic_0.6.5_amd64.deb'))

    def test_rejects_wrong_tag(self):
        with self.assertRaises(ValueError): self.make(tag='v0.6.4')

    def test_rejects_wrong_package_or_architecture(self):
        with self.assertRaises(ValueError): self.make(package='proton-medic_0.6.5_arm64.deb')

    def test_rejects_missing_or_wrong_signed_version(self):
        for comment in [b'trusted comment: old', b'trusted comment: version:0.6.4']:
            with self.assertRaises(ValueError): self.make(signature=base64.b64encode(comment).decode())

    def test_rejects_prerelease_version(self):
        with self.assertRaises(ValueError): self.make(version='0.6.5-beta.1', tag='v0.6.5-beta.1')


if __name__ == '__main__': unittest.main()
