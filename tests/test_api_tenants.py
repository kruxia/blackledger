import json
from datetime import datetime, timezone
from http import HTTPStatus

import dateparser
import pytest

from blackledger.domain import types


@pytest.mark.parametrize(
    "query, names",
    [
        ("", ["test"]),
        ("?name=t", ["test"]),
        ("?name=foo", []),
        (f"?id={types.ID()}", []),
    ],
)
def test_search_tenants_ok(client, query, names):
    response = client.get(f"/api/tenants{query}")
    assert response.status_code == HTTPStatus.OK
    response_data = response.json()
    response_names = [item["name"] for item in response_data]
    assert response_names == names


@pytest.mark.parametrize(
    "data",
    [
        {"name": "Tenant 1"},
        {"name": "Tenant-1"},
        {"name": "Tenant.1"},
    ],
)
def test_post_tenant_insert_ok(client, data, json_dumps):
    response = client.post("/api/tenants", content=json_dumps(data))
    assert response.status_code == HTTPStatus.OK
    response_data = response.json()
    assert response_data["name"] == data["name"]
    assert "id" in response_data


def test_post_tenant_update_ok(client, base_tenant, json_dumps):
    # "test" already exists, but that's fine
    data = json.loads(base_tenant.model_dump_json())
    data["name"] = "TESTED"
    data["created"] = datetime.now(tz=timezone.utc).isoformat()
    response = client.post("/api/tenants", json=data)
    assert response.status_code == HTTPStatus.OK
    response_data = response.json()
    assert response_data["name"] == data["name"]
    assert str(response_data["id"]) == str(data["id"])
    assert dateparser.parse(response_data["created"]) == dateparser.parse(
        data["created"]
    )
