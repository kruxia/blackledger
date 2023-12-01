from http import HTTPStatus

import pytest


@pytest.mark.parametrize(
    "query, memos",
    [
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
