### build build image
```bash
docker build -t build .
```

### build project
```bash
docker run --rm -v/home/denis/projects/rust/typing:/app build
```

### copy applicaton on remove server
```bash
scp  -i ~/Downloads/rust.pem target/release/typing ec2-user@ec2-3-65-101-155.eu-central-1.compute.amazonaws.com:/typing/
scp -r -i ~/Downloads/rust.pem target/site ec2-user@ec2-3-65-101-155.eu-central-1.compute.amazonaws.com:/typing/
```

### edit service
sudo systemctl edit typing --force --full

cat /etc/systemd/system/typing.service

[Unit]
Description=typing daemon
[Service]
Type=simple
User=ec2-user
Environment="RUST_LOG=info" "LEPTOS_SITE_ADDR=0.0.0.0:3000" "LEPTOS_SITE_ROOT=/typing/site" "DATABASE_PATH=/typing/typing_db"
ExecStart=/typing/typing
[Install]
WantedBy=multi-user.target


