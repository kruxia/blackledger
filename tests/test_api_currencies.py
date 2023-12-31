from http import HTTPStatus

import pytest


@pytest.mark.parametrize(
    "query, results",
    [
        # no query gives all currencies in order inserted
        ("", [{"code": "USD"}, {"code": "CAD"}, {"code": "MSFT"}]),
        # -- FILTERS --
        # filter to code returns that currency
        ("?code=USD", [{"code": "USD"}]),
        # filter to code returns regex matches
        ("?code=U", [{"code": "USD"}]),
        ("?code=S", [{"code": "USD"}, {"code": "MSFT"}]),
        ("?code=^S", []),
        # match is case-insensitive
        ("?code=usd", [{"code": "USD"}]),
        # -- SEARCH PARAMS
        # _orderby
        ("?_orderby=code", [{"code": "CAD"}, {"code": "MSFT"}, {"code": "USD"}]),
        # _limit
        ("?_orderby=code&_limit=1", [{"code": "CAD"}]),
        # _offset
        ("?_orderby=code&_limit=1&_offset=1", [{"code": "MSFT"}]),
        ("?_orderby=code&_limit=1&_offset=3", []),
    ],
)
def testsearch_currencies_ok(client, query, results):
    response = client.get(f"/api/currencies{query}")
    assert response.status_code == HTTPStatus.OK
    assert response.json() == results


@pytest.mark.parametrize(
    "data",
    [
        {"code": "GOOG"},  # all uppercase ok
        {"code": "G-O_O.G"},  # internal hyphen, period, underscore ok
        {"code": "G33G1"},  # non-initial number ok
    ],
)
def test_post_currencies_insert_ok(client, data):
    response = client.post("/api/currencies", json=data)
    assert response.status_code == HTTPStatus.OK
    assert response.json() == data


def test_post_currencies_update_ok(client):
    # "USD" already exists, but that's fine
    response = client.post("/api/currencies", json={"code": "USD"})
    assert response.status_code == HTTPStatus.OK
    assert response.json() == {"code": "USD"}


@pytest.mark.parametrize(
    "data",
    [  # input must have a valid Currency object
        {},  # code is required
        {"code": ""},  # at least two characters
        {"code": "U"},  # at least two characters
        {"code": 34},  # not a number
        {"code": "34S"},  # must start with a letter
        {"code": "us"},  # must be all uppercase
        {"code": "US-"},  # must end with a letter or number
    ],
)
def test_post_currencies_422(client, data):
    response = client.post("/api/currencies", json=data)
    assert response.status_code == HTTPStatus.UNPROCESSABLE_ENTITY
