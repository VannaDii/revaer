# Recorded media inputs

These are review inputs from the ongoing media integration at
`f1a8624f8c9c80325bfd14769bc1305c0fa5b793`, read on 2026-09-15. They preserve its
22-source lock, 30-fixture manifest and reviewed probe snapshots so the Python
foundation can test compatibility before that branch is rebased.

`f1-host-version.txt` and `f1-linux-version.txt` are the complete tool reports
already approved by that integration's ADR 578. Tests verify their exact hashes.
Changing a file here does not grant a new diagnostic exception.

No acquired media binary is committed. Unit coordination uses clearly synthetic
source bytes and process doubles. Native recipe checks download the existing
SHA-locked Big Buck Bunny sample, run FFmpeg and compare actual probe output.
The expanded acquisition/probe proof is recorded separately in ADR 592.
