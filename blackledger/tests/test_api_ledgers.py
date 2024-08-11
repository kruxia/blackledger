import json
from datetime import datetime, timezone
from http import HTTPStatus

import dateparser
import pytest

from blackledger import types


@pytest.mark.parametrize(
    "query, names",
    [
        ("", ["test"]),
        ("?name=t", ["test"]),
        ("?name=foo", []),
        (f"?id={types.make_bigid()}", []),
    ],
)
def testsearch_ledgers_ok(client, query, names):
    response = client.get(f"/api/ledgers{query}")
    assert response.status_code == HTTPStatus.OK
    response_data = response.json()
    response_names = [item["name"] for item in response_data]
    assert response_names == names


@pytest.mark.parametrize(
    "data",
    [
        {"name": "Ledger 1"},
        {"name": "Ledger-1"},
        {"name": "Ledger.1"},
        {"name": "Ledger 2", "id": types.make_bigid()},
        {"name": "Ledger 3", "created": datetime.now(tz=timezone.utc)},
    ],
)
def test_post_ledger_insert_ok(client, data, json_dumps):
    response = client.post("/api/ledgers", content=json_dumps(data))
    assert response.status_code == HTTPStatus.OK
    response_data = response.json()
    assert response_data["name"] == data["name"]
    assert "id" in response_data


def test_post_ledger_update_ok(client, base_ledger, json_dumps):
    # "test" already exists, but that's fine
    data = json.loads(base_ledger.model_dump_json())
    data["name"] = "TESTED"
    data["created"] = datetime.now(tz=timezone.utc).isoformat()
    response = client.post("/api/ledgers", json=data)
    assert response.status_code == HTTPStatus.OK
    response_data = response.json()
    assert response_data["name"] == data["name"]
    assert str(response_data["id"]) == str(data["id"])
    assert dateparser.parse(response_data["created"]) == dateparser.parse(
        data["created"]
    )


@pytest.mark.parametrize(
    "data",
    [
        # name not empty
        {"name": ""},
        {"name": None},
        # name is NameString
        {"name": "Ledger,1"},
        # id is ID
        {"name": "Ledger 1", "id": "NOT_AN_ID"},
        # created is datetime
        {"name": "Ledger 1", "created": "NOT_A_TIMESTAMP"},
    ],
)
def test_post_ledger_insert_unprocessable(client, data, json_dumps):
    response = client.post("/api/ledgers", content=json_dumps(data))
    print(response.json())
    assert response.status_code == HTTPStatus.UNPROCESSABLE_ENTITY
