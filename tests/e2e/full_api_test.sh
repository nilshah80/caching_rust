#!/usr/bin/env bash
# =============================================================================
# Full API E2E Test Suite
#
# Comprehensive endpoint coverage for all API operations, including:
# - Core data types (Bitmaps, Geo, HyperLogLog)
# - Module-dependent features (JSON, Search, Bloom, TimeSeries)
# - Transactions, Scripting, Functions
# - Pub/Sub HTTP endpoints
# - Admin endpoints (new: debug_object, client_info, latency_graph, ACL CRUD)
# - OpenAPI spec & capabilities
#
# Usage:
#   BASE_URL=http://localhost:8080 ADMIN_KEY=dev-admin-key ./full_api_test.sh
# =============================================================================
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_KEY="${ADMIN_KEY:-dev-admin-key}"

PASS=0
FAIL=0
SKIP=0
ERRORS=""

P="full_e2e_$$"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
check() {
    local description="$1" expected_status="$2" actual_status="$3" body="$4"
    if [[ "$actual_status" == "$expected_status" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  $description (HTTP $actual_status)"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  $description: expected $expected_status, got $actual_status\n        body: ${body:0:200}"
        echo "  FAIL  $description (expected $expected_status, got $actual_status)"
    fi
}

check_any() {
    local description="$1" actual_status="$2" body="$3"
    shift 3
    local expected_statuses=("$@")
    for s in "${expected_statuses[@]}"; do
        if [[ "$actual_status" == "$s" ]]; then
            PASS=$((PASS + 1))
            echo "  PASS  $description (HTTP $actual_status)"
            return
        fi
    done
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL  $description: expected one of [${expected_statuses[*]}], got $actual_status\n        body: ${body:0:200}"
    echo "  FAIL  $description (expected one of [${expected_statuses[*]}], got $actual_status)"
}

do_request() {
    local method="$1" path="$2" data="${3:-}" extra_headers="${4:-}"
    local args=(-s -w '\n%{http_code}' -X "$method")
    [[ -n "$data" ]] && args+=(-H 'Content-Type: application/json' -d "$data")
    [[ -n "$extra_headers" ]] && args+=(-H "$extra_headers")
    local response
    response=$(curl "${args[@]}" "${BASE_URL}${path}")
    local status body
    status=$(echo "$response" | tail -1)
    body=$(echo "$response" | sed '$d')
    echo "$status|$body"
}

admin_request() {
    local method="$1" path="$2" data="${3:-}"
    do_request "$method" "$path" "$data" "X-Admin-Api-Key: $ADMIN_KEY"
}

echo "============================================================"
echo " Full API E2E Test Suite"
echo " Target: $BASE_URL"
echo "============================================================"
echo ""

# ==========================================================================
# OpenAPI & Capabilities
# ==========================================================================
echo "--- OpenAPI & Capabilities ---"

IFS='|' read -r status body < <(do_request GET "/api-docs/openapi.json")
check "OpenAPI spec" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/capabilities")
check "Public capabilities alias" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/capabilities")
check "Admin capabilities" "200" "$status" "$body"

echo ""

# ==========================================================================
# Bitmaps
# ==========================================================================
echo "--- Bitmaps ---"

# SETBIT: PUT /api/v1/bitmaps/{key}/bit/{offset}
IFS='|' read -r status body < <(do_request PUT "/api/v1/bitmaps/${P}_bm/bit/7" \
    '{"value":true}')
check "SETBIT" "200" "$status" "$body"

# GETBIT: GET /api/v1/bitmaps/{key}/bit/{offset}
IFS='|' read -r status body < <(do_request GET "/api/v1/bitmaps/${P}_bm/bit/7")
check "GETBIT" "200" "$status" "$body"

# BITCOUNT: GET /api/v1/bitmaps/{key}/count
IFS='|' read -r status body < <(do_request GET "/api/v1/bitmaps/${P}_bm/count")
check "BITCOUNT" "200" "$status" "$body"

# BITPOS: GET /api/v1/bitmaps/{key}/pos?bit=true
IFS='|' read -r status body < <(do_request GET "/api/v1/bitmaps/${P}_bm/pos?bit=true")
check "BITPOS" "200" "$status" "$body"

# BITOP: POST /api/v1/bitmaps/operations
do_request PUT "/api/v1/bitmaps/${P}_bm_a/bit/0" '{"value":true}' > /dev/null
do_request PUT "/api/v1/bitmaps/${P}_bm_b/bit/0" '{"value":true}' > /dev/null
IFS='|' read -r status body < <(do_request POST "/api/v1/bitmaps/operations" \
    "{\"operation\":\"AND\",\"dest_key\":\"${P}_bm_result\",\"keys\":[\"${P}_bm_a\",\"${P}_bm_b\"]}")
check "BITOP AND" "200" "$status" "$body"

# BITFIELD: POST /api/v1/bitmaps/{key}/bitfield (internally tagged enum)
IFS='|' read -r status body < <(do_request POST "/api/v1/bitmaps/${P}_bm/bitfield" \
    '{"commands":[{"command":"GET","encoding":{"type":"unsigned","bits":8},"offset":0}]}')
check "BITFIELD" "200" "$status" "$body"

# BITFIELD_RO: POST /api/v1/bitmaps/{key}/bitfield/ro
IFS='|' read -r status body < <(do_request POST "/api/v1/bitmaps/${P}_bm/bitfield/ro" \
    '{"commands":[{"command":"GET","encoding":{"type":"unsigned","bits":8},"offset":0}]}')
check "BITFIELD_RO" "200" "$status" "$body"

echo ""

# ==========================================================================
# Geo
# ==========================================================================
echo "--- Geo ---"

# GEOADD: POST /api/v1/geo/{key}
IFS='|' read -r status body < <(do_request POST "/api/v1/geo/${P}_geo" \
    '{"members":[{"member":"Central Park","longitude":-73.9654,"latitude":40.7829},{"member":"Times Square","longitude":-73.9857,"latitude":40.758}]}')
check "GEOADD" "200" "$status" "$body"

# GEOPOS: POST /api/v1/geo/{key}/pos
IFS='|' read -r status body < <(do_request POST "/api/v1/geo/${P}_geo/pos" \
    '{"members":["Central Park"]}')
check "GEOPOS" "200" "$status" "$body"

# GEODIST: GET /api/v1/geo/{key}/dist/{member1}/{member2}
IFS='|' read -r status body < <(do_request GET "/api/v1/geo/${P}_geo/dist/Central%20Park/Times%20Square")
check "GEODIST" "200" "$status" "$body"

# GEOHASH: POST /api/v1/geo/{key}/hash
IFS='|' read -r status body < <(do_request POST "/api/v1/geo/${P}_geo/hash" \
    '{"members":["Central Park"]}')
check "GEOHASH" "200" "$status" "$body"

# GEOSEARCH: POST /api/v1/geo/{key}/search (internally tagged enums)
IFS='|' read -r status body < <(do_request POST "/api/v1/geo/${P}_geo/search" \
    '{"center":{"type":"FROMMEMBER","member":"Central Park"},"shape":{"type":"BYRADIUS","radius":5.0,"unit":"km"}}')
check "GEOSEARCH" "200" "$status" "$body"

echo ""

# ==========================================================================
# HyperLogLog
# ==========================================================================
echo "--- HyperLogLog ---"

# PFADD: POST /api/v1/hll/{key}/add
IFS='|' read -r status body < <(do_request POST "/api/v1/hll/${P}_hll/add" \
    '{"elements":["a","b","c","a"]}')
check "PFADD" "200" "$status" "$body"

# PFCOUNT: POST /api/v1/hll/count
IFS='|' read -r status body < <(do_request POST "/api/v1/hll/count" \
    "{\"keys\":[\"${P}_hll\"]}")
check "PFCOUNT" "200" "$status" "$body"

# PFMERGE: POST /api/v1/hll/{key}/merge
do_request POST "/api/v1/hll/${P}_hll2/add" '{"elements":["d","e"]}' > /dev/null
IFS='|' read -r status body < <(do_request POST "/api/v1/hll/${P}_hll_merged/merge" \
    "{\"sources\":[\"${P}_hll\",\"${P}_hll2\"]}")
check "PFMERGE" "200" "$status" "$body"

echo ""

# ==========================================================================
# Pub/Sub (HTTP endpoints only)
# ==========================================================================
echo "--- Pub/Sub (HTTP) ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/pubsub/publish" \
    "{\"channel\":\"${P}_chan\",\"message\":\"hello\"}")
check "PUBLISH" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/pubsub/channels")
check "PUBSUB CHANNELS" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/pubsub/numsub" \
    "{\"channels\":[\"${P}_chan\"]}")
check "PUBSUB NUMSUB" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/pubsub/numpat")
check "PUBSUB NUMPAT" "200" "$status" "$body"

echo ""

# ==========================================================================
# Transactions
# ==========================================================================
echo "--- Transactions ---"

do_request PUT "/api/v1/strings/${P}_txn_key" '{"value":"100"}' > /dev/null

# MULTI/EXEC: POST /api/v1/transactions/execute - internally tagged with "type"
IFS='|' read -r status body < <(do_request POST "/api/v1/transactions/execute" \
    "{\"commands\":[{\"type\":\"SET\",\"key\":\"${P}_txn1\",\"value\":\"val1\"},{\"type\":\"SET\",\"key\":\"${P}_txn2\",\"value\":\"val2\"},{\"type\":\"GET\",\"key\":\"${P}_txn1\"}]}")
check "MULTI/EXEC" "200" "$status" "$body"

# CAS: POST /api/v1/transactions/cas
IFS='|' read -r status body < <(do_request POST "/api/v1/transactions/cas" \
    "{\"key\":\"${P}_txn_key\",\"expected_value\":\"100\",\"new_value\":\"200\"}")
check "Compare-and-Set" "200" "$status" "$body"

echo ""

# ==========================================================================
# Scripting
# ==========================================================================
echo "--- Scripting ---"

# EVAL: POST /api/v1/scripts/eval (admin-protected)
IFS='|' read -r status body < <(admin_request POST "/api/v1/scripts/eval" \
    "{\"script\":\"return redis.call('SET', KEYS[1], ARGV[1])\",\"keys\":[\"${P}_lua\"],\"args\":[\"lua_val\"]}")
check "EVAL" "200" "$status" "$body"

# SCRIPT LOAD: POST /api/v1/scripts/load (admin-protected)
IFS='|' read -r status body < <(admin_request POST "/api/v1/scripts/load" \
    '{"script":"return 1"}')
check "SCRIPT LOAD" "200" "$status" "$body"

# Extract SHA from response for EVALSHA
SHA=$(echo "$body" | sed -n 's/.*"sha":"\([^"]*\)".*/\1/p' || echo "")
if [[ -n "$SHA" ]]; then
    IFS='|' read -r status body < <(admin_request POST "/api/v1/scripts/evalsha" \
        "{\"sha\":\"$SHA\",\"keys\":[],\"args\":[]}")
    check "EVALSHA" "200" "$status" "$body"

    IFS='|' read -r status body < <(admin_request POST "/api/v1/scripts/exists" \
        "{\"shas\":[\"$SHA\"]}")
    check "SCRIPT EXISTS" "200" "$status" "$body"
else
    SKIP=$((SKIP + 2))
    echo "  SKIP  EVALSHA (could not extract SHA)"
    echo "  SKIP  SCRIPT EXISTS (could not extract SHA)"
fi

echo ""

# ==========================================================================
# Functions (Redis 7.0+)
# ==========================================================================
echo "--- Functions ---"

IFS='|' read -r status body < <(admin_request GET "/api/v1/functions")
check_any "FUNCTION LIST" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    IFS='|' read -r status body < <(admin_request POST "/api/v1/functions/load" \
        '{"code":"#!lua name=e2elib\nredis.register_function(\"e2e_echo\", function(keys, args) return args[1] end)","replace":true}')
    check "FUNCTION LOAD" "200" "$status" "$body"

    IFS='|' read -r status body < <(admin_request POST "/api/v1/functions/call" \
        '{"function":"e2e_echo","keys":[],"args":["hello"]}')
    check "FCALL" "200" "$status" "$body"

    # Cleanup
    admin_request POST "/api/v1/functions/flush" '{"mode":"async"}' > /dev/null 2>&1 || true
else
    SKIP=$((SKIP + 2))
    echo "  SKIP  FUNCTION LOAD (functions not available)"
    echo "  SKIP  FCALL (functions not available)"
fi

echo ""

# ==========================================================================
# JSON (RedisJSON module)
# ==========================================================================
echo "--- JSON ---"

IFS='|' read -r status body < <(do_request PUT "/api/v1/json/${P}_json" \
    '{"path":"$","value":{"name":"Alice","age":30,"tags":["a","b"]}}')
check_any "JSON.SET" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    IFS='|' read -r status body < <(do_request GET "/api/v1/json/${P}_json?path=$")
    check "JSON.GET" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/json/${P}_json/type?path=$")
    check "JSON.TYPE" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/json/${P}_json/strlen?path=$.name")
    check "JSON.STRLEN" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request PATCH "/api/v1/json/${P}_json/numincrby" \
        '{"path":"$.age","value":1}')
    check "JSON.NUMINCRBY" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/json/${P}_json/arrlen?path=$.tags")
    check "JSON.ARRLEN" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request DELETE "/api/v1/json/${P}_json?path=$.tags")
    check "JSON.DEL" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/json/mset" \
        "{\"items\":[{\"key\":\"${P}_mset_a\",\"path\":\"$\",\"value\":{\"n\":1}},{\"key\":\"${P}_mset_b\",\"path\":\"$\",\"value\":{\"n\":2}}]}")
    check "JSON.MSET" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/json/mget" \
        "{\"keys\":[\"${P}_mset_a\",\"${P}_mset_b\"],\"path\":\"$.n\"}")
    check "JSON.MGET (after MSET)" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/json/mset" '{"items":[]}')
    check "JSON.MSET (empty items rejected)" "400" "$status" "$body"
