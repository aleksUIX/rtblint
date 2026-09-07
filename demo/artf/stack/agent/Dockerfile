FROM node:22-alpine
WORKDIR /app
COPY demo/artf/stack/agent/server.mjs ./
COPY crates/rtblint-core/tests/fixtures/artf /fixtures
COPY demo/artf/fixtures/. /fixtures/
ENV PORT=8091 FIXTURES=/fixtures
EXPOSE 8091
CMD ["node", "server.mjs"]
