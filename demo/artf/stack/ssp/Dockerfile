FROM node:22-alpine
WORKDIR /app
COPY demo/artf/stack/ssp/package.json ./
RUN npm install --omit=dev
COPY demo/artf/stack/ssp/server.mjs ./
COPY demo/artf/stack/ssp/ticket.mjs ./
COPY demo/artf/stack/ssp/public ./public
COPY proto /proto
COPY crates/rtblint-core/tests/fixtures/artf /fixtures
COPY demo/artf/fixtures/. /fixtures/
ENV PORT=8080 \
    FIXTURES=/fixtures \
    PROTO=/proto/openadtech/rtblint/v1/rtblint.proto \
    PROTO_ROOT=/proto
EXPOSE 8080
CMD ["node", "server.mjs"]