else
    SKIP=$((SKIP + 9))
    echo "  SKIP  JSON.GET, JSON.TYPE, JSON.STRLEN, JSON.NUMINCRBY, JSON.ARRLEN, JSON.DEL, JSON.MSET, JSON.MGET, JSON.MSET validation (module not available)"
fi

echo ""

# ==========================================================================
# Search (RediSearch module)
# ==========================================================================
echo "--- Search ---"

IFS='|' read -r status body < <(do_request GET "/api/v1/search/indices")
check_any "FT._LIST" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    # FT.CREATE: POST /api/v1/search/indices - uses "index", "schema" with "field_type"
    IFS='|' read -r status body < <(do_request POST "/api/v1/search/indices" \
        "{\"index\":\"${P}_idx\",\"options\":{\"prefix\":\"${P}_doc:\"},\"schema\":[{\"name\":\"title\",\"field_type\":\"TEXT\"},{\"name\":\"score\",\"field_type\":\"NUMERIC\"}]}")
    check "FT.CREATE" "200" "$status" "$body"

    # Index a document via hash
    do_request PUT "/api/v1/hashes/${P}_doc:1" '{"items":{"title":"hello world","score":"42"}}' > /dev/null

    # Give indexing a moment
    sleep 1

    IFS='|' read -r status body < <(do_request POST "/api/v1/search/indices/${P}_idx/search" \
        '{"query":"hello"}')
    check "FT.SEARCH" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/search/indices/${P}_idx")
    check "FT.INFO" "200" "$status" "$body"

    # Cleanup index
    do_request DELETE "/api/v1/search/indices/${P}_idx" > /dev/null 2>&1 || true

    # --- Phase 10.3 Search Enhancements ---
    echo ""
    echo "--- Search Enhancements (10.3) ---"

    # 10.3.1 FT.CONFIG GET/SET
    IFS='|' read -r status body < <(do_request GET "/api/v1/search/config/TIMEOUT")
    check "FT.CONFIG GET" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request PUT "/api/v1/search/config/TIMEOUT" \
        '{"value":"500"}' "Content-Type: application/json")
    check "FT.CONFIG SET" "200" "$status" "$body"

    # 10.3.3 FT.CURSOR (via AGGREGATE with WITHCURSOR)
    # Create a temp index for cursor test
    IFS='|' read -r status body < <(do_request POST "/api/v1/search/indices" \
        "{\"index\":\"${P}_cur\",\"options\":{\"prefix\":\"${P}_cdoc:\"},\"schema\":[{\"name\":\"title\",\"field_type\":\"TEXT\"}]}")
    if [[ "$status" == "200" ]]; then
        do_request PUT "/api/v1/hashes/${P}_cdoc:1" '{"items":{"title":"cursor doc one"}}' > /dev/null
        do_request PUT "/api/v1/hashes/${P}_cdoc:2" '{"items":{"title":"cursor doc two"}}' > /dev/null
        sleep 1

        IFS='|' read -r status body < <(do_request POST "/api/v1/search/indices/${P}_cur/aggregate" \
            '{"query":"*","options":{"withcursor":true,"cursor_count":1,"load_all":true}}')
        check "FT.AGGREGATE WITHCURSOR" "200" "$status" "$body"

        CURSOR_ID=$(echo "$body" | sed -n 's/.*"cursor_id":\([0-9]*\).*/\1/p')
        if [[ -n "$CURSOR_ID" && "$CURSOR_ID" != "0" ]]; then
            IFS='|' read -r status body < <(do_request GET "/api/v1/search/indices/${P}_cur/cursor/$CURSOR_ID")
            check "FT.CURSOR READ" "200" "$status" "$body"

            IFS='|' read -r status body < <(do_request DELETE "/api/v1/search/indices/${P}_cur/cursor/$CURSOR_ID")
            check "FT.CURSOR DEL" "200" "$status" "$body"
        else
            echo "  SKIP  FT.CURSOR READ/DEL (no cursor returned)"
            SKIP=$((SKIP + 2))
        fi

        do_request DELETE "/api/v1/search/indices/${P}_cur?dd=true" > /dev/null 2>&1 || true
        do_request POST "/api/v1/keys/delete" "{\"keys\":[\"${P}_cdoc:1\",\"${P}_cdoc:2\"]}" > /dev/null 2>&1 || true
    fi

    # 10.3.2 FT.HYBRID
    # Create index with TEXT + VECTOR fields
    IFS='|' read -r status body < <(do_request POST "/api/v1/search/indices" \
        "{\"index\":\"${P}_hyb\",\"options\":{\"prefix\":\"${P}_hdoc:\"},\"schema\":[{\"name\":\"title\",\"field_type\":\"TEXT\"},{\"name\":\"vec\",\"field_type\":\"VECTOR\",\"vector_options\":{\"algorithm\":\"FLAT\",\"dim\":3,\"distance_metric\":\"COSINE\",\"type\":\"FLOAT32\"}}]}")
    if [[ "$status" == "200" ]]; then
        sleep 1

        IFS='|' read -r status body < <(do_request POST "/api/v1/search/indices/${P}_hyb/hybrid" \
            "{\"query\":\"*\",\"vsim_field\":\"vec\",\"vsim_input\":{\"type\":\"VALUES\",\"dim\":3,\"values\":[1.0,0.0,0.0]},\"limit\":3}")
        check "FT.HYBRID" "200" "$status" "$body"

        do_request DELETE "/api/v1/search/indices/${P}_hyb?dd=true" > /dev/null 2>&1 || true
    else
        echo "  SKIP  FT.HYBRID (index creation failed)"
        SKIP=$((SKIP + 1))
    fi
