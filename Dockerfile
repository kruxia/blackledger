# == base ==
FROM python:3.11-slim-bullseye AS base
LABEL maintainer="sah@kruxia.com"

WORKDIR /opt/blackledger

RUN apt update \
    && apt-get install -y git postgresql-client \
    && git config --global --add safe.directory /opt/blackledger

COPY ./pyproject.toml setup.cfg ./
COPY ./blackledger/ ./blackledger/

# == deploy ==

FROM base AS deploy

RUN pip install -e .[http]

EXPOSE 8000
CMD ["gunicorn", "-w", "4", "-k", "uvicorn.workers.UvicornWorker", \
    "--bind", "0.0.0.0:8000", "blackledger.http:app"]

# == develop ==

FROM base AS develop

RUN pip install -e .[http,dev,test]

EXPOSE 8000
CMD ["bash"]
CMD ["uvicorn", "--reload",  \
    "--host", "0.0.0.0", "--port", "8000", "blackledger.http:app"]
