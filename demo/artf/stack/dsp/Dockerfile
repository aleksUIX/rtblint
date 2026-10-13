FROM node:22-alpine
WORKDIR /app
COPY demo/artf/stack/dsp/server.mjs ./
ENV PORT=8092
EXPOSE 8092
CMD ["node", "server.mjs"]