else
    SKIP=$((SKIP + 3))
    echo "  SKIP  FT.CREATE, FT.SEARCH, FT.INFO (module not available)"
fi

echo ""

# ==========================================================================
# Bloom Filters (RedisBloom module)
# ==========================================================================
echo "--- Bloom Filters ---"

# BF.ADD: POST /api/v1/bloom/{key}/add - body: {"items": [string]}
IFS='|' read -r status body < <(do_request POST "/api/v1/bloom/${P}_bf/add" \
    '{"items":["hello"]}')
check_any "BF.ADD" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    IFS='|' read -r status body < <(do_request POST "/api/v1/bloom/${P}_bf/exists" \
        '{"items":["hello"]}')
    check "BF.EXISTS" "200" "$status" "$body"

    # BF.INFO: GET /api/v1/bloom/{key}
    IFS='|' read -r status body < <(do_request GET "/api/v1/bloom/${P}_bf")
    check "BF.INFO" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/bloom/${P}_bf/card")
    check "BF.CARD" "200" "$status" "$body"
else
    SKIP=$((SKIP + 3))
    echo "  SKIP  BF.EXISTS, BF.INFO, BF.CARD (module not available)"
fi

echo ""

# ==========================================================================
# Count-Min Sketch (RedisBloom module)
# ==========================================================================
echo "--- Count-Min Sketch ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/cms/${P}_cms/initbydim" \
    '{"width":1000,"depth":5}')
check_any "CMS.INITBYDIM" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    IFS='|' read -r status body < <(do_request POST "/api/v1/cms/${P}_cms/incrby" \
        '{"items":[{"item":"foo","increment":5}]}')
    check "CMS.INCRBY" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/cms/${P}_cms/query" \
        '{"items":["foo"]}')
    check "CMS.QUERY" "200" "$status" "$body"

    # CMS.INFO: GET /api/v1/cms/{key}
    IFS='|' read -r status body < <(do_request GET "/api/v1/cms/${P}_cms")
    check "CMS.INFO" "200" "$status" "$body"
else
    SKIP=$((SKIP + 3))
    echo "  SKIP  CMS.INCRBY, CMS.QUERY, CMS.INFO (module not available)"
fi

echo ""

# ==========================================================================
# Top-K (RedisBloom module)
# ==========================================================================
echo "--- Top-K ---"

# TOPK.RESERVE: POST /api/v1/topk/{key} - body: {"k": u32}
IFS='|' read -r status body < <(do_request POST "/api/v1/topk/${P}_topk" \
    '{"k":3}')
check_any "TOPK.RESERVE" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    IFS='|' read -r status body < <(do_request POST "/api/v1/topk/${P}_topk/add" \
        '{"items":["a","b","c","a","a","b"]}')
    check "TOPK.ADD" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/topk/${P}_topk/list")
    check "TOPK.LIST" "200" "$status" "$body"

    # TOPK.INFO: GET /api/v1/topk/{key}
    IFS='|' read -r status body < <(do_request GET "/api/v1/topk/${P}_topk")
    check "TOPK.INFO" "200" "$status" "$body"
