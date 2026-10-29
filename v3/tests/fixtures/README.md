# TLS test fixtures

These certificates and the private key are public test data.
They belong only to the local mock RouterOS server.
Do not use them on any real router or deployed service.

The test CA signs a leaf certificate for `router.test`.
The tests check a trusted name, a wrong name, and an untrusted certificate.
The CA private key was discarded after fixture creation.
The certificates have a ten-year validity period from 2026-10-29.
Replace the fixture pair before expiry.
