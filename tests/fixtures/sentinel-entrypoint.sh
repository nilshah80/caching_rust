#!/bin/sh
# Wait for master to be resolvable
echo "Waiting for master..."
for i in $(seq 1 30); do
  if redis-cli -h redis-master -p 6380 PING 2>/dev/null | grep -q PONG; then
    echo "Master reachable"
    break
  fi
  sleep 1
done

# Generate config dynamically (avoids hostname resolution at config parse time)
cat > /tmp/sentinel.conf << EOF
port 26379
sentinel resolve-hostnames yes
sentinel monitor mymaster redis-master 6380 2
sentinel down-after-milliseconds mymaster 5000
sentinel failover-timeout mymaster 10000
sentinel parallel-syncs mymaster 1
EOF

exec redis-sentinel /tmp/sentinel.conf
