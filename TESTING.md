# Testing Jon Listen Locally

This guide provides step-by-step instructions for testing the server locally.

## Prerequisites

- Rust installed (stable edition)
- Terminal with `nc` (netcat) installed (usually pre-installed on macOS/Linux)

## Basic Testing

### 1. Start the Server

Open a terminal and start the server:

```bash
# Basic start (UDP mode, default port 8080)
RUST_LOG=info cargo run

# Or with debug logging
RUST_LOG=debug cargo run

# Or override port
APP_server_port=9000 RUST_LOG=info cargo run
```

You should see output like:
```
INFO  jon_listen > Starting jon-listen app...
INFO  jon_listen::listener::udp_server > Listening at 0.0.0.0:8080 via UDP...
INFO  jon_listen::writer::file_writer > File writer starting
INFO  jon_listen::metrics > Metrics initialized. Prometheus metrics available at http://0.0.0.0:9090/metrics
```

### 2. Test with UDP (Default)

**Option A: Using the example client** (in a new terminal):

```bash
# Send messages for 10 seconds
cargo run --example logging_client -- --address 127.0.0.1:8080 --duration 10

# Send a specific number of messages
cargo run --example send_via_udp -- 127.0.0.1:8080 100
```

**Option B: Using netcat** (quick test):

```bash
# Send a single message
echo "Hello from netcat!" | nc -u -w1 127.0.0.1 8080

# Send multiple messages
for i in {1..10}; do
  echo "Message $i" | nc -u -w1 127.0.0.1 8080
done
```

**Option C: Using the helper script**:

```bash
# Send 1000 datagrams
./scripts/send_datagrams.sh 1000
```

### 3. Test with TCP

**First, start the server in TCP mode**:

```bash
APP_server_protocol=TCP RUST_LOG=info cargo run
```

**Then send messages** (in a new terminal):

```bash
# Using the example client
cargo run --example logging_client -- --address 127.0.0.1:8080 --duration 10 --tcp

# Using netcat
echo "Hello from TCP!" | nc 127.0.0.1 8080

# Send multiple messages
for i in {1..10}; do
  echo "TCP Message $i" | nc 127.0.0.1 8080
done
```

### 4. Verify Log Files

Check that messages were written to the log file:

```bash
# View the log file (default location: ./log)
cat log

# Or with tail to see new messages
tail -f log

# Check for rotated files
ls -la log*
```

### 5. Check Metrics

View Prometheus metrics:

```bash
# Get all metrics
curl http://localhost:9090/metrics

# Get specific metrics
curl http://localhost:9090/metrics | grep messages_received
curl http://localhost:9090/metrics | grep tcp_connections
curl http://localhost:9090/metrics | grep udp_datagrams
```

## Advanced Testing Scenarios

### Test File Rotation

**Test rotation by duration** (rotates every 10 seconds):

```bash
APP_filewriter_rotation_policy=ByDuration \
APP_filewriter_rotation_duration=10 \
RUST_LOG=info cargo run
```

Then send messages continuously and watch for file rotation:

```bash
# In another terminal, send messages continuously
while true; do
  echo "Test message $(date)" | nc -u -w1 127.0.0.1 8080
  sleep 1
done
```

Watch the log directory:
```bash
watch -n 1 'ls -lh log*'
```

### Test Backpressure Handling

**Test Block policy** (waits when buffer is full):

```bash
APP_filewriter_backpressure_policy=Block \
RUST_LOG=info cargo run
```

**Test Discard policy** (drops messages when buffer is full):

```bash
APP_filewriter_backpressure_policy=Discard \
RUST_LOG=info cargo run
```

Send a burst of messages to test:

```bash
# Send 1000 messages quickly
for i in {1..1000}; do
  echo "Message $i" | nc -u -w1 127.0.0.1 8080 &
done
```

### Test TCP Connection Limits

Start server with low connection limit:

```bash
APP_server_protocol=TCP \
APP_server_max_connections=2 \
RUST_LOG=info cargo run
```

Then try to connect with multiple clients:

```bash
# Terminal 1
echo "Client 1" | nc 127.0.0.1 8080

# Terminal 2
echo "Client 2" | nc 127.0.0.1 8080

# Terminal 3 (should be rejected)
echo "Client 3" | nc 127.0.0.1 8080
```

Check server logs for rejection messages.

### Test Graceful Shutdown

1. Start the server and send some messages
2. Press `Ctrl+C` in the server terminal
3. Verify the server shuts down gracefully:
   - Check logs for "Shutdown signal received"
   - Verify all messages were written to file
   - No errors in output

### Test Metrics Endpoint

Start server and verify metrics are accessible:

```bash
# Start server
RUST_LOG=info cargo run

# In another terminal, check metrics
curl http://localhost:9090/metrics

# Send some messages
cargo run --example send_via_udp -- 127.0.0.1:8080 10

# Check metrics again - should show increased counters
curl http://localhost:9090/metrics | grep messages_received
```

## Quick Test Checklist

- [ ] Server starts without errors
- [ ] UDP messages are received and written to file
- [ ] TCP messages are received and written to file
- [ ] Log file is created in the correct location
- [ ] Messages appear in the log file
- [ ] Metrics endpoint responds at `/metrics`
- [ ] Graceful shutdown works (Ctrl+C)
- [ ] File rotation works (if configured)
- [ ] Multiple concurrent connections work (TCP)

## Troubleshooting

**Server won't start:**
- Check if port 8080 is already in use: `lsof -i :8080`
- Check config file exists: `ls config/default.toml`
- Check logs for errors: `RUST_LOG=debug cargo run`

**Messages not appearing in log file:**
- Check file permissions in the output directory
- Verify server is actually running and listening
- Check server logs for errors
- Verify you're sending to the correct address/port

**Metrics not accessible:**
- Check if metrics port 9090 is available
- Verify metrics are enabled in config
- Check server logs for metrics initialization errors

