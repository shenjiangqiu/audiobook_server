
deploy:
    cargo install --path .
    systemctl restart --user audiobook_server.service
status:
    systemctl status --user audiobook_server.service
stop:
    systemctl stop --user audiobook_server.service
start:
    systemctl start --user audiobook_server.service
debug_deploy:
    RUST_LOG=debug cargo run -- -p 3001    