FROM rust:1.85.0 AS build

WORKDIR /usr/src/
COPY . .

RUN cargo build --release

FROM rust:1.85.0 AS game-server
COPY --from=build /usr/src/target/release/game-server /bin/game-server
CMD ["/bin/game-server"]

FROM rust:1.85.0 AS matchmaking-server
COPY --from=build /usr/src/target/release/matchmaking-server /bin/matchmaking-server
CMD ["/bin/matchmaking-server"]
