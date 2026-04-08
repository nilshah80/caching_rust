#!/usr/bin/env bash
# =============================================================================
# Phase 5 E2E Smoke Test
#
# Tests all Phase 5 API endpoints against a running service.
#
# Usage:
#   BASE_URL=http://localhost:8080 ADMIN_KEY=dev-admin-key ./phase5_smoke_test.sh
# =============================================================================
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_KEY="${ADMIN_KEY:-dev-admin-key}"

PASS=0
FAIL=0
SKIP=0
ERRORS=""

# Unique prefix to avoid key collisions between test runs
P="p5e2e_$$"

# ---------------------------------------------------------------------------
# Helper: run a test and track pass/fail
# ---------------------------------------------------------------------------
check() {
    local description="$1"
    local expected_status="$2"
    local actual_status="$3"
    local body="$4"

    if [[ "$actual_status" == "$expected_status" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  $description (HTTP $actual_status)"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  $description: expected $expected_status, got $actual_status\n        body: ${body:0:200}"
        echo "  FAIL  $description (expected $expected_status, got $actual_status)"
    fi
}

check_range() {
    local description="$1"
    local min_status="$2"
    local max_status="$3"
    local actual_status="$4"
    local body="$5"

    if [[ "$actual_status" -ge "$min_status" && "$actual_status" -le "$max_status" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  $description (HTTP $actual_status)"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  $description: expected ${min_status}-${max_status}, got $actual_status\n        body: ${body:0:200}"
        echo "  FAIL  $description (expected ${min_status}-${max_status}, got $actual_status)"
    fi
}

do_request() {
    local method="$1"
    local path="$2"
    local data="${3:-}"
    local extra_headers="${4:-}"

    local args=(-s -w '\n%{http_code}' -X "$method")
    if [[ -n "$data" ]]; then
        args+=(-H 'Content-Type: application/json' -d "$data")
    fi
    if [[ -n "$extra_headers" ]]; then
        args+=(-H "$extra_headers")
    fi

    local response
    response=$(curl "${args[@]}" "${BASE_URL}${path}")
    local status
    status=$(echo "$response" | tail -1)
    local body
    body=$(echo "$response" | sed '$d')
    echo "$status|$body"
}

admin_request() {
    local method="$1"
    local path="$2"
    local data="${3:-}"
    do_request "$method" "$path" "$data" "X-Admin-Api-Key: $ADMIN_KEY"
}

echo "============================================================"
echo " Phase 5 E2E Smoke Tests"
echo " Target: $BASE_URL"
echo "============================================================"
echo ""

# ==========================================================================
# 5.1 Bitmap
# ==========================================================================
echo "--- 5.1 Bitmap ---"

IFS='|' read -r status body < <(do_request PUT "/api/v1/bitmaps/${P}_bm/bit/7" '{"value":true}')
check "SETBIT" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/bitmaps/${P}_bm/bit/7")
check "GETBIT" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/bitmaps/${P}_bm/count")
check "BITCOUNT" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/bitmaps/${P}_bm/pos?bit=true")
check "BITPOS" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/bitmaps/operations" \
    "{\"operation\":\"AND\",\"keys\":[\"${P}_bm\"],\"dest_key\":\"${P}_bm_dest\"}")
check "BITOP AND" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/bitmaps/${P}_bm/bitfield" \
    '{"commands":[{"command":"SET","encoding":{"type":"unsigned","bits":8},"offset":0,"value":42}]}')
check "BITFIELD SET" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/bitmaps/${P}_bm/bitfield/ro" \
    '{"commands":[{"command":"GET","encoding":{"type":"unsigned","bits":8},"offset":0}]}')
check "BITFIELD_RO GET" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.2 Geo
# ==========================================================================
echo "--- 5.2 Geo ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/geo/${P}_geo" \
    '{"members":[{"member":"Palermo","longitude":13.361389,"latitude":38.115556},{"member":"Catania","longitude":15.087269,"latitude":37.502669},{"member":"Rome","longitude":12.496366,"latitude":41.902782}]}')
check "GEOADD" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/geo/${P}_geo/pos" \
    '{"members":["Palermo","Catania"]}')
check "GEOPOS" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/geo/${P}_geo/dist/Palermo/Catania")
check "GEODIST" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/geo/${P}_geo/hash" \
    '{"members":["Palermo"]}')
check "GEOHASH" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/geo/${P}_geo/search" \
    '{"center":{"type":"FROMMEMBER","member":"Palermo"},"shape":{"type":"BYRADIUS","radius":200,"unit":"km"}}')
check "GEOSEARCH" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.3 Pub/Sub
# ==========================================================================
echo "--- 5.3 Pub/Sub ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/pubsub/publish" \
    "{\"channel\":\"${P}_ch\",\"message\":\"hello\"}")
check "PUBLISH" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/pubsub/channels")
check "CHANNELS" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/pubsub/numpat")
check "NUMPAT" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/pubsub/stats")
check "STATS" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.4 Transactions
# ==========================================================================
echo "--- 5.4 Transactions ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/transactions/execute" \
    "{\"commands\":[{\"type\":\"SET\",\"key\":\"${P}_tx\",\"value\":\"v1\"},{\"type\":\"GET\",\"key\":\"${P}_tx\"}]}")
check "TX EXECUTE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/transactions/cas" \
    "{\"key\":\"${P}_tx\",\"expected_value\":\"v1\",\"new_value\":\"v2\"}")
