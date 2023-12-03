from http import HTTPStatus

import pytest

from blackledger.domain import types


@pytest.mark.parametrize(
    "query, memos",
    [
        # -- FILTERS --
        ("?_orderby=id&curr=CAD", ["lunch", "dinner"]),
        ("?_orderby=id&memo=@", ["5 MSFT @ 377.43 USD"]),
        # -- SELECT PARAMS --
        # orderby
        (
            "?_orderby=id",
            ["client1", "client2", "lunch", "dinner", "5 MSFT @ 377.43 USD"],
        ),
        (
            "?_orderby=-memo",
            ["lunch", "dinner", "client2", "client1", "5 MSFT @ 377.43 USD"],
        ),
        # limit
        ("?_orderby=id&_limit=1", ["client1"]),
        # offset
        (
            "?_orderby=id&_offset=1",
            ["client2", "lunch", "dinner", "5 MSFT @ 377.43 USD"],
        ),
    ],
)
def test_search_transactions(client, test_transactions, query, memos):
    """
    For the given URL query, we expect:

    - transactions with the given memos (from those loaded in test_transactions)
    - each transaction with all its entries listed.
    """
    test_transaction_ids = [str(t["id"]) for t in test_transactions]

    # Add to the query a filter for the first test_transaction timestamp (to avoid
    # conflict with other test cases -- because transactions cannot be deleted.)
    query += f"&tx={','.join(test_transaction_ids)}"

    response = client.get(f"/api/transactions{query}")
    assert response.status_code == HTTPStatus.OK
    response_data = [t for t in response.json() if t["id"] in test_transaction_ids]
    response_memos = [t["memo"] for t in response_data]
    assert response_memos == memos
    for response_tx in response_data:
        test_tx = test_transactions[test_transaction_ids.index(response_tx["id"])]
        assert len(response_tx["entries"]) == len(test_tx["entries"])


def test_post_transactions_ok(client, base_currencies, test_accounts, json_dumps):
    """
    The first transaction posted to an account should not have an 'acct_version' because
    it doesn't yet exist.

    Following transactions to an should have the correct 'acct_version', which is the
    last entry.id
    """
    post_txs = [
        {
            "memo": "first tx",
            "entries": [
                {"acct": test_accounts["Asset"].id, "dr": "1000", "curr": "USD"},
                {"acct": test_accounts["Income"].id, "cr": "1000", "curr": "USD"},
            ],
        }
    ]
    acct_versions = {}
    for index, post_tx in enumerate(post_txs):
        for entry in post_tx["entries"]:
            # update the entry acct_version
            if entry["acct"] in acct_versions:
                entry["acct_version"] = acct_versions[entry["acct"]]

            # validate the entry acct_version
            if index == 0:
                assert entry.get("acct_version") is None
            else:
                assert entry.get("acct_version") is not None

        response = client.post("/api/transactions", data=json_dumps(post_tx))
        response_tx = response.json()
        print(response.status_code, response_tx)
        assert response.status_code == HTTPStatus.CREATED

        # update acct_versions for following transactions
        for entry in response_tx["entries"]:
            acct_versions[entry["acct"]] = entry["id"]


def test_post_transaction_not_found(
    client, test_accounts, test_transactions, json_dumps
):
    """
    When posting a transaction to an account that doesn't exist, the
    response returns 404 NOT FOUND
    """

    post_tx = {
        "memo": "tx with unknown debit account",
        "entries": [
            {
                "acct": types.ID(),
                "dr": "1000",
                "curr": "USD",
            },
            {
                "acct": test_accounts["Income"].id,
                "cr": "1000",
                "curr": "USD",
            },
        ],
    }

    response = client.post("/api/transactions", data=json_dumps(post_tx))
    response_tx = response.json()
    print(response.status_code, response_tx)
    assert response.status_code == HTTPStatus.NOT_FOUND


@pytest.mark.parametrize(
    "acct_version",
    [
        # posting with no acct_version
        (None,),
        # posting with an invalid acct_version
        (types.ID(),),
    ],
)
def test_post_transaction_conflict(
    client, test_accounts, test_transactions, acct_version, json_dumps
):
    """
    When posting a transaction to an account that already has transactions, the
    'acct_version' field is required and must match the latest entry.id.
    """

    for version in acct_version:
        post_tx = {
            "memo": "not the first tx",
            "entries": [
                {
                    "acct": test_accounts["Asset"].id,
                    "acct_version": version,
                    "dr": "1000",
                    "curr": "USD",
                },
                {
                    "acct": test_accounts["Income"].id,
                    "acct_version": version,
                    "cr": "1000",
                    "curr": "USD",
                },
            ],
        }

        response = client.post("/api/transactions", data=json_dumps(post_tx))
        response_tx = response.json()
        print(response.status_code, response_tx)
        assert response.status_code == HTTPStatus.CONFLICT


def test_post_transaction_precondition_failed(client, test_accounts, json_dumps):
    """
    When posting a transaction with invalid input data, the response has the status code
    412 PRECONDITION FAILED.
    """

    for post_tx in [
        {
            "memo": "amounts must be strings, not ints",
            "entries": [
                {
                    "acct": test_accounts["Asset"].id,
                    "acct_version": None,
                    "dr": 1000,
                    "curr": "USD",
                },
                {
                    "acct": test_accounts["Income"].id,
                    "acct_version": None,
                    "cr": 1000,
                    "curr": "USD",
                },
            ],
        },
        {
            "memo": "amounts must be strings, not floats",
            "entries": [
                {
                    "acct": test_accounts["Asset"].id,
                    "acct_version": None,
                    "dr": 1000.00,
                    "curr": "USD",
                },
                {
                    "acct": test_accounts["Income"].id,
                    "acct_version": None,
                    "cr": 1000.00,
                    "curr": "USD",
                },
            ],
        },
        {
            "memo": "acct must be valid types.ID",
            "entries": [
                {
                    "acct": "NOT_AN_ID",
                    "acct_version": None,
                    "dr": 1000,
                    "curr": "USD",
                },
                {
                    "acct": test_accounts["Income"].id,
                    "acct_version": None,
                    "cr": 1000,
                    "curr": "USD",
                },
            ],
        },
        {
            "memo": "either cr or dr must be defined",
            "entries": [
                {
                    "acct": test_accounts["Asset"].id,
                    "acct_version": None,
                    # "dr": "",
                    "curr": "USD",
                },
                {
                    "acct": test_accounts["Income"].id,
                    "acct_version": None,
                    # "cr": "1000",
                    "curr": "USD",
                },
            ],
        },
        {
            "memo": "cr or dr must not both be defined",
            "entries": [
                {
                    "acct": test_accounts["Asset"].id,
                    "acct_version": None,
                    "dr": "1000",
                    "cr": "1000",
                    "curr": "USD",
                },
                {
                    "acct": test_accounts["Income"].id,
                    "acct_version": None,
                    "dr": "1000",
                    "cr": "1000",
                    "curr": "USD",
                },
            ],
        },
    ]:
        response = client.post("/api/transactions", data=json_dumps(post_tx))
        response_tx = response.json()
        print(post_tx["memo"], response.status_code, response_tx)
        assert response.status_code == HTTPStatus.PRECONDITION_FAILED