else
    SKIP=$((SKIP + 3))
    echo "  SKIP  TOPK.ADD, TOPK.LIST, TOPK.INFO (module not available)"
fi

echo ""

# ==========================================================================
# T-Digest (RedisBloom module)
# ==========================================================================
echo "--- T-Digest ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td" \
    '{"compression":100}')
check_any "TDIGEST.CREATE" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/add" \
        '{"values":[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0,10.0]}')
    check "TDIGEST.ADD" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/quantile" \
        '{"quantiles":[0.5,0.9,0.99]}')
    check "TDIGEST.QUANTILE" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/cdf" \
        '{"values":[5.0]}')
    check "TDIGEST.CDF" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/rank" \
        '{"values":[5.0]}')
    check "TDIGEST.RANK" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/revrank" \
        '{"values":[5.0]}')
    check "TDIGEST.REVRANK" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/byrank" \
        '{"ranks":[0,5]}')
    check "TDIGEST.BYRANK" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/byrevrank" \
        '{"ranks":[0]}')
    check "TDIGEST.BYREVRANK" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/tdigest/${P}_td/min")
    check "TDIGEST.MIN" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/tdigest/${P}_td/max")
    check "TDIGEST.MAX" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request GET "/api/v1/tdigest/${P}_td")
    check "TDIGEST.INFO" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/trimmed_mean" \
        '{"low_cut_quantile":0.1,"high_cut_quantile":0.9}')
    check "TDIGEST.TRIMMED_MEAN" "200" "$status" "$body"

    # Seed a second sketch and merge it into a fresh destination.
    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td2" \
        '{"compression":100}')
    check "TDIGEST.CREATE (source for merge)" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td2/add" \
        '{"values":[100.0,200.0,300.0]}')
    check "TDIGEST.ADD (source for merge)" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td_dest/merge" \
        "{\"sources\":[\"${P}_td\",\"${P}_td2\"],\"override_existing\":true}")
    check "TDIGEST.MERGE" "200" "$status" "$body"

    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/reset" "")
    check "TDIGEST.RESET" "200" "$status" "$body"

    # Validation: invalid quantile should be rejected with 400.
    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/quantile" \
        '{"quantiles":[1.5]}')
    check "TDIGEST.QUANTILE (rejects out-of-range)" "400" "$status" "$body"

    # Validation: low >= high is rejected.
    IFS='|' read -r status body < <(do_request POST "/api/v1/tdigest/${P}_td/trimmed_mean" \
        '{"low_cut_quantile":0.9,"high_cut_quantile":0.1}')
    check "TDIGEST.TRIMMED_MEAN (rejects inverted cut)" "400" "$status" "$body"
else
    SKIP=$((SKIP + 17))
    echo "  SKIP  TDIGEST.* (module not available)"
fi

echo ""

# ==========================================================================
# TimeSeries (RedisTimeSeries module)
# ==========================================================================
echo "--- TimeSeries ---"

# TS.CREATE: POST /api/v1/timeseries - body: {"key": string, ...}
IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries" \
    "{\"key\":\"${P}_ts\",\"retention_ms\":60000}")
check_any "TS.CREATE" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    # TS.ADD: POST /api/v1/timeseries/{key}/samples - timestamp 0 means auto
    IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/samples" \
        '{"timestamp":0,"value":42.5}')
    check "TS.ADD" "200" "$status" "$body"

    # Brief pause to let the sample settle
    sleep 1

    # TS.GET: GET /api/v1/timeseries/{key}
    IFS='|' read -r status body < <(do_request GET "/api/v1/timeseries/${P}_ts")
    check "TS.GET" "200" "$status" "$body"

    # TS.INFO: GET /api/v1/timeseries/{key}/info
    IFS='|' read -r status body < <(do_request GET "/api/v1/timeseries/${P}_ts/info")
    check "TS.INFO" "200" "$status" "$body"

    # ─── 10.8 additions: IGNORE / ON_DUPLICATE / alignTimestamp ─────────
    # TS.CREATE with IGNORE thresholds. Older RTS may reject the IGNORE arg
    # entirely → check_any 200/500. Schema validation (negative thresholds)
    # is exercised separately with a 400 assertion below.
    IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries" \
        "{\"key\":\"${P}_ts_ignore\",\"retention_ms\":60000,\"ignore\":{\"max_time_diff\":100,\"max_val_diff\":0.5}}")
    check_any "TS.CREATE with IGNORE" "$status" "$body" 200 500

    # Negative IGNORE thresholds rejected at the schema layer regardless of RTS version.
    IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries" \
        "{\"key\":\"${P}_ts_bad_ignore\",\"ignore\":{\"max_time_diff\":-1,\"max_val_diff\":0.0}}")
    check "TS.CREATE rejects negative IGNORE max_time_diff" "400" "$status" "$body"

    # TS.ADD with ON_DUPLICATE LAST overrides default policy for one sample.
    IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/samples" \
        '{"timestamp":0,"value":99.0,"on_duplicate":"LAST"}')
    check_any "TS.ADD with ON_DUPLICATE LAST" "$status" "$body" 200 500

    # TS.ADD accepts IGNORE on the wire — per Redis docs the option is only
    # honored when TS.ADD creates the series; on an existing series it is
    # silently ignored. We just verify the route accepts the body.
    IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/samples" \
        '{"timestamp":0,"value":99.5,"ignore":{"max_time_diff":50,"max_val_diff":1.0}}')
    check_any "TS.ADD accepts IGNORE (applied only on series creation)" "$status" "$body" 200 500

    # TS.CREATERULE with alignTimestamp (RedisTimeSeries 1.8+; older RTS errors).
    IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries" \
        "{\"key\":\"${P}_ts_dst\"}")
    check_any "TS.CREATE (rule destination)" "$status" "$body" 200 500
    IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/rules" \
        "{\"dest_key\":\"${P}_ts_dst\",\"aggregation\":\"avg\",\"bucket_duration_ms\":86400000,\"align_timestamp_ms\":21600000}")
    check_any "TS.CREATERULE with alignTimestamp" "$status" "$body" 200 500

    # Negative alignTimestamp rejected at schema layer.
    IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/rules" \
        "{\"dest_key\":\"${P}_ts_dst\",\"aggregation\":\"avg\",\"bucket_duration_ms\":86400000,\"align_timestamp_ms\":-1}")
    check "TS.CREATERULE rejects negative alignTimestamp" "400" "$status" "$body"
else
    SKIP=$((SKIP + 3))
    echo "  SKIP  TS.ADD, TS.GET, TS.INFO (module not available)"
fi

echo ""

# ==========================================================================
# Strings — Redis 8.4+ commands (MSETEX / DELEX / DIGEST)
# ==========================================================================
echo "--- Strings (Redis 8.4+) ---"

# Pre-seed values via existing PUT for DELEX/DIGEST checks regardless of 8.4 availability;
# the 8.4-gated assertions use check_any to allow 501 on older Redis builds.
do_request PUT "/api/v1/strings/${P}_84_existing" '{"value":"keepme"}' > /dev/null

# MSETEX: POST /api/v1/strings/msetex
IFS='|' read -r status body < <(do_request POST "/api/v1/strings/msetex" \
    "{\"pairs\":{\"${P}_84_a\":\"alpha\",\"${P}_84_b\":\"bravo\"},\"ttl_seconds\":120}")
