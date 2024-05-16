FROM rust

WORKDIR /app
COPY ./target/debug/rmq-consumer-prom ./.env* ./
EXPOSE 8888
CMD [ "sh", "-c", "./rmq-consumer-prom" ]