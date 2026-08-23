FROM node:24-bookworm-slim AS builder

WORKDIR /workspace
RUN corepack enable && corepack prepare pnpm@11.19.0 --activate

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY web/package.json ./web/package.json
RUN pnpm install --frozen-lockfile

COPY web ./web
RUN pnpm --dir web build

FROM nginx:1.29-alpine AS runtime

COPY deploy/docker/nginx.conf /etc/nginx/nginx.conf
COPY --from=builder /workspace/web/dist /usr/share/nginx/html

USER nginx
EXPOSE 8080