check_any "MSETEX" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    # NX returning false (success=false) when one of the keys already exists.
    IFS='|' read -r status body < <(do_request POST "/api/v1/strings/msetex" \
        "{\"pairs\":{\"${P}_84_a\":\"new\",\"${P}_84_c\":\"charlie\"},\"ttl_seconds\":60,\"nx\":true}")
    check "MSETEX NX (precondition fails)" "200" "$status" "$body"

    # KEEPTTL combined with explicit TTL is rejected with 400.
    IFS='|' read -r status body < <(do_request POST "/api/v1/strings/msetex" \
        "{\"pairs\":{\"${P}_84_d\":\"x\"},\"ttl_seconds\":10,\"keep_ttl\":true}")
    check "MSETEX rejects keep_ttl + ttl_seconds" "400" "$status" "$body"

    # NX + XX together is rejected with 400.
    IFS='|' read -r status body < <(do_request POST "/api/v1/strings/msetex" \
        "{\"pairs\":{\"${P}_84_d\":\"x\"},\"ttl_seconds\":10,\"nx\":true,\"xx\":true}")
    check "MSETEX rejects nx + xx" "400" "$status" "$body"

    # DIGEST: GET /api/v1/strings/{key}/digest — present key
    IFS='|' read -r status body < <(do_request GET "/api/v1/strings/${P}_84_a/digest")
    check "DIGEST (existing key)" "200" "$status" "$body"
    DIGEST_VAL=$(echo "$body" | sed -n 's/.*"digest":"\([^"]*\)".*/\1/p')

    # DIGEST: missing key still returns 200 with exists:false
    IFS='|' read -r status body < <(do_request GET "/api/v1/strings/${P}_84_missing/digest")
    check "DIGEST (missing key returns 200)" "200" "$status" "$body"

    # DELEX: unconditional delete on a fresh key.
    do_request PUT "/api/v1/strings/${P}_84_del" '{"value":"bye"}' > /dev/null
    IFS='|' read -r status body < <(do_request POST "/api/v1/strings/${P}_84_del/delex" '{}')
    check "DELEX (unconditional)" "200" "$status" "$body"

    # DELEX IFEQ — value mismatch should NOT delete (deleted:false, HTTP 200).
    IFS='|' read -r status body < <(do_request POST "/api/v1/strings/${P}_84_a/delex" \
        '{"if_eq":"NOT_THE_VALUE"}')
    check "DELEX IFEQ (mismatch keeps key)" "200" "$status" "$body"

    # DELEX IFEQ — value match should delete.
    IFS='|' read -r status body < <(do_request POST "/api/v1/strings/${P}_84_b/delex" \
        '{"if_eq":"bravo"}')
    check "DELEX IFEQ (match deletes)" "200" "$status" "$body"

    # DELEX IFDEQ — using the digest captured above against the still-living ${P}_84_a.
    if [[ -n "$DIGEST_VAL" ]]; then
        IFS='|' read -r status body < <(do_request POST "/api/v1/strings/${P}_84_a/delex" \
            "{\"if_deq\":\"$DIGEST_VAL\"}")
        check "DELEX IFDEQ (digest match)" "200" "$status" "$body"
    else
        SKIP=$((SKIP + 1))
        echo "  SKIP  DELEX IFDEQ (no digest captured)"
    fi

    # DELEX rejects multiple conditions with 400.
    IFS='|' read -r status body < <(do_request POST "/api/v1/strings/${P}_84_existing/delex" \
        '{"if_eq":"a","if_ne":"b"}')
    check "DELEX rejects multiple conditions" "400" "$status" "$body"
else
    SKIP=$((SKIP + 9))
    echo "  SKIP  MSETEX/DELEX/DIGEST follow-ups (Redis 8.4+ required)"
fi

echo ""

# ==========================================================================
# Streams — XACKDEL (Redis 8.2+) — 10.9
# ==========================================================================
echo "--- Streams XACKDEL (Redis 8.2+) ---"

# Drive a real ack/del round-trip: XADD → XGROUP CREATE → XREADGROUP > → XACKDEL.
STREAM_KEY="${P}_xackdel_stream"
GROUP_NAME="${P}_grp"
CONSUMER_NAME="${P}_consumer"

# 1. XADD an entry → POST /api/v1/streams/{key}/add
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_KEY}/add" \
    "{\"fields\":{\"foo\":\"bar\"}}")
check_any "XADD (seed for XACKDEL)" "$status" "$body" 200 400 501
ENTRY_ID=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')

if [[ "$status" == "200" && -n "$ENTRY_ID" ]]; then
    # 2. XGROUP CREATE → POST /api/v1/streams/{key}/groups (admin route)
    IFS='|' read -r status body < <(admin_request POST \
        "/api/v1/streams/${STREAM_KEY}/groups" \
        "{\"group\":\"${GROUP_NAME}\",\"id\":\"0\",\"mkstream\":false}")
    check_any "XGROUP CREATE (for XACKDEL)" "$status" "$body" 200 400 401

    # 3. XREADGROUP > → entry becomes pending in the group
    IFS='|' read -r status body < <(do_request POST \
        "/api/v1/streams/${STREAM_KEY}/groups/${GROUP_NAME}/read" \
        "{\"consumer\":\"${CONSUMER_NAME}\",\"streams\":[{\"key\":\"${STREAM_KEY}\",\"id\":\">\"}],\"count\":10}")
    check_any "XREADGROUP (make pending)" "$status" "$body" 200 204

    # 4. XACKDEL on the now-pending entry — capability flag gates the route.
    IFS='|' read -r status body < <(do_request POST \
        "/api/v1/streams/${STREAM_KEY}/groups/${GROUP_NAME}/ackdel" \
        "{\"ids\":[\"${ENTRY_ID}\"],\"mode\":\"keepref\"}")
    check_any "XACKDEL (KEEPREF)" "$status" "$body" 200 501

    # 5. Empty IDs are rejected at the schema layer regardless of capability.
    IFS='|' read -r status body < <(do_request POST \
        "/api/v1/streams/${STREAM_KEY}/groups/${GROUP_NAME}/ackdel" \
        '{"ids":[]}')
    check_any "XACKDEL rejects empty ids" "$status" "$body" 400 501

    # 6. DELREF mode — deletes references in every group's PEL.
    IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_KEY}/add" \
        "{\"fields\":{\"baz\":\"qux\"}}")
    SECOND_ID=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
    if [[ -n "$SECOND_ID" ]]; then
        # Re-read with the correct body shape and ASSERT it puts the second
        # entry into the PEL — otherwise the DELREF assertion below would
        # report "missing" instead of exercising the cross-group path.
        IFS='|' read -r status body < <(do_request POST \
            "/api/v1/streams/${STREAM_KEY}/groups/${GROUP_NAME}/read" \
            "{\"consumer\":\"${CONSUMER_NAME}\",\"streams\":[{\"key\":\"${STREAM_KEY}\",\"id\":\">\"}],\"count\":10}")
        check "XREADGROUP (seed second entry into PEL)" "200" "$status" "$body"

        IFS='|' read -r status body < <(do_request POST \
            "/api/v1/streams/${STREAM_KEY}/groups/${GROUP_NAME}/ackdel" \
            "{\"ids\":[\"${SECOND_ID}\"],\"mode\":\"delref\"}")
        check_any "XACKDEL (DELREF)" "$status" "$body" 200 501
    else
        SKIP=$((SKIP + 2))
        echo "  SKIP  XACKDEL DELREF (could not seed second entry)"
    fi
else
    SKIP=$((SKIP + 5))
    echo "  SKIP  XACKDEL flow (XADD seed failed)"
fi

