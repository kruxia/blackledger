import psycopg_pool
import pytest
from fastapi.testclient import TestClient

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
