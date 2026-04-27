session := "zhiying-backend"

serve:
  tmux has-session -t {{session}} 2>/dev/null || tmux new -d -s {{session}} 'cargo run'

stop:
  -tmux kill-session -t {{session}}

log:
  tmux attach -t {{session}}

fmt:
  cargo fmt

check:
  cargo check
