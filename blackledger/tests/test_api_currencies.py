from http import HTTPStatus

import pytest


@pytest.fixture
def base_currencies(dbpool):
    with dbpool.connection() as conn:
        conn.execute("insert into currency (code) values ('USD'), ('CAD')")

    yield

    with dbpool.connection() as conn:
        conn.execute("delete from currency")


def test_get_currencies(client, base_currencies):
    response = client.get("/api/currencies")
    assert response.status_code == HTTPStatus.OK
    assert response.json() == [{"code": "USD"}, {"code": "CAD"}]


def test_post_currencies_insert(client):
    response = client.post("/api/currencies", json={"code": "GOOG"})
    assert response.status_code == HTTPStatus.OK
    assert response.json() == {"code": "GOOG"}


def test_post_currencies_update(client, base_currencies):
    # "USD" already exists, but that's fine
    response = client.post("/api/currencies", json={"code": "USD"})
    assert response.status_code == HTTPStatus.OK
    assert response.json() == {"code": "USD"}
