/**
 * Redis Caching Service - JavaScript/Node.js Client Examples
 *
 * Requirements:
 *   Node.js 18+ (uses native fetch and WebSocket)
 *   npm install ws eventsource   # for WebSocket and SSE examples
 *
 * Usage:
 *   node javascript_client.mjs
 */

const BASE_URL = "http://localhost:8080";
const ADMIN_KEY = "dev-admin-key";

async function api(method, path, body = null, headers = {}) {
  const opts = {
    method,
    headers: { "Content-Type": "application/json", ...headers },
  };
  if (body) opts.body = JSON.stringify(body);

  const res = await fetch(`${BASE_URL}${path}`, opts);
  const text = await res.text();
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    data = text;
  }
  return { status: res.status, data };
}

function adminHeaders() {
  return { "X-Admin-Api-Key": ADMIN_KEY };
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

async function checkHealth() {
  console.log("=== Health Check ===");
  const r = await api("GET", "/health");
  console.log(`  Status: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// Strings
// ---------------------------------------------------------------------------

async function stringOperations() {
  console.log("\n=== String Operations ===");

  let r = await api("PUT", "/api/v1/strings/greeting", {
    value: "Hello from JavaScript!",
    ttl_seconds: 3600,
  });
  console.log(`  SET: ${r.status}`, r.data);

  r = await api("GET", "/api/v1/strings/greeting");
  console.log(`  GET: ${r.status}`, r.data);

  r = await api("POST", "/api/v1/strings/mset", {
    pairs: { counter: "0", name: "Bob", lang: "javascript" },
  });
  console.log(`  MSET: ${r.status}`);

  r = await api("POST", "/api/v1/strings/mget", {
    keys: ["counter", "name", "lang"],
  });
  console.log(`  MGET: ${r.status}`, r.data);

  r = await api("PATCH", "/api/v1/strings/counter/incr", { delta: 10 });
  console.log(`  INCR: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// Hashes
// ---------------------------------------------------------------------------

async function hashOperations() {
  console.log("\n=== Hash Operations ===");

  let r = await api("PUT", "/api/v1/hashes/session:abc", {
    items: { user_id: "42", role: "editor", theme: "dark" },
  });
  console.log(`  HSET: ${r.status}`);

  r = await api("GET", "/api/v1/hashes/session:abc");
  console.log(`  HGETALL: ${r.status}`, r.data);

  r = await api("POST", "/api/v1/hashes/session:abc/fields/get", {
    fields: ["user_id", "role"],
  });
  console.log(`  HMGET: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

async function listOperations() {
  console.log("\n=== List Operations ===");

  let r = await api("POST", "/api/v1/lists/queue/rpush", {
    values: ["job-1", "job-2", "job-3"],
  });
  console.log(`  RPUSH: ${r.status}`, r.data);

  r = await api("GET", "/api/v1/lists/queue/length");
  console.log(`  LLEN: ${r.status}`, r.data);

  r = await api("GET", "/api/v1/lists/queue/range?start=0&stop=-1");
  console.log(`  LRANGE: ${r.status}`, r.data);

  r = await api("POST", "/api/v1/lists/queue/lpop", { count: 1 });
  console.log(`  LPOP: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// Sorted Sets
// ---------------------------------------------------------------------------

async function sortedSetOperations() {
  console.log("\n=== Sorted Set Operations ===");

  let r = await api("POST", "/api/v1/sorted-sets/scores/members", {
    members: [
      { member: "alice", score: 95 },
      { member: "bob", score: 87 },
      { member: "charlie", score: 92 },
    ],
  });
  console.log(`  ZADD: ${r.status}`, r.data);

  r = await api(
    "GET",
    "/api/v1/sorted-sets/scores/range?start=0&stop=-1&rev=true&with_scores=true"
  );
  console.log(`  ZRANGE (rev): ${r.status}`, r.data);

  r = await api("GET", "/api/v1/sorted-sets/scores/score/alice");
  console.log(`  ZSCORE alice: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// Transactions
// ---------------------------------------------------------------------------

async function transactionOperations() {
  console.log("\n=== Transaction Operations ===");

  await api("PUT", "/api/v1/strings/version", { value: "1" });

  let r = await api("POST", "/api/v1/transactions/execute", {
    commands: [
      { type: "GET", key: "version" },
      { type: "SET", key: "version", value: "2" },
    ],
  });
  console.log(`  MULTI/EXEC: ${r.status}`, r.data);

  r = await api("POST", "/api/v1/transactions/cas", {
    key: "version",
    expected_value: "2",
    new_value: "3",
  });
  console.log(`  CAS: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// Pub/Sub
// ---------------------------------------------------------------------------

async function pubsubOperations() {
  console.log("\n=== Pub/Sub Operations ===");

  let r = await api("POST", "/api/v1/pubsub/publish", {
    channel: "events",
    message: "Hello from JavaScript!",
  });
  console.log(`  PUBLISH: ${r.status}`, r.data);

  r = await api("GET", "/api/v1/pubsub/stats");
  console.log(`  STATS: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

async function adminOperations() {
  console.log("\n=== Admin Operations ===");

  let r = await api("GET", "/api/v1/admin/pool/stats");
  console.log(`  Pool: ${r.status}`, r.data);

  r = await api("GET", "/api/v1/admin/capabilities");
  console.log(`  Capabilities: ${r.status}`, r.data);

  r = await api("GET", "/api/v1/admin/server/dbsize", null, adminHeaders());
  console.log(`  DB Size: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// SSE Stream Subscription (Browser-compatible EventSource)
// ---------------------------------------------------------------------------

function sseExample() {
  console.log("\n=== SSE Stream Example (browser) ===");
  console.log("  In a browser, use:");
  console.log(`
  const es = new EventSource('${BASE_URL}/api/v1/streams/mystream/subscribe?last_id=0');

  // Each entry is emitted as a separate 'message' event
  es.addEventListener('message', (event) => {
    const entry = JSON.parse(event.data);
    console.log('ID:', entry.id, entry.fields);
  });

  es.addEventListener('error', (event) => {
    console.error('SSE error:', event.data);
  });
`);
}

// ---------------------------------------------------------------------------
// WebSocket Pub/Sub (Browser-compatible)
// ---------------------------------------------------------------------------

function websocketExample() {
  console.log("=== WebSocket Pub/Sub Example (browser) ===");
  console.log("  In a browser, use:");
  console.log(`
  const ws = new WebSocket('ws://localhost:8080/api/v1/pubsub/subscribe?channels=news,alerts');

  ws.onopen = () => console.log('Connected');

  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    console.log(\`[\${msg.channel}] \${msg.message}\`);
  };

  ws.onerror = (err) => console.error('WebSocket error:', err);
  ws.onclose = () => console.log('Disconnected');
`);
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

async function cleanup() {
  console.log("\n=== Cleanup ===");
  const r = await api("POST", "/api/v1/keys/delete", {
    keys: [
      "greeting", "counter", "name", "lang",
      "session:abc", "queue", "scores", "version",
    ],
  });
  console.log(`  Deleted: ${r.status}`, r.data);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  console.log("Redis Caching Service - JavaScript Client Examples");
  console.log(`Server: ${BASE_URL}\n`);

  try {
    await checkHealth();
    await stringOperations();
    await hashOperations();
    await listOperations();
    await sortedSetOperations();
    await transactionOperations();
    await pubsubOperations();
    await adminOperations();
    sseExample();
    websocketExample();
    await cleanup();

    console.log("\nAll examples completed successfully!");
  } catch (err) {
    console.error(`\nERROR: ${err.message}`);
    console.error("Make sure the service is running: docker-compose up -d");
  }
}

main();
