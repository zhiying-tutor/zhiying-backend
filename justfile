session := "zhiying-backend"
health_url := "http://localhost:9000/health"

build:
  @cargo build

serve:
  @cargo build --quiet
  @tmux has-session -t {{session}} 2>/dev/null || tmux new -d -s {{session}} 'cargo run'
  @just wait

# Block until /health responds 200, or fail after 60s.
wait:
  @timeout 60 bash -c 'until curl -sf {{health_url}} >/dev/null 2>&1; do sleep 0.5; done' \
    && printf '\033[32m✔\033[0m zhiying-backend \033[32mReady\033[0m\n' \
    || (printf '\033[31m✗\033[0m zhiying-backend \033[31mFailed\033[0m\n'; exit 1)

stop:
  @tmux kill-session -t {{session}} 2>/dev/null; \
   printf '\033[32m✔\033[0m zhiying-backend \033[32mStopped\033[0m\n'

log:
  @tmux attach -t {{session}}

fmt:
  cargo fmt

check:
  cargo check
