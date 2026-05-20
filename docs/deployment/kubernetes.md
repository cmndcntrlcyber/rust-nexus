# Kubernetes quickstart (scaffold, Phase 1.3.9)

> Full guide lands in Phase 1.3.9. This skeleton makes the link
> target real.

v1.3 shipped `kustomize`-based manifests. **v1.4 adds a Helm chart**
that wraps the same manifests for operators preferring Helm. Pick one
or the other based on your cluster tooling — both produce equivalent
resources.

## Option A: Helm (v1.4+)

```bash
# Install from the in-tree chart.
helm install nexus-server ./deploy/helm/nexus-server \
    --namespace nexus --create-namespace \
    --set image.repository=ghcr.io/yourorg/nexus-server \
    --set image.tag=v1.4.0 \
    --set-file tls.caCert=./certs/ca.crt.pem \
    --set-file tls.serverCert=./certs/server.crt.pem \
    --set-file tls.serverKey=./certs/server.key.pem

# Inspect / lint before applying.
helm lint ./deploy/helm/nexus-server
helm template ./deploy/helm/nexus-server | less

# Upgrade in place.
helm upgrade nexus-server ./deploy/helm/nexus-server \
    --namespace nexus \
    --set image.tag=v1.4.1
```

The chart's `values.yaml` mirrors the kustomize base 1:1; override
sections (resource limits, replica count, `nexusToml` body) per
environment via `--set` flags or a local values file:

```bash
helm install nexus-server ./deploy/helm/nexus-server \
    --namespace nexus -f my-overrides.yaml
```

## Option B: kustomize (v1.3+)

```
deploy/k8s/
├── base/                              # Reference resources
│   ├── kustomization.yaml
│   ├── namespace.yaml
│   ├── server-statefulset.yaml        # 1 replica, PVC for identity + audit log
│   ├── server-service.yaml            # ClusterIP for A2A + legacy ports
│   ├── metrics-service.yaml           # ClusterIP for /metrics
│   ├── configmap-nexus-toml.yaml      # Server config
│   └── secret-tls.yaml                # TLS material (PLACEHOLDER values)
└── overlays/
    ├── dev/kustomization.yaml         # Lower resource requests + 1 GiB PVC
    └── prod/kustomization.yaml        # Higher requests + pinned image digest
```

## Quick local kind cluster

```bash
kind create cluster --name nexus-dev
kubectl apply -k deploy/k8s/overlays/dev/
kubectl -n nexus get pods -w
```

## TLS material

The `nexus-tls` Secret in `base/secret-tls.yaml` ships placeholders.
For production:

- Use `kubectl create secret generic nexus-tls --from-file=ca.crt.pem=... --from-file=server.crt.pem=... --from-file=server.key.pem=...`
- Or wire `sealed-secrets` / `external-secrets-operator` to materialize
  the secret from your secret store.

The StatefulSet's `volumeMounts` for the secret are read-only with
mode 0600.

## Agents (out-of-cluster)

Agents run on endpoint hosts, not in the cluster. The k8s manifests
only cover the server. See
[`production.md`](production.md#agent-provisioning) for the agent-side
deployment story (systemd unit pattern).