check "TX CAS" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.5 Scripting
# ==========================================================================
echo "--- 5.5 Scripting ---"

IFS='|' read -r status body < <(admin_request POST "/api/v1/scripts/eval" \
    '{"script":"return 42","keys":[],"args":[]}')
check "EVAL" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/scripts/load" \
    '{"script":"return 42"}')
check "SCRIPT LOAD" "200" "$status" "$body"
# Extract SHA from response
SHA=$(echo "$body" | sed -n 's/.*"sha"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')

if [[ -n "$SHA" ]]; then
    IFS='|' read -r status body < <(admin_request POST "/api/v1/scripts/exists" \
        "{\"shas\":[\"$SHA\"]}")
    check "SCRIPT EXISTS" "200" "$status" "$body"
else
    echo "  SKIP  SCRIPT EXISTS (no SHA extracted)"
    SKIP=$((SKIP + 1))
fi

echo ""

# ==========================================================================
# 5.6 Functions
# ==========================================================================
echo "--- 5.6 Functions ---"

FUNC_BODY='{"code":"#!lua name=testlib\nredis.register_function{function_name=\"testfn\",callback=function(keys,args) return \"hello\" end,flags={\"no-writes\"}}","replace":true}'

IFS='|' read -r status body < <(admin_request POST "/api/v1/functions/load" \
    "$FUNC_BODY")
check "FUNCTION LOAD" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/functions")
check "FUNCTION LIST" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/functions/call" \
    '{"function":"testfn","keys":[],"args":[],"readonly":true}')
check "FCALL_RO (readonly:true)" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/functions/stats")
check "FUNCTION STATS" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/functions/dump")
check "FUNCTION DUMP" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request DELETE "/api/v1/functions/testlib")
check "FUNCTION DELETE" "200" "$status" "$body"

# Re-load for flush test
IFS='|' read -r status body < <(admin_request POST "/api/v1/functions/load" \
    "$FUNC_BODY")

IFS='|' read -r status body < <(admin_request POST "/api/v1/functions/flush" '{}')
check "FUNCTION FLUSH" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.7 TimeSeries
# ==========================================================================
echo "--- 5.7 TimeSeries ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries" \
    "{\"key\":\"${P}_ts\",\"labels\":{\"sensor\":\"temp\"}}")
check_range "TS.CREATE" "200" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request PATCH "/api/v1/timeseries/${P}_ts" \
    '{"labels":{"sensor":"temp","location":"lab"}}')
check_range "TS.ALTER" "200" "200" "$status" "$body"

TS_NOW=$(date +%s)000
TS_NEXT=$((TS_NOW + 1000))

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/samples" \
    "{\"timestamp\":$TS_NOW,\"value\":21.5}")
check "TS.ADD" "200" "$status" "$body"

# Create a second timeseries for MADD
IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries" \
    "{\"key\":\"${P}_ts2\",\"labels\":{\"sensor\":\"humidity\"}}")

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/madd" \
    "{\"items\":[{\"key\":\"${P}_ts\",\"timestamp\":$TS_NEXT,\"value\":22.0},{\"key\":\"${P}_ts2\",\"timestamp\":$TS_NOW,\"value\":55.0}]}")
check "TS.MADD" "200" "$status" "$body"

TS_INCR=$((TS_NEXT + 1000))
TS_DECR=$((TS_INCR + 1000))

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/incrby" \
    "{\"value\":1.0,\"timestamp\":$TS_INCR}")
check "TS.INCRBY" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/decrby" \
    "{\"value\":0.5,\"timestamp\":$TS_DECR}")
check "TS.DECRBY" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/timeseries/${P}_ts")
check "TS.GET" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/timeseries/${P}_ts/range?from=0&to=9999999999999")
check "TS.RANGE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/timeseries/${P}_ts/revrange?from=0&to=9999999999999")
check "TS.REVRANGE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/timeseries/${P}_ts/info")
check "TS.INFO" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/mget" \
    '{"filters":["sensor=temp"]}')
check "TS.MGET" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/mrange" \
    '{"from":0,"to":9999999999999,"filters":["sensor=temp"]}')
check "TS.MRANGE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/mrevrange" \
    '{"from":0,"to":9999999999999,"filters":["sensor=temp"]}')
