import json

import psycopg_pool
import pytest
from fastapi.testclient import TestClient
from sqly import SQL

from blackledger.domain import types
from blackledger.http import app
from blackledger.settings import DatabaseSettings


@pytest.fixture(scope="session")
def client():
    # "You should use TestClient as a context manager, to ensure that the lifespan is
    # called." --<https://www.starlette.io/lifespan/#running-lifespan-in-tests>
    with TestClient(app) as testclient:
        yield testclient


@pytest.fixture(scope="session")
def dbpool():
    settings = DatabaseSettings()
    with psycopg_pool.ConnectionPool(
        conninfo=settings.url.get_secret_value(), timeout=1.00
    ) as pool:
        yield pool


@pytest.fixture(scope="session")
def sql():
    settings = DatabaseSettings()
    return SQL(dialect=settings.dialect)


@pytest.fixture(scope="session")
def json_dumps():
    class TestJsonEncoder(json.JSONEncoder):
        def default(self, obj):
            if isinstance(obj, (types.ID,)):
                return str(obj)
            return super().default(obj)

    return TestJsonEncoder().encode
