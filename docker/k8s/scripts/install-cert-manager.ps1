# Install cert-manager + NGINX Ingress (Helm) on a Kubernetes cluster.
# Run from repo root in PowerShell (requires helm + kubectl).
#
#   .\docker\k8s\scripts\install-cert-manager.ps1 -LetsEncryptEmail "ops@yourdomain.com"

param(
    [Parameter(Mandatory = $true)]
    [string] $LetsEncryptEmail,

    [string] $CertManagerVersion = "v1.16.2",
    [string] $IngressChartVersion = "4.11.3"
)

$ErrorActionPreference = "Stop"

Write-Host "==> cert-manager $CertManagerVersion"
helm repo add jetstack https://charts.jetstack.io --force-update
helm repo update
helm upgrade --install cert-manager jetstack/cert-manager `
    --namespace cert-manager --create-namespace `
    --version $CertManagerVersion `
    --set crds.enabled=true `
    --wait

Write-Host "==> NGINX Ingress Controller"
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx --force-update
helm upgrade --install ingress-nginx ingress-nginx/ingress-nginx `
    --namespace ingress-nginx --create-namespace `
    --version $IngressChartVersion `
    --set controller.ingressClassResource.name=nginx `
    --set controller.ingressClassResource.default=true `
    --set controller.ingressClass=nginx `
    --wait

Write-Host "==> ClusterIssuers (patch email)"
$staging = Get-Content -Raw "docker/k8s/cert-manager/cluster-issuer-staging.yaml"
$prod = Get-Content -Raw "docker/k8s/cert-manager/cluster-issuer-prod.yaml"
$staging = $staging -replace "LETSENCRYPT_EMAIL@example.com", $LetsEncryptEmail
$prod = $prod -replace "LETSENCRYPT_EMAIL@example.com", $LetsEncryptEmail
$staging | kubectl apply -f -
$prod | kubectl apply -f -

Write-Host "Done. Next (HTTP-01 single host):"
Write-Host "  1. kubectl get svc -n ingress-nginx ingress-nginx-controller"
Write-Host "  2. Point DNS api.example.com to LB IP"
Write-Host "  3. make -f docker/Makefile k8s-tls-apply"
Write-Host ""
Write-Host "Wildcard (DNS-01) — pick Cloudflare OR DigitalOcean:"
Write-Host "  kubectl apply -f docker/k8s/cert-manager/secret-cloudflare.example.yaml"
Write-Host "  make -f docker/Makefile k8s-tls-dns-cloudflare k8s-tls-wildcard-apply"
