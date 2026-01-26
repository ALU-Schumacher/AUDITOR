
FROM python:3.9-slim-bookworm AS builder

# install build dependencies and create python env
RUN set -ex                                                          \
    && apt-get update                                                \
    && apt-get -y install build-essential curl git                   \
    && python -m venv /env                                           \
    && /env/bin/pip install --upgrade pip                            \
    && apt-get -y autoremove && apt-get clean                        \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# install rust toolchain
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# activate python env
ENV VIRTUAL_ENV=/env
ENV PATH=/env/bin:$PATH

# copy code
COPY . /code

# build pyauditor wheel
WORKDIR /code/pyauditor
RUN pip wheel .

# install python package
WORKDIR /code/plugins/utilization_tool
RUN    pip install --no-cache-dir /code/pyauditor/*.whl \
    && pip install --no-cache-dir .


FROM python:3.9-slim-bookworm AS runner

# prevent python from writing *.pyc files to disc
ENV PYTHONDONTWRITEBYTECODE=1
# prevent python from buffering stdout and stderr
ENV PYTHONUNBUFFERED=1

# install run dependencies
RUN set -ex                                                          \
    && apt-get update                                                \
    && apt-get -y install ca-certificates                            \
    && apt-get -y autoremove && apt-get clean                        \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# copy python env from builder image
COPY --from=builder /env /env

# For the local persistent queue.
# Mapped users must be able to access the data dir if nothing else is mounted there
RUN mkdir --mode=777 /data

# activate python env
ENV VIRTUAL_ENV=/env
ENV PATH=/env/bin:$PATH

WORKDIR /app
CMD ["/bin/bash"]
