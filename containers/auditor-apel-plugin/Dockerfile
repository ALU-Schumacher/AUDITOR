#################
# BUILDER IMAGE #
#################

FROM python:3.8.17-slim-bookworm AS builder

# install build dependencies and create python env
RUN set -ex                                                          \
    && apt-get update                                                \
    && apt-get -y install git                                        \
    && python -m venv /env                                           \
    && /env/bin/pip install --upgrade pip                            \
    && apt-get -y autoremove && apt-get clean                        \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# activate python env
ENV VIRTUAL_ENV /env
ENV PATH /env/bin:$PATH

# copy code
COPY . /code

# install python package
WORKDIR /code/plugins/apel
RUN pip install --no-cache-dir .


################
# RUNNER IMAGE #
################

FROM python:3.8.17-slim-bookworm AS runner

# prevent python from writing *.pyc files to disc
ENV PYTHONDONTWRITEBYTECODE 1
# prevent python from buffering stdout and stderr
ENV PYTHONUNBUFFERED 1

# install run dependencies
RUN set -ex                                                          \
    && apt-get update                                                \
    && apt-get -y install ca-certificates                            \
    && apt-get -y autoremove && apt-get clean                        \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# copy python env from builder image
COPY --from=builder /env /env

# activate python env
ENV VIRTUAL_ENV /env
ENV PATH /env/bin:$PATH

WORKDIR /app
CMD ["/bin/bash"]
