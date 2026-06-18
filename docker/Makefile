# Convenience targets — run from repo root: make -f docker/Makefile up

COMPOSE_LOCAL := docker compose -f docker/docker-compose.yml
COMPOSE_PROD  := docker compose -f docker/docker-compose.prod.yml
IMAGE         ?= rsc-gateway:latest

.PHONY: up down build logs image k8s-apply k8s-tls-staging k8s-tls-apply k8s-tls-status \
        k8s-tls-dns-cloudflare k8s-tls-dns-digitalocean k8s-tls-wildcard-staging k8s-tls-wildcard-apply

up:
	$(COMPOSE_LOCAL) up --build -d

down:
	$(COMPOSE_LOCAL) down

build:
	$(COMPOSE_LOCAL) build

logs:
	$(COMPOSE_LOCAL) logs -f gateway nginx

image:
	docker build -f docker/Dockerfile -t $(IMAGE) .

prod-up:
	$(COMPOSE_PROD) up --build -d

k8s-apply:
	kubectl apply -f docker/k8s/namespace.yaml
	kubectl apply -f docker/k8s/configmap.yaml
	kubectl apply -f docker/k8s/deployment.yaml
	kubectl apply -f docker/k8s/service.yaml
	kubectl apply -f docker/k8s/hpa.yaml

# TLS: apply ClusterIssuers first (install-cert-manager.ps1), then Certificate + Ingress
k8s-tls-staging:
	kubectl apply -f docker/k8s/cert-manager/cluster-issuer-staging.yaml
	kubectl apply -f docker/k8s/certificate-staging.yaml
	kubectl apply -f docker/k8s/ingress-staging.yaml

k8s-tls-apply:
	kubectl apply -f docker/k8s/cert-manager/cluster-issuer-prod.yaml
	kubectl apply -f docker/k8s/certificate.yaml
	kubectl apply -f docker/k8s/ingress.yaml

k8s-tls-status:
	@echo "=== Certificate ==="
	kubectl describe certificate -n rsc-gateway rsc-gateway-tls
	@echo "=== cert-manager challenges ==="
	kubectl get challenges,certificaterequests,orders -n rsc-gateway
	@echo "=== Ingress TLS secret ==="
	kubectl get secret -n rsc-gateway rsc-gateway-tls

# DNS-01 wildcard — create API token secret in cert-manager namespace first
k8s-tls-dns-cloudflare:
	kubectl apply -f docker/k8s/cert-manager/cluster-issuer-dns-cloudflare-staging.yaml
	kubectl apply -f docker/k8s/cert-manager/cluster-issuer-dns-cloudflare-prod.yaml

k8s-tls-dns-digitalocean:
	kubectl apply -f docker/k8s/cert-manager/cluster-issuer-dns-digitalocean-staging.yaml
	kubectl apply -f docker/k8s/cert-manager/cluster-issuer-dns-digitalocean-prod.yaml

k8s-tls-wildcard-staging:
	kubectl apply -f docker/k8s/certificate-wildcard-staging.yaml
	kubectl apply -f docker/k8s/ingress-wildcard.yaml

k8s-tls-wildcard-apply:
	kubectl apply -f docker/k8s/certificate-wildcard.yaml
	kubectl apply -f docker/k8s/ingress-wildcard.yaml

k8s-tls-wildcard-do:
	kubectl apply -f docker/k8s/certificate-wildcard-digitalocean.yaml
	kubectl apply -f docker/k8s/ingress-wildcard.yaml

k8s-tls-wildcard-status:
	@echo "=== Wildcard Certificate ==="
	kubectl describe certificate -n rsc-gateway rsc-gateway-wildcard-tls
	kubectl get challenges,certificaterequests,orders -n rsc-gateway
	kubectl get secret -n rsc-gateway rsc-gateway-wildcard-tls
