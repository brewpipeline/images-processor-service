# syntax=docker/dockerfile:1
FROM rust:1.95-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev make && rm -rf /var/lib/apt/lists/*

ARG DOMAIN
ARG THUMBNAIL_SMALL_WIDTH=250
ARG THUMBNAIL_MEDIUM_WIDTH=750
ARG THUMBNAIL_HEIGHT_MULTIPLIER=3

ENV SERVER_ADDRESS="127.0.0.1:3000" \
    EXTERN_LOCATION_IMAGES_STORAGE_PATH=https://$DOMAIN/ \
    LOCAL_IMAGES_STORAGE_PATH=/images/ \
    THUMBNAIL_SMALL_WIDTH=$THUMBNAIL_SMALL_WIDTH \
    THUMBNAIL_MEDIUM_WIDTH=$THUMBNAIL_MEDIUM_WIDTH \
    THUMBNAIL_HEIGHT_MULTIPLIER=$THUMBNAIL_HEIGHT_MULTIPLIER \
    EXTERNAL_TO_LOCAL_PATHS_MAP=https://$DOMAIN/\|/

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 nginx gettext-base && rm -rf /var/lib/apt/lists/*
RUN rm -f /etc/nginx/sites-enabled/default \
          /etc/nginx/sites-available/default \
          /etc/nginx/conf.d/default.conf \
          /var/www/html/index.nginx-debian.html

WORKDIR /app
COPY --from=builder /app/target/release/images-processor-service .

COPY <<'EOF' /etc/nginx/conf.d/default.conf.template
map $host $cors_origin {
    ~^images\.(.+)$ "https://$1";
    default          "";
}

server {
    listen 0.0.0.0:${PORT};

    root /images;

    underscores_in_headers on;

    add_header X-Content-Type-Options "nosniff" always;
    add_header Access-Control-Allow-Origin $cors_origin always;

    location / {
        directio 512;
        output_buffers 2 512k;
        try_files $uri =404;
    }

    location /mirror/ {
        proxy_pass http://127.0.0.1:3000/;
        proxy_http_version 1.1;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-Host $host;
        proxy_set_header X-Forwarded-Port $server_port;
    }
}
EOF

COPY <<'EOF' /app/start.sh
#!/bin/sh
set -eu
echo "PORT=$PORT"
export PORT
envsubst '${PORT}' \
    < /etc/nginx/conf.d/default.conf.template \
    > /etc/nginx/conf.d/default.conf
nginx -t
mkdir -p /images
./images-processor-service &
SERVER_PID=$!
( wait "$SERVER_PID"; echo "images-processor-service exited" >&2; kill 1 ) &
exec nginx -g "daemon off;"
EOF

RUN chmod +x /app/start.sh

CMD ["/app/start.sh"]
