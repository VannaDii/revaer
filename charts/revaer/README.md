# Revaer Helm Chart

This chart deploys the Revaer application container and assumes PostgreSQL is
managed separately. Revaer requires a `DATABASE_URL`; provide it directly in
values for development or reference an existing Kubernetes Secret for real
deployments.

## Required Operator Inputs

There are no working default image or compliance bindings. Prepare an existing
PVC separately under the approved
[ADR 588 C1-D contract](../../docs/adr/support/588-decision-details.md#c1-d-exact-read-only-delivery-proposal).
The trusted operator installer must authenticate the exact platform image and
complete compliance predicate, validate the nine-file closure against that
image's original static files, and finish all writes before application
admission. This chart does not implement or run that installer.

Populate an operator-owned values file with verified inputs:

```yaml
image:
  digest: "" # Required: sha256: followed by exactly 64 lowercase hex characters.
  architecture: "" # Required: amd64 or arm64, matching that platform digest.
  tag: "" # Must remain empty; there is no appVersion fallback.
compliance:
  existingClaim: "" # Required: prepared PVC in the release namespace.
  manifestDigest: "" # Required: sha256: digest of the exact manifest bytes.
database:
  existingSecret: "" # Select the separately managed database Secret.
```

These empty placeholders deliberately fail validation. `image.digest` must be
the verified architecture-specific registry digest, not a mutable tag, local
image config ID or multi-platform index digest. Helm validates its syntax, not
its registry identity, signatures, provenance or architecture. Keep
`image.repository` aligned with the verified image; registry ports are supported.

The PVC must already contain the immutable directory
`<image-sha256-hex>/<manifest-sha256-hex>/`, without the `sha256:` prefixes.
The chart mounts exactly that subPath at `/app/compliance`, with `readOnly: true`
on both the PVC volume source and container mount. It does not create this PVC,
fetch artifacts, select a `latest` directory or provision a sidecar. C1-D requires
root-owned `0555` directories and `0444` files; templates do not verify those
permissions, bytes, storage immutability or the absence of other writers.

`kubernetes.io/arch` is enforced from `image.architecture`. A matching caller
selector is allowed; a conflicting one fails. Other node selectors, affinity
and tolerations are retained. Configure separate releases for amd64 and arm64.
The pod annotation `checksum/compliance-manifest` is reserved and equals the
complete `compliance.manifestDigest`. Any caller definition of that annotation,
including an empty or identical value, is rejected. Other pod annotations remain
available.

An image change selects a different immutable subPath; a manifest change also
changes the checksum annotation and pod template. Roll back only to a previously
jointly verified image, architecture and immutable set. Never rewrite an active
directory or reuse a bundle belonging to a different image.

## Render And Install

Local rendering through the repository command surface:

```bash
just --command helm template revaer charts/revaer \
  --values "${OPERATOR_VALUES:?path to verified operator values required}"
```

Rendering is not installation, cluster/PVC validation, signature verification,
package qualification or evidence of service startup. This binding does not
activate E1 or change runtime environment, probes or startup clocks. Required
installer, early-bootstrap and native amd64/arm64 qualification remain separate.

After those prerequisites and deployment authorization, select a verified chart
version and the prepared inputs explicitly:

```bash
helm install revaer oci://ghcr.io/vannadii/charts/revaer \
  --version "${CHART_VERSION:?verified chart version required}" \
  --values "${OPERATOR_VALUES:?path to verified operator values required}"
```

## First-Run Setup

Revaer starts in setup mode and the API bind guard remains loopback-only until
the instance is activated. The chart therefore uses in-container exec probes,
but cluster Services will not reach the API until the bind address is changed
out of setup mode.

Use a pod or deployment port-forward for the initial setup flow:

```bash
kubectl port-forward deployment/revaer 7070:7070 8080:8080
```

After completing setup, update the Revaer app profile bind address through the
UI or CLI so the Service can route traffic normally.

## Signature Verification

The release workflow's chart asset names are:

- `revaer-<version>.tgz`
- `revaer-<version>.tgz.prov`
- `revaer-helm-public.asc`
- `revaer-helm-public.gpg`

Verify a release package before installation:

```bash
curl -LO "https://github.com/VannaDii/Revaer/releases/download/${RELEASE_TAG:?}/revaer-${CHART_VERSION:?}.tgz"
curl -LO "https://github.com/VannaDii/Revaer/releases/download/${RELEASE_TAG:?}/revaer-${CHART_VERSION:?}.tgz.prov"
curl -LO "https://github.com/VannaDii/Revaer/releases/download/${RELEASE_TAG:?}/revaer-helm-public.gpg"

helm verify "./revaer-${CHART_VERSION:?}.tgz" --keyring ./revaer-helm-public.gpg
helm install revaer "./revaer-${CHART_VERSION:?}.tgz" \
  --verify \
  --keyring ./revaer-helm-public.gpg \
  --values "${OPERATOR_VALUES:?path to verified operator values required}"
```

Artifact Hub verified-publisher and official badges remain manual Artifact Hub
control-plane steps after the OCI repository is registered:

1. Add `oci://ghcr.io/vannadii/charts/revaer` as the repository URL in Artifact
   Hub and ensure the GHCR chart package is public so Artifact Hub can pull it
   anonymously.
2. Claim or verify the repository using the published `artifacthub.io` metadata
   tag. Release packaging publishes the repository ID and owner identity needed
   for Artifact Hub's `Verified publisher` flow.
3. After the repository shows `Verified publisher`, file Artifact Hub's
   `official` status request for the Revaer publisher or organization.

Use `revaer-logo.svg` as the Artifact Hub repository and organization logo when
completing that setup.

## Key Values

- `database.url`: Inline PostgreSQL connection string used to create a Secret.
- `database.existingSecret`: Existing Secret containing `DATABASE_URL`.
- `image.repository`: Image repository to deploy.
- `image.digest`: Required verified platform-image SHA-256 digest.
- `image.architecture`: Required matching `amd64` or `arm64` node architecture.
- `image.tag`: Must be empty. There is no tag or `appVersion` fallback.
- `compliance.existingClaim`: Required separately prepared existing PVC.
- `compliance.manifestDigest`: Required SHA-256 of the selected manifest bytes.
- `service.type`: Kubernetes Service type for the API/UI service.
- `configPersistence.*`: Persistent volume controls for `/config`.
- `dataPersistence.*`: Persistent volume controls for `/data`.
