# == base ==
FROM python:3.11-slim-bullseye AS base
LABEL maintainer="sah@kruxia.com"

WORKDIR /opt/blackledger

RUN apt update \
    && apt-get install -y git postgresql-client \
    && git config --global --add safe.directory /opt/blackledger

COPY setup.json setup.py README.md ./

# # == deploy ==

# FROM base AS deploy

# RUN pip install -e .
# COPY ./ ./

# EXPOSE 8000
# CMD ["gunicorn", "-w", "4", "--bind", "0.0.0.0:8000", \
#     "project.wsgi:application"]

# == develop ==

FROM base AS develop

RUN pip install -e .[dev,test]
COPY ./ ./

EXPOSE 8000
CMD ["bash"]
# CMD ["gunicorn", "-w", "1", "--bind", "0.0.0.0:8000", "--reload", \
#     "project.wsgi:application"]
