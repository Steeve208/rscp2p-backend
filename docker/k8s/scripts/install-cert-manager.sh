#!/usr/bin/env bash
# Install cert-manager + NGINX Ingress (Helm).
# Usage: ./docker/k8s/scripts/install-cert-manager.sh ops@yourdomain.com

set -euo pipefail

LE_EMAIL="${1:?Usage: $0 <letsencrypt-email>}"
CERT_MANAGER_VERSION="${CERT_MANAGER_VERSION:-v1.16.2}"
INGRESS_CHART_VERSION="${INGRESS_CHART_VERSION:-4.11.3}"
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"

echo "==> cert-manager ${CERT_MANAGER_VERSION}"
helm repo add jetstack https://charts.jetstack.io --force-update
helm repo update
helm upgrade --install cert-manager jetstack/cert-manager \
  --namespace cert-manager --create-namespace \
  --version "${CERT_MANAGER_VERSION}" \
  --set crds.enabled=true \
  --wait

echo "==> NGINX Ingress Controller"
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx --force-update
helm upgrade --install ingress-nginx ingress-nginx/ingress-nginx \
  --namespace ingress-nginx --create-namespace \
  --version "${INGRESS_CHART_VERSION}" \
  --set controller.ingressClassResource.name=nginx \
  --set controller.ingressClassResource.default=true \
  --set controller.ingressClass=nginx \
  --wait

echo "==> ClusterIssuers"
sed "s/LETSENCRYPT_EMAIL@example.com/${LE_EMAIL}/g" \
  "${ROOT}/docker/k8s/cert-manager/cluster-issuer-staging.yaml" | kubectl apply -f -
sed "s/LETSENCRYPT_EMAIL@example.com/${LE_EMAIL}/g" \
  "${ROOT}/docker/k8s/cert-manager/cluster-issuer-prod.yaml" | kubectl apply -f -

echo "Done. HTTP-01: point api.example.com to LB IP, then: make -f docker/Makefile k8s-tls-apply"
echo "Wildcard DNS-01: create cloudflare or digitalocean secret in cert-manager, then:"
echo "  make -f docker/Makefile k8s-tls-dns-cloudflare k8s-tls-wildcard-apply"