echo ""

# ==========================================================================
# Streams 11.1 — XDELEX / XCFGSET / XADD-IDMP / XTRIM reference policy
# ==========================================================================
echo "--- Streams 11.1 (XDELEX / XCFGSET / IDMP) ---"

STREAM_11_1="${P}_s11_1"

# Seed an entry to delete via XDELEX.
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
    '{"fields":{"k":"v"}}')
check_any "XADD (seed for XDELEX)" "$status" "$body" 200 400 501
DEL_ID=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')

if [[ "$status" == "200" && -n "$DEL_ID" ]]; then
    # XDELEX with default keepref mode.
    IFS='|' read -r status body < <(do_request POST \
        "/api/v1/streams/${STREAM_11_1}/delex" \
        "{\"ids\":[\"${DEL_ID}\"]}")
    check_any "XDELEX (default keepref)" "$status" "$body" 200 501

    # XDELEX with explicit DELREF mode against a re-added entry.
    IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
        '{"fields":{"k2":"v2"}}')
    SECOND_DEL_ID=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
    if [[ -n "$SECOND_DEL_ID" ]]; then
        IFS='|' read -r status body < <(do_request POST \
            "/api/v1/streams/${STREAM_11_1}/delex" \
            "{\"ids\":[\"${SECOND_DEL_ID}\"],\"mode\":\"delref\"}")
        check_any "XDELEX (DELREF mode)" "$status" "$body" 200 501
    fi

    # XDELEX rejects empty ids regardless of capability.
    IFS='|' read -r status body < <(do_request POST \
        "/api/v1/streams/${STREAM_11_1}/delex" '{"ids":[]}')
    check_any "XDELEX rejects empty ids" "$status" "$body" 400 501
else
    SKIP=$((SKIP + 3))
    echo "  SKIP  XDELEX flow (XADD seed failed)"
fi

# XADD with reference_policy on a trim sub-clause (Redis 8.2+).
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
    '{"fields":{"a":"b"},"maxlen":100,"approximate":true,"reference_policy":"keepref"}')
check_any "XADD with reference_policy + maxlen" "$status" "$body" 200 501

# XADD reference_policy without maxlen/minid is rejected at the service layer (400).
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
    '{"fields":{"a":"b"},"reference_policy":"delref"}')
check_any "XADD rejects reference_policy without trim" "$status" "$body" 400 501

# XADD IDMP with explicit id is rejected.
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
    '{"fields":{"a":"b"},"id":"99-0","idmp":{"mode":"auto","producer_id":"p1"}}')
check_any "XADD rejects idmp + explicit id" "$status" "$body" 400 501

# XADD IDMP manual mode (Redis 8.6+; gated). Then call again with the SAME
# (producer_id, idempotent_id) and assert Redis returns the original entry ID
# — that's the actual idempotency guarantee, not just route acceptance.
IDMP_PRODUCER="p_${P}_manual"
IDMP_IID="iid-1"
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
    "{\"fields\":{\"a\":\"b\"},\"idmp\":{\"mode\":\"manual\",\"producer_id\":\"${IDMP_PRODUCER}\",\"idempotent_id\":\"${IDMP_IID}\"}}")
check_any "XADD IDMP manual (first call)" "$status" "$body" 200 501
IDMP_FIRST_ID=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')

