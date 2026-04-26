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
# Admin (new endpoints)
# ==========================================================================
echo "--- Admin (new endpoints) ---"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/client/info")
check "CLIENT INFO" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/client/id")
check "CLIENT ID" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/client/getname")
check "CLIENT GETNAME" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/time")
check "SERVER TIME" "200" "$status" "$body"

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
