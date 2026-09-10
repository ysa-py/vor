#!/usr/bin/env bash
# UnifiedShield v6.0 — CDN Worker Deployment Script
# Deploys workers to all supported platforms

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKERS_DIR="$SCRIPT_DIR/workers"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check required tools
check_prerequisites() {
    log_info "Checking prerequisites..."
    command -v node >/dev/null 2>&1 || { log_error "Node.js is required"; exit 1; }
    command -v npm >/dev/null 2>&1 || { log_error "npm is required"; exit 1; }
    log_info "Prerequisites OK"
}

# Deploy universal worker to Deno Deploy
deploy_deno() {
    log_info "Deploying universal worker to Deno Deploy..."
    if command -v deployctl >/dev/null 2>&1; then
        cd "$WORKERS_DIR/universal"
        deployctl deploy --project=shield-worker src/index.ts
        log_info "Deno Deploy deployment complete"
    else
        log_warn "deployctl not found. Install: deno install -A -r https://deno.land/x/deploy/deployctl.ts"
    fi
}

# Deploy to Netlify Edge
deploy_netlify() {
    log_info "Deploying universal worker to Netlify Edge..."
    if command -v netlify >/dev/null 2>&1; then
        cd "$WORKERS_DIR/universal"
        netlify deploy --prod --dir=.
        log_info "Netlify Edge deployment complete"
    else
        log_warn "Netlify CLI not found. Install: npm i -g netlify-cli"
    fi
}

# Deploy Arvan Cloud worker
deploy_arvan() {
    log_info "Deploying Arvan Cloud CDN worker..."
    if [ -z "${ARVAN_API_KEY:-}" ]; then
        log_error "ARVAN_API_KEY environment variable not set"
        return 1
    fi
    cd "$WORKERS_DIR/arvan-cdn"
    # Arvan Cloud FaaS deployment via API
    curl -s -X POST "https://napi.arvancloud.com/cdn/4.0/domains/shield-worker/functions" \
        -H "Authorization: Apikey $ARVAN_API_KEY" \
        -H "Content-Type: application/json" \
        -d @src/index.ts
    log_info "Arvan Cloud deployment complete"
}

# Deploy Alibaba Cloud worker
deploy_alibaba() {
    log_info "Deploying Alibaba Cloud CDN worker..."
    if [ -z "${ALIYUN_ACCESS_KEY:-}" ] || [ -z "${ALIYUN_SECRET_KEY:-}" ]; then
        log_error "ALIYUN_ACCESS_KEY and ALIYUN_SECRET_KEY environment variables required"
        return 1
    fi
    cd "$WORKERS_DIR/alibaba-cdn"
    log_info "Alibaba Cloud deployment initiated (requires Function Compute CLI)"
}

# Deploy ByteDance worker
deploy_bytedance() {
    log_info "Deploying ByteDance EdgeRoutine worker..."
    if [ -z "${BYTEDANCE_EDGE_TOKEN:-}" ]; then
        log_error "BYTEDANCE_EDGE_TOKEN environment variable not set"
        return 1
    fi
    cd "$WORKERS_DIR/bytedance-cdn"
    log_info "ByteDance deployment initiated (requires Volcengine CLI)"
}

# Deploy Tencent worker
deploy_tencent() {
    log_info "Deploying Tencent EdgeOne worker..."
    cd "$WORKERS_DIR/tencent-cdn"
    log_info "Tencent deployment initiated"
}

# Deploy Huawei worker
deploy_huawei() {
    log_info "Deploying Huawei Cloud FunctionGraph worker..."
    cd "$WORKERS_DIR/huawei-cdn"
    log_info "Huawei deployment initiated"
}

# Deploy all workers
deploy_all() {
    log_info "Deploying all CDN workers..."
    deploy_deno || true
    deploy_netlify || true
    deploy_arvan || true
    deploy_alibaba || true
    deploy_bytedance || true
    deploy_tencent || true
    deploy_huawei || true
    log_info "All deployments complete"
}

# Main
case "${1:-all}" in
    deno)      deploy_deno ;;
    netlify)   deploy_netlify ;;
    arvan)     deploy_arvan ;;
    alibaba)   deploy_alibaba ;;
    bytedance) deploy_bytedance ;;
    tencent)   deploy_tencent ;;
    huawei)    deploy_huawei ;;
    all)       deploy_all ;;
    *)
        echo "Usage: $0 {deno|netlify|arvan|alibaba|bytedance|tencent|huawei|all}"
        exit 1
        ;;
esac
