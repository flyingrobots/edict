FROM rust:1.95.0-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /edict
COPY . /edict
RUN cargo build --locked -p edict-cli
WORKDIR /consumer-source
RUN git init && git remote add origin https://github.com/flyingrobots/jedit.git \
    && git fetch --depth=1 origin 19edb6fba94a8fea2dea63aa2f05cffc3e084f97 \
    && git archive FETCH_HEAD edict/replace-range-probes/state-read | tar -x
WORKDIR /echo-source
RUN git init && git remote add origin https://github.com/flyingrobots/echo.git \
    && git fetch --depth=1 origin 49e9efb68001dfd78563d18bac9359a87671e431 \
    && git archive FETCH_HEAD schemas/edict-provider/package/v1 | tar -x
WORKDIR /edict
CMD ["python3", "scripts/consumer-witnesses/jedit-state-read.py"]