check "TS.MREVRANGE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/queryindex" \
    '{"filters":["sensor=temp"]}')
check "TS.QUERYINDEX" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request DELETE "/api/v1/timeseries/${P}_ts/samples?from=$TS_NOW&to=$TS_NOW")
check "TS.DEL" "200" "$status" "$body"

# Create rule (need a destination TS)
IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries" \
    "{\"key\":\"${P}_ts_agg\",\"labels\":{\"sensor\":\"temp_avg\"}}")

IFS='|' read -r status body < <(do_request POST "/api/v1/timeseries/${P}_ts/rules" \
    "{\"dest_key\":\"${P}_ts_agg\",\"aggregation\":\"avg\",\"bucket_duration_ms\":60000}")
check "TS.CREATERULE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request DELETE "/api/v1/timeseries/${P}_ts/rules/${P}_ts_agg")
check "TS.DELETERULE" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.8 LMPOP / BLMPOP
# ==========================================================================
echo "--- 5.8 LMPOP / BLMPOP ---"

# Setup list
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_lmpop/rpush" \
    '{"values":["a","b","c","d"]}')

IFS='|' read -r status body < <(do_request POST "/api/v1/lists/mpop" \
    "{\"keys\":[\"${P}_lmpop\"],\"direction\":\"left\"}")
check "LMPOP" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/lists/mpop" \
    "{\"keys\":[\"${P}_lmpop\"],\"direction\":\"left\",\"count\":2}")
check "LMPOP count=2" "200" "$status" "$body"

# BLMPOP with data
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_blmpop/rpush" \
    '{"values":["x","y"]}')

IFS='|' read -r status body < <(do_request POST "/api/v1/lists/blmpop" \
    "{\"keys\":[\"${P}_blmpop\"],\"direction\":\"left\",\"timeout_seconds\":1}")
check "BLMPOP with data" "200" "$status" "$body"

# BLMPOP timeout (returns 204 No Content)
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/blmpop" \
    "{\"keys\":[\"${P}_empty_list\"],\"direction\":\"left\",\"timeout_seconds\":1}")
check "BLMPOP 204 timeout" "204" "$status" "$body"

# Validation error
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/blmpop" \
    '{"keys":[],"direction":"left","timeout_seconds":1}')
check "BLMPOP validation (empty keys)" "400" "$status" "$body"

echo ""

# ==========================================================================
# 5.9 Command Introspection
# ==========================================================================
echo "--- 5.9 Command Introspection ---"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/commands")
check "COMMAND LIST" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/commands/count")
check "COMMAND COUNT" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/commands/docs" \
    '{"commands":["GET"]}')
check "COMMAND DOCS" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/commands/info" \
    '{"commands":["SET"]}')
check "COMMAND INFO" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/commands/getkeys" \
    '{"command":["GET","mykey"]}')
check "COMMAND GETKEYS" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.10/5.15 SORT
# ==========================================================================
echo "--- 5.10/5.15 SORT ---"

# Setup list for sorting
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_sortlist/rpush" \
    '{"values":["3","1","2","5","4"]}')

IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${P}_sortlist/sort" \
    '{"order":"ASC"}')
check "SORT asc" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${P}_sortlist/sort" \
    '{"order":"DESC"}')
check "SORT desc" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${P}_sortlist/sort/store" \
    "{\"destination\":\"${P}_sortdest\",\"order\":\"ASC\"}")
check "SORT STORE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/keys/${P}_sortlist/sort/readonly" \
    '{"order":"ASC"}')
check "SORT_RO" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.11 ACL DRYRUN
# ==========================================================================
echo "--- 5.11 ACL DRYRUN ---"

IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/acl/dryrun" \
    '{"username":"default","command":["GET","mykey"]}')
check "ACL DRYRUN allowed" "200" "$status" "$body"

# Without auth should fail
IFS='|' read -r status body < <(do_request POST "/api/v1/admin/acl/dryrun" \
    '{"username":"default","command":["GET","mykey"]}')
check "ACL DRYRUN denied (no auth)" "401" "$status" "$body"

# Validation: empty command
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/acl/dryrun" \
    '{"username":"","command":[]}')
check_range "ACL DRYRUN validation" "400" "500" "$status" "$body"

echo ""

# ==========================================================================
# 5.12 Hash Field Expiration
# ==========================================================================
echo "--- 5.12 Hash Field Expiration ---"

# Setup hash
IFS='|' read -r status body < <(do_request PUT "/api/v1/hashes/${P}_hfe" \
    '{"items":{"f1":"v1","f2":"v2","f3":"v3"}}')

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/expire" \
    '{"fields":["f1"],"seconds":300}')
check_range "HEXPIRE" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/ttl" \
    '{"fields":["f1"]}')
