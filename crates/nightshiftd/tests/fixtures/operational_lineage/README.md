# Accepted FIELD vectors

These fixed bytes were generated once through the exact accepted owner crates:

- Monitor result: b2d52fe34f146774cbf5601819982c267c7fb082
- NQ result: 39b9f84f2f70955dd12e5cbfe798c740f9e52854

field-monitor.accepted.json was produced by pulse-runtime CanarySigningIdentityV1 and
sign_operational_acquisition. field-nq.accepted.json was produced by nq-core
qualify_operational_observations over those exact Monitor bytes and payload bytes.

Exact byte digests:

- Monitor: sha256:9908a346475a228c75c48a30d947e3a15ad86f7c11079295e4e03e4e6df70345
- NQ: sha256:4e5958ccce4013e3d28531b32940630f7c7962c2690bd7a7493ca7f1981dc378

The refused Monitor vectors are fixed mutations of the accepted vector for one
closed unknown locator kind, 33 locators, 33 attachments, and a 513-byte subject
namespace. They are input compatibility fixtures only; they carry no authority
or operational result.

The generator used ephemeral key material in a temporary directory. The
generator, temporary directory, and private key were removed after these public
signed bytes and NQ qualification bytes were fixed. No owner repository was
modified.
