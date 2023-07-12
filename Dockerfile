FROM rust AS build

RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli
WORKDIR /app
COPY ./ /app
RUN cargo build --target wasm32-unknown-unknown --profile deploy
RUN wasm-bindgen --out-dir ./web_gen/ --target web ./target/wasm32-unknown-unknown/deploy/caticorn.wasm
RUN apt update && apt install binaryen
# https://bevy-cheatbook.github.io/platforms/wasm/size-opt.html
RUN ls -alh web_gen
RUN wasm-opt -O -ol 100 -s 100 -o web_gen/caticorn_bg.wasm web_gen/caticorn_bg.wasm

FROM nginx:alpine

COPY --from=build /app/web_gen/* /usr/share/nginx/html/
COPY wasm/* /usr/share/nginx/html/
COPY assets /usr/share/nginx/html/assets

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

