"""
Redis Caching Service - Python Client Examples

Requirements:
    pip install requests websockets

Usage:
    python python_client.py
"""

import json
import threading
import time

import requests

BASE_URL = "http://localhost:8080"
ADMIN_KEY = "dev-admin-key"


def admin_headers():
    return {"X-Admin-Api-Key": ADMIN_KEY, "Content-Type": "application/json"}


def json_headers():
    return {"Content-Type": "application/json"}


# ---------------------------------------------------------------------------
# Health
# ---------------------------------------------------------------------------

def check_health():
    print("=== Health Check ===")
    r = requests.get(f"{BASE_URL}/health")
    print(f"  Status: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/health/ready")
    print(f"  Ready:  {r.status_code}")


# ---------------------------------------------------------------------------
# Strings
# ---------------------------------------------------------------------------

def string_operations():
    print("\n=== String Operations ===")

    r = requests.put(
        f"{BASE_URL}/api/v1/strings/greeting",
        headers=json_headers(),
        json={"value": "Hello, World!", "ttl_seconds": 3600},
    )
    print(f"  SET: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/strings/greeting")
    print(f"  GET: {r.status_code} {r.json()}")

    r = requests.post(
        f"{BASE_URL}/api/v1/strings/mset",
        headers=json_headers(),
        json={"pairs": {"counter": "0", "name": "Alice", "role": "admin"}},
    )
    print(f"  MSET: {r.status_code}")

    r = requests.post(
        f"{BASE_URL}/api/v1/strings/mget",
        headers=json_headers(),
        json={"keys": ["counter", "name", "role", "missing"]},
    )
    print(f"  MGET: {r.status_code} {r.json()}")

    r = requests.patch(
        f"{BASE_URL}/api/v1/strings/counter/incr",
        headers=json_headers(),
        json={"delta": 5},
    )
    print(f"  INCR: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Hashes
# ---------------------------------------------------------------------------

def hash_operations():
    print("\n=== Hash Operations ===")

    r = requests.put(
        f"{BASE_URL}/api/v1/hashes/user:100",
        headers=json_headers(),
        json={"items": {"name": "Alice", "email": "alice@example.com", "age": "30"}},
    )
    print(f"  HSET: {r.status_code}")

    r = requests.get(f"{BASE_URL}/api/v1/hashes/user:100")
    print(f"  HGETALL: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/hashes/user:100/fields/name")
    print(f"  HGET name: {r.status_code} {r.json()}")

    r = requests.patch(
        f"{BASE_URL}/api/v1/hashes/user:100/fields/age/incr",
        headers=json_headers(),
        json={"delta": 1},
    )
    print(f"  HINCRBY age: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Lists
# ---------------------------------------------------------------------------

def list_operations():
    print("\n=== List Operations ===")

    r = requests.post(
        f"{BASE_URL}/api/v1/lists/tasks/rpush",
        headers=json_headers(),
        json={"values": ["task1", "task2", "task3"]},
    )
    print(f"  RPUSH: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/lists/tasks/range?start=0&stop=-1")
    print(f"  LRANGE: {r.status_code} {r.json()}")

    r = requests.post(
        f"{BASE_URL}/api/v1/lists/tasks/lpop",
        headers=json_headers(),
        json={"count": 1},
    )
    print(f"  LPOP: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Sets
# ---------------------------------------------------------------------------

def set_operations():
    print("\n=== Set Operations ===")

    r = requests.post(
        f"{BASE_URL}/api/v1/sets/tags:post1/members",
        headers=json_headers(),
        json={"members": ["python", "rust", "redis"]},
    )
    print(f"  SADD: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/sets/tags:post1/members")
    print(f"  SMEMBERS: {r.status_code} {r.json()}")

    r = requests.post(
        f"{BASE_URL}/api/v1/sets/tags:post1/ismember",
        headers=json_headers(),
        json={"member": "rust"},
    )
    print(f"  SISMEMBER: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Sorted Sets
# ---------------------------------------------------------------------------

def sorted_set_operations():
    print("\n=== Sorted Set Operations ===")

    r = requests.post(
        f"{BASE_URL}/api/v1/sorted-sets/leaderboard/members",
        headers=json_headers(),
        json={"members": [
            {"member": "alice", "score": 100},
            {"member": "bob", "score": 85},
            {"member": "charlie", "score": 92},
        ]},
    )
    print(f"  ZADD: {r.status_code} {r.json()}")

    r = requests.get(
        f"{BASE_URL}/api/v1/sorted-sets/leaderboard/range?start=0&stop=-1&rev=true&with_scores=true"
    )
    print(f"  ZRANGE (rev): {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/sorted-sets/leaderboard/rank/alice")
    print(f"  ZRANK alice: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Keys
# ---------------------------------------------------------------------------

def key_operations():
    print("\n=== Key Operations ===")

    r = requests.post(
        f"{BASE_URL}/api/v1/keys/exists",
        headers=json_headers(),
        json={"keys": ["greeting", "user:100", "nonexistent"]},
    )
    print(f"  EXISTS: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/keys/scan?pattern=*&count=20")
    print(f"  SCAN: {r.status_code} {r.json()}")

    r = requests.patch(
        f"{BASE_URL}/api/v1/keys/greeting/expire",
        headers=json_headers(),
        json={"seconds": 60},
    )
    print(f"  EXPIRE: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/keys/greeting/ttl")
    print(f"  TTL: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Transactions
# ---------------------------------------------------------------------------

def transaction_operations():
    print("\n=== Transaction Operations ===")

    requests.put(
        f"{BASE_URL}/api/v1/strings/balance",
        headers=json_headers(),
        json={"value": "100"},
    )

    r = requests.post(
        f"{BASE_URL}/api/v1/transactions/execute",
        headers=json_headers(),
        json={
            "commands": [
                {"type": "GET", "key": "balance"},
                {"type": "SET", "key": "balance", "value": "150"},
            ]
        },
    )
    print(f"  MULTI/EXEC: {r.status_code} {r.json()}")

    r = requests.post(
        f"{BASE_URL}/api/v1/transactions/cas",
        headers=json_headers(),
        json={"key": "balance", "expected_value": "150", "new_value": "200"},
    )
    print(f"  CAS: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Pub/Sub (publish only - subscribe requires WebSocket)
# ---------------------------------------------------------------------------

def pubsub_operations():
    print("\n=== Pub/Sub Operations ===")

    r = requests.post(
        f"{BASE_URL}/api/v1/pubsub/publish",
        headers=json_headers(),
        json={"channel": "notifications", "message": "Hello from Python!"},
    )
    print(f"  PUBLISH: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/pubsub/stats")
    print(f"  STATS: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Admin
# ---------------------------------------------------------------------------

def admin_operations():
    print("\n=== Admin Operations ===")

    r = requests.get(f"{BASE_URL}/api/v1/admin/pool/stats")
    print(f"  Pool Stats: {r.status_code} {r.json()}")

    r = requests.get(f"{BASE_URL}/api/v1/admin/capabilities")
    print(f"  Capabilities: {r.status_code} {r.json()}")

    r = requests.get(
        f"{BASE_URL}/api/v1/admin/server/info?section=memory",
        headers=admin_headers(),
    )
    print(f"  Server Info (memory): {r.status_code} (truncated)")

    r = requests.get(
        f"{BASE_URL}/api/v1/admin/server/dbsize",
        headers=admin_headers(),
    )
    print(f"  DB Size: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# WebSocket Pub/Sub Example (async)
# ---------------------------------------------------------------------------

async def pubsub_subscribe_example():
    """Subscribes to a channel via WebSocket and prints messages."""
    import asyncio
    import websockets

    print("\n=== WebSocket Pub/Sub (subscribe for 5s) ===")

    uri = f"ws://localhost:8080/api/v1/pubsub/subscribe?channels=notifications"

    async def subscriber():
        async with websockets.connect(uri) as ws:
            try:
                while True:
                    msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                    data = json.loads(msg)
                    print(f"  Received: [{data.get('channel')}] {data.get('message')}")
            except asyncio.TimeoutError:
                print("  (no more messages, closing)")

    async def publisher():
        await asyncio.sleep(1)
        for i in range(3):
            requests.post(
                f"{BASE_URL}/api/v1/pubsub/publish",
                headers=json_headers(),
                json={"channel": "notifications", "message": f"Message #{i+1}"},
            )
            await asyncio.sleep(0.5)

    await asyncio.gather(subscriber(), publisher())


# ---------------------------------------------------------------------------
# SSE Stream Subscribe Example
# ---------------------------------------------------------------------------

def sse_stream_example():
    """Subscribe to a Redis stream via SSE and print entries."""
    print("\n=== SSE Stream Subscribe (3 entries) ===")

    def publisher():
        time.sleep(0.5)
        for i in range(3):
            requests.post(
                f"{BASE_URL}/api/v1/streams/events/add",
                headers=json_headers(),
                json={"fields": {"event": "click", "page": f"/page/{i}"}},
                timeout=5,
            )
            time.sleep(0.2)

    publisher_thread = threading.Thread(target=publisher, daemon=True)
    publisher_thread.start()

    response = requests.get(
        f"{BASE_URL}/api/v1/streams/events/subscribe?last_id=0",
        stream=True,
        timeout=(3, 15),
    )
    response.raise_for_status()

    event_name = None
    received = 0
    try:
        for raw_line in response.iter_lines(decode_unicode=True):
            if raw_line is None:
                continue

            line = raw_line.strip()
            if not line:
                continue

            if line.startswith("event:"):
                event_name = line.split(":", 1)[1].strip()
                continue

            if line.startswith("data:"):
                payload = line.split(":", 1)[1].strip()
                if event_name == "message":
                    entry = json.loads(payload)
                    print(f"  SSE entry {received + 1}: {entry['id']} {entry['fields']}")
                    received += 1
                    if received == 3:
                        break
                elif event_name == "error":
                    print(f"  SSE error: {payload}")
                    break
    finally:
        response.close()
        publisher_thread.join(timeout=1)


# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------

def cleanup():
    print("\n=== Cleanup ===")
    r = requests.post(
        f"{BASE_URL}/api/v1/keys/delete",
        headers=json_headers(),
        json={"keys": [
            "greeting", "counter", "name", "role", "user:100",
            "tasks", "tags:post1", "leaderboard", "balance",
            "sse-demo", "events",
        ]},
    )
    print(f"  Deleted keys: {r.status_code} {r.json()}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    print("Redis Caching Service - Python Client Examples")
    print(f"Server: {BASE_URL}\n")

    try:
        check_health()
        string_operations()
        hash_operations()
        list_operations()
        set_operations()
        sorted_set_operations()
        key_operations()
        transaction_operations()
        pubsub_operations()
        admin_operations()
        sse_stream_example()
        cleanup()

        print("\nAll examples completed successfully!")
    except requests.ConnectionError:
        print(f"\nERROR: Cannot connect to {BASE_URL}")
        print("Make sure the service is running: docker-compose up -d")