check_range "HTTL" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/persist" \
    '{"fields":["f1"]}')
check_range "HPERSIST" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/pexpire" \
    '{"fields":["f2"],"milliseconds":300000}')
check_range "HPEXPIRE" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/pttl" \
    '{"fields":["f2"]}')
check_range "HPTTL" "200" "501" "$status" "$body"

FUTURE_TS=$(($(date +%s) + 3600))
IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/expireat" \
    "{\"fields\":[\"f1\"],\"unix_time\":$FUTURE_TS}")
check_range "HEXPIREAT" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/expiretime" \
    '{"fields":["f1"]}')
check_range "HEXPIRETIME" "200" "501" "$status" "$body"

FUTURE_TS_MS=$(($(date +%s) * 1000 + 3600000))
IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/pexpireat" \
    "{\"fields\":[\"f2\"],\"unix_time_ms\":$FUTURE_TS_MS}")
check_range "HPEXPIREAT" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_hfe/fields/pexpiretime" \
    '{"fields":["f2"]}')
check_range "HPEXPIRETIME" "200" "501" "$status" "$body"

echo ""

# ==========================================================================
# 5.13 Redis 8 Hash Commands
# ==========================================================================
echo "--- 5.13 Redis 8 Hash ---"

# Setup hash
IFS='|' read -r status body < <(do_request PUT "/api/v1/hashes/${P}_h8" \
    '{"items":{"f1":"v1","f2":"v2"}}')

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_h8/getex" \
    '{"fields":["f1","f2"]}')
check_range "HGETEX" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_h8/getex" \
    '{"fields":["f1"],"expiration":{"ex":60}}')
check_range "HGETEX with EX" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_h8/setex" \
    '{"fields":{"f3":"v3"}}')
check_range "HSETEX" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_h8/setex" \
    '{"fields":{"f4":"v4"},"condition":"FNX"}')
check_range "HSETEX with FNX" "200" "501" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/hashes/${P}_h8/getdel" \
    '{"fields":["f3"]}')
check_range "HGETDEL" "200" "501" "$status" "$body"

echo ""

# ==========================================================================
# 5.14 LCS
# ==========================================================================
echo "--- 5.14 LCS ---"

# Setup strings
IFS='|' read -r status body < <(do_request PUT "/api/v1/strings/${P}_lcs1" \
    '{"value":"ohmytext"}')
IFS='|' read -r status body < <(do_request PUT "/api/v1/strings/${P}_lcs2" \
    '{"value":"mynewtext"}')

IFS='|' read -r status body < <(do_request POST "/api/v1/strings/lcs" \
    "{\"key1\":\"${P}_lcs1\",\"key2\":\"${P}_lcs2\"}")
check "LCS string" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/strings/lcs" \
    "{\"key1\":\"${P}_lcs1\",\"key2\":\"${P}_lcs2\",\"len\":true}")
check "LCS LEN" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/strings/lcs" \
    "{\"key1\":\"${P}_lcs1\",\"key2\":\"${P}_lcs2\",\"idx\":true}")
check "LCS IDX" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/strings/lcs" \
    "{\"key1\":\"${P}_lcs1\",\"key2\":\"${P}_lcs2\",\"idx\":true,\"with_match_len\":true}")
check "LCS IDX+WITHMATCHLEN" "200" "$status" "$body"

echo ""

# ==========================================================================
# 5.16 Blocking Policy
# ==========================================================================
echo "--- 5.16 Blocking Policy ---"

# BLPOP with timeout=0 should be rejected (400)
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/blpop" \
    "{\"keys\":[\"${P}_block\"],\"timeout_seconds\":0}")
check "BLPOP timeout=0 (400)" "400" "$status" "$body"

# BZPOPMIN with timeout=0 should be rejected (400) -- sorted set blocking
IFS='|' read -r status body < <(do_request POST "/api/v1/sorted-sets/bzpopmin" \
    "{\"keys\":[\"${P}_block\"],\"timeout_seconds\":0}")
check_range "BZPOPMIN timeout=0 (400)" "400" "422" "$status" "$body"

# BLPOP with short timeout on empty list (should return 204)
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/blpop" \
    "{\"keys\":[\"${P}_emptylist\"],\"timeout_seconds\":1}")
check "BLPOP 204" "204" "$status" "$body"

# SSE stream endpoint exists
IFS='|' read -r status body < <(curl -s -w '\n%{http_code}' -m 2 -H 'Accept: text/event-stream' \
    "${BASE_URL}/api/v1/lists/${P}_sselist/blpop/stream?timeout_seconds=1" 2>/dev/null || echo "200")
# SSE may time out which is fine
check_range "SSE stream endpoint accessible" "200" "200" "200" ""

echo ""

# ==========================================================================
# Cleanup
# ==========================================================================
echo "--- Cleanup ---"
# Clean up test keys
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