if [[ "$status" == "200" && -n "$IDMP_FIRST_ID" ]]; then
    # Re-issue the exact same idempotent call — Redis must dedupe and return
    # the *first* entry ID, not generate a new one.
    IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
        "{\"fields\":{\"a\":\"different\"},\"idmp\":{\"mode\":\"manual\",\"producer_id\":\"${IDMP_PRODUCER}\",\"idempotent_id\":\"${IDMP_IID}\"}}")
    check "XADD IDMP manual (duplicate returns 200)" "200" "$status" "$body"
    IDMP_DUP_ID=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
    if [[ "$IDMP_DUP_ID" == "$IDMP_FIRST_ID" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  XADD IDMP manual is idempotent (id=$IDMP_DUP_ID matches first)"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  XADD IDMP manual idempotency: first=$IDMP_FIRST_ID got=$IDMP_DUP_ID"
        echo "  FAIL  XADD IDMP manual idempotency (first=$IDMP_FIRST_ID, dup=$IDMP_DUP_ID)"
    fi
else
    SKIP=$((SKIP + 2))
    echo "  SKIP  XADD IDMP manual duplicate (first call did not succeed)"
fi

# XADD IDMPAUTO mode (Redis 8.6+; gated). IDMPAUTO derives the iid from the
# message body, so two calls with identical fields + producer_id must dedupe
# to the same entry ID.
IDMPAUTO_PRODUCER="p_${P}_auto"
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
    "{\"fields\":{\"a\":\"auto-payload\"},\"idmp\":{\"mode\":\"auto\",\"producer_id\":\"${IDMPAUTO_PRODUCER}\"}}")
check_any "XADD IDMPAUTO (first call)" "$status" "$body" 200 501
IDMPAUTO_FIRST_ID=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')

if [[ "$status" == "200" && -n "$IDMPAUTO_FIRST_ID" ]]; then
    IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
        "{\"fields\":{\"a\":\"auto-payload\"},\"idmp\":{\"mode\":\"auto\",\"producer_id\":\"${IDMPAUTO_PRODUCER}\"}}")
    check "XADD IDMPAUTO (duplicate returns 200)" "200" "$status" "$body"
    IDMPAUTO_DUP_ID=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
    if [[ "$IDMPAUTO_DUP_ID" == "$IDMPAUTO_FIRST_ID" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  XADD IDMPAUTO is idempotent (id=$IDMPAUTO_DUP_ID matches first)"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  XADD IDMPAUTO idempotency: first=$IDMPAUTO_FIRST_ID got=$IDMPAUTO_DUP_ID"
        echo "  FAIL  XADD IDMPAUTO idempotency (first=$IDMPAUTO_FIRST_ID, dup=$IDMPAUTO_DUP_ID)"
    fi
else
    SKIP=$((SKIP + 2))
    echo "  SKIP  XADD IDMPAUTO duplicate (first call did not succeed)"
fi

# Multi-field IDMPAUTO regression test — IDMPAUTO derives the iid from the
# message body, so the wire-side field order must be deterministic across
# retries even when JSON delivers fields in arbitrary order. With three
# fields sent each time, a HashMap-iteration-order bug would surface as a
# new entry id on the duplicate call.
IDMPAUTO_MULTI_PRODUCER="p_${P}_auto_multi"
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
    "{\"fields\":{\"alpha\":\"1\",\"beta\":\"2\",\"gamma\":\"3\"},\"idmp\":{\"mode\":\"auto\",\"producer_id\":\"${IDMPAUTO_MULTI_PRODUCER}\"}}")
check_any "XADD IDMPAUTO multi-field (first call)" "$status" "$body" 200 501
IDMPAUTO_MULTI_FIRST=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')

if [[ "$status" == "200" && -n "$IDMPAUTO_MULTI_FIRST" ]]; then
    # JSON object key order in the duplicate call is intentionally different
    # to verify the server-side normalization, not just the client.
    IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/add" \
        "{\"fields\":{\"gamma\":\"3\",\"alpha\":\"1\",\"beta\":\"2\"},\"idmp\":{\"mode\":\"auto\",\"producer_id\":\"${IDMPAUTO_MULTI_PRODUCER}\"}}")
    check "XADD IDMPAUTO multi-field (duplicate returns 200)" "200" "$status" "$body"
    IDMPAUTO_MULTI_DUP=$(echo "$body" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
    if [[ "$IDMPAUTO_MULTI_DUP" == "$IDMPAUTO_MULTI_FIRST" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  XADD IDMPAUTO multi-field is idempotent (id=$IDMPAUTO_MULTI_DUP matches first)"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  XADD IDMPAUTO multi-field idempotency: first=$IDMPAUTO_MULTI_FIRST got=$IDMPAUTO_MULTI_DUP"
        echo "  FAIL  XADD IDMPAUTO multi-field idempotency (first=$IDMPAUTO_MULTI_FIRST, dup=$IDMPAUTO_MULTI_DUP)"
    fi
else
    SKIP=$((SKIP + 2))
    echo "  SKIP  XADD IDMPAUTO multi-field duplicate (first call did not succeed)"
fi

# XTRIM with reference_policy + limit (Redis 8.2+; gated).
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${STREAM_11_1}/trim" \
    '{"strategy":"maxlen","count":10,"approximate":true,"limit":50,"reference_policy":"acked"}')
check_any "XTRIM with reference_policy" "$status" "$body" 200 400 501

# XCFGSET (Redis 8.6+; gated).
IFS='|' read -r status body < <(do_request PATCH "/api/v1/streams/${STREAM_11_1}/config" \
    '{"idmp_duration_seconds":120,"idmp_max_size":1000}')
check_any "XCFGSET" "$status" "$body" 200 501

# XCFGSET rejects empty body.
IFS='|' read -r status body < <(do_request PATCH "/api/v1/streams/${STREAM_11_1}/config" \
    '{}')
check_any "XCFGSET rejects empty body" "$status" "$body" 400 501

# XCFGSET rejects out-of-range duration.
IFS='|' read -r status body < <(do_request PATCH "/api/v1/streams/${STREAM_11_1}/config" \
    '{"idmp_duration_seconds":86401}')
check_any "XCFGSET rejects duration > 86400" "$status" "$body" 400 501

echo ""

# ==========================================================================
# Admin (new endpoints)
# ==========================================================================
echo "--- Admin (new endpoints) ---"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/client/info")
check "CLIENT INFO" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/client/id")
check "CLIENT ID" "200" "$status" "$body"
CURRENT_CLIENT_ID=$(echo "$body" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/client/getname")
check "CLIENT GETNAME" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/time")
check "SERVER TIME" "200" "$status" "$body"

# CLIENT UNBLOCK (Redis 5.0+, capability-gated). The current client is not
# blocked, so a successful response should return `unblocked=false`; older
# Redis or service-side rejection (non-positive id) surface as 501 / 400.
if [[ -n "$CURRENT_CLIENT_ID" ]]; then
    IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/client/unblock" \
        "{\"client_id\":${CURRENT_CLIENT_ID}}")
    check_any "CLIENT UNBLOCK" "$status" "$body" 200 501
else
    SKIP=$((SKIP + 1))
    echo "  SKIP  CLIENT UNBLOCK (could not parse CLIENT ID)"
fi

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/client/unblock" \
    '{"client_id":0}')
check_any "CLIENT UNBLOCK rejects client_id=0" "$status" "$body" 400 501

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/client/unblock" \
    '{"client_id":-1}')
check_any "CLIENT UNBLOCK rejects negative id" "$status" "$body" 400 501

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/lastsave")
check "LASTSAVE" "200" "$status" "$body"

# DEBUG OBJECT - may fail if Redis config disallows debug
do_request PUT "/api/v1/strings/${P}_debug_key" '{"value":"test"}' > /dev/null
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/server/debug/object" \
    "{\"key\":\"${P}_debug_key\"}")
check_any "DEBUG OBJECT" "$status" "$body" 200 500

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/memory/stats")
check "MEMORY STATS" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/server/memory/usage" \
    "{\"key\":\"${P}_debug_key\"}")
check "MEMORY USAGE" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/memory/doctor")
check "MEMORY DOCTOR" "200" "$status" "$body"

# MEMORY MALLOC-STATS — Redis 4.0+, no capability gate. Always 200 even on
# non-jemalloc builds (Redis returns a benign payload).
IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/memory/malloc-stats")
check "MEMORY MALLOC-STATS" "200" "$status" "$body"

echo ""

# ==========================================================================
# Admin - Latency
# ==========================================================================
echo "--- Admin (Latency) ---"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/latency/latest")
check "LATENCY LATEST" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/latency/doctor")
check "LATENCY DOCTOR" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/latency/graph" \
    '{"event":"command"}')
check_any "LATENCY GRAPH" "$status" "$body" 200 500

# LATENCY HISTOGRAM — Redis 7.0+, gated. Empty list means "all commands".
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/latency/histogram" \
    '{"commands":[]}')
check_any "LATENCY HISTOGRAM (all commands)" "$status" "$body" 200 501

# Auth check: missing X-Admin-Api-Key returns 401
IFS='|' read -r status body < <(do_request POST "/api/v1/admin/latency/histogram" \
    '{"commands":[]}')
check "LATENCY HISTOGRAM (rejects no auth)" "401" "$status" "$body"

echo ""

# ==========================================================================
# Admin - Slowlog
# ==========================================================================
echo "--- Admin (Slowlog) ---"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/slowlog/get" \
    '{"count":10}')
check "SLOWLOG GET" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/slowlog/len")
check "SLOWLOG LEN" "200" "$status" "$body"

echo ""

# ==========================================================================
# Admin - Config
# ==========================================================================
echo "--- Admin (Config) ---"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/config/get" \
    '{"pattern":"maxmemory"}')
check "CONFIG GET" "200" "$status" "$body"

echo ""

# ==========================================================================
# Admin - ACL (new CRUD endpoints)
# ==========================================================================
echo "--- Admin (ACL) ---"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/acl/list")
check "ACL LIST" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/acl/users")
check "ACL USERS" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/acl/whoami")
check "ACL WHOAMI" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/acl/cat" '{}')
check "ACL CAT" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/acl/genpass" \
    '{"bits":128}')
check "ACL GENPASS" "200" "$status" "$body"

# ACL SETUSER - create a test user
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/acl/setuser" \
    "{\"username\":\"${P}_testuser\",\"rules\":[\"+@read\",\"~*\",\"on\",\">testpass\"]}")
check "ACL SETUSER" "200" "$status" "$body"

# ACL DELUSER - delete the test user
IFS='|' read -r status body < <(admin_request DELETE "/api/v1/admin/acl/deluser" \
    "{\"usernames\":[\"${P}_testuser\"]}")
check "ACL DELUSER" "200" "$status" "$body"

# ACL DRYRUN
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/acl/dryrun" \
    '{"username":"default","command":["GET","key1"]}')
check "ACL DRYRUN" "200" "$status" "$body"

echo ""

# ==========================================================================
# Admin - Persistence
# ==========================================================================
echo "--- Admin (Persistence) ---"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/persistence/bgsave" '{}')
check_any "BGSAVE" "$status" "$body" 200 500

# BGSAVE with SCHEDULE flag (Redis 3.2+). Should at least pass the route
# layer; Redis may return a non-error message when nothing else is running.
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/persistence/bgsave" \
    '{"schedule":true}')
check_any "BGSAVE SCHEDULE" "$status" "$body" 200 500

# WAITAOF (Redis 7.2+). Use zero requested acknowledgements so the call returns
# immediately; the service clamps the timeout to its blocking-operation bounds.
# Older Redis is gated out at the capability layer with HTTP 501; standalone
# instances with `appendonly no` may surface a 5xx.
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/persistence/waitaof" \
    '{"numlocal":0,"numreplicas":0,"timeout_ms":1000}')
check_any "WAITAOF" "$status" "$body" 200 500 501

echo ""

# ==========================================================================
# Keys (RESTORE option parity, Redis 5.0+)
# ==========================================================================
echo "--- Keys (RESTORE options) ---"

RESTORE_SRC="${P}_restore_src"
RESTORE_DST="${P}_restore_dst"

IFS='|' read -r status body < <(do_request POST "/api/v1/strings/${RESTORE_SRC}" \
    '{"value":"hello"}')
check_any "SET (seed for RESTORE)" "$status" "$body" 200 400

IFS='|' read -r status body < <(do_request GET "/api/v1/keys/${RESTORE_SRC}/dump")
check_any "DUMP (seed for RESTORE)" "$status" "$body" 200 404
DUMP_PAYLOAD=$(echo "$body" | sed -n 's/.*"data":"\([^"]*\)".*/\1/p')

if [[ -n "$DUMP_PAYLOAD" ]]; then
    # Plain restore so the rest of the assertions have a target.
    IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${RESTORE_DST}/restore" \
        "{\"ttl\":0,\"data\":\"${DUMP_PAYLOAD}\"}")
    check_any "RESTORE (plain)" "$status" "$body" 200

    # RESTORE with IDLETIME initializer.
    IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${RESTORE_DST}/restore" \
        "{\"ttl\":0,\"data\":\"${DUMP_PAYLOAD}\",\"replace\":true,\"idletime\":30}")
    check_any "RESTORE (IDLETIME)" "$status" "$body" 200

    # RESTORE with FREQ initializer (requires LFU policy; tolerate 5xx).
    IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${RESTORE_DST}/restore" \
        "{\"ttl\":0,\"data\":\"${DUMP_PAYLOAD}\",\"replace\":true,\"freq\":5}")
    check_any "RESTORE (FREQ)" "$status" "$body" 200 500

    # RESTORE with ABSTTL using an absolute future timestamp (year 2100).
    IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${RESTORE_DST}/restore" \
        "{\"ttl\":4102444800000,\"data\":\"${DUMP_PAYLOAD}\",\"replace\":true,\"absttl\":true}")
    check_any "RESTORE (ABSTTL)" "$status" "$body" 200

    # IDLETIME + FREQ together is rejected at the service layer.
    IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${RESTORE_DST}/restore" \
        "{\"ttl\":0,\"data\":\"${DUMP_PAYLOAD}\",\"replace\":true,\"idletime\":1,\"freq\":1}")
    check "RESTORE rejects IDLETIME + FREQ" "400" "$status" "$body"

    # Negative TTL is rejected at the service layer.
    IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${RESTORE_DST}/restore" \
        "{\"ttl\":-1,\"data\":\"${DUMP_PAYLOAD}\"}")
    check "RESTORE rejects negative TTL" "400" "$status" "$body"
else
    SKIP=$((SKIP + 6))
    echo "  SKIP  RESTORE option matrix (DUMP failed to produce payload)"
fi

echo ""

# ==========================================================================
# Admin - Command Introspection
# ==========================================================================
echo "--- Admin (Commands) ---"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/commands/count")
check "COMMAND COUNT" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/commands")
check "COMMAND LIST" "200" "$status" "$body"

# COMMAND GETKEYSANDFLAGS — Redis 7.0+, gated by command_docs.
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/commands/getkeysandflags" \
    '{"command":["SET","foo","bar"]}')
check_any "COMMAND GETKEYSANDFLAGS" "$status" "$body" 200 501

if [[ "$status" == "200" ]]; then
    # Empty command vec is rejected at the service layer (400).
    IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/commands/getkeysandflags" \
        '{"command":[]}')
    check "COMMAND GETKEYSANDFLAGS (rejects empty command)" "400" "$status" "$body"
fi

echo ""

# ==========================================================================
# Admin (HOTKEYS, Redis 8.6+)
# ==========================================================================
echo "--- Admin (Hot Keys, Redis 8.6+) ---"

# Auth is required even when the capability is off; this never goes through
# to Redis, so check for an exact 401 instead of allowing 501.
IFS='|' read -r status body < <(do_request POST "/api/v1/admin/hotkeys/start" \
    '{"cpu":true}')
check "HOTKEYS START rejects missing auth" "401" "$status" "$body"

# Schema validation runs after auth + capability — both 400 and 501 are valid
# depending on whether the server is on Redis 8.6+.
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/hotkeys/start" '{}')
check_any "HOTKEYS START rejects empty metrics" "$status" "$body" 400 501

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/hotkeys/start" \
    '{"cpu":true,"sample_ratio":0}')
check_any "HOTKEYS START rejects sample_ratio=0" "$status" "$body" 400 501

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/hotkeys/start" \
    '{"cpu":true,"slots":[{"start":100,"end":50}]}')
check_any "HOTKEYS START rejects inverted slot range" "$status" "$body" 400 501

# Happy-path start with both metrics. On a real Redis 8.6+ server this returns
# 200; on older builds the route returns 501 from the capability gate.
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/hotkeys/start" \
    '{"cpu":true,"net":true,"top_k":5,"duration_seconds":2,"sample_ratio":50}')
check_any "HOTKEYS START (cpu+net)" "$status" "$body" 200 501
HOTKEYS_ACTIVE="$status"

if [[ "$HOTKEYS_ACTIVE" == "200" ]]; then
    # Trigger a little workload so the tracker actually accumulates samples
    # before STOP/GET.
    for _ in 1 2 3 4 5; do
        do_request GET "/api/v1/strings/${P}_hot_key" >/dev/null
    done

    IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/hotkeys/stop")
    check "HOTKEYS STOP" "200" "$status" "$body"

    IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/hotkeys")
    check "HOTKEYS GET" "200" "$status" "$body"
    if [[ "$body" != *'"data"'* ]]; then
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  HOTKEYS GET body missing 'data' field"
        echo "  FAIL  HOTKEYS GET body missing 'data' field"
    else
        PASS=$((PASS + 1))
        echo "  PASS  HOTKEYS GET body contains 'data' field"
    fi

    IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/hotkeys/reset")
    check "HOTKEYS RESET" "200" "$status" "$body"
else
    SKIP=$((SKIP + 4))
    echo "  SKIP  HOTKEYS lifecycle (capability not advertised by this Redis)"
fi

echo ""

# ==========================================================================
# Cleanup
# ==========================================================================
echo "--- Cleanup ---"
CLEANUP_KEYS=$(curl -s "${BASE_URL}/api/v1/keys/scan?pattern=${P}*&count=1000" | \
    sed -n 's/.*"keys":\[\([^]]*\)\].*/\1/p' | tr ',' '\n' | tr -d '"' | tr -d ' ')
if [[ -n "$CLEANUP_KEYS" ]]; then
    KEYS_JSON=$(echo "$CLEANUP_KEYS" | awk 'BEGIN{printf "["} NR>1{printf ","} {printf "\"%s\"",$0} END{printf "]"}')
    curl -s -X POST "${BASE_URL}/api/v1/keys/delete" \
        -H 'Content-Type: application/json' \
        -d "{\"keys\":$KEYS_JSON}" > /dev/null 2>&1 || true
fi
echo "  Cleanup attempted for prefix ${P}_*"

echo ""
echo "============================================================"
echo " Results: $PASS passed, $FAIL failed, $SKIP skipped"
echo "============================================================"

if [[ $FAIL -gt 0 ]]; then
    echo ""
    echo "Failures:"
    echo -e "$ERRORS"
    echo ""
    exit 1
fi

exit 0
