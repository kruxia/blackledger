from blackledger.domain import model


def _get_where_clause(filters):
    select_filters = filters.select_filters()
    query_filters = []
    if select_filters:
        query_filters += [
            "WHERE",
            " AND ".join(select_filters),
        ]
    return query_filters


def _get_query_params(params):
    select_params = params.select_params()
    query_params = []
    if select_params.get("orderby"):
        query_params.append(f"ORDER BY {select_params['orderby']}")
    if select_params.get("limit"):
        query_params.append(f"LIMIT {select_params['limit']}")
    if select_params.get("offset"):
        query_params.append(f"OFFSET {select_params['offset']}")
    return query_params


async def select_balances(conn, sql, filters, params):
    query = [
        "WITH balances AS (",
        "    SELECT a.id account_id,",
        "        e.curr, sum(e.dr) dr, sum(e.cr) cr",
        "    FROM account a",
        "    JOIN entry e",
        "        ON a.id = e.acct",
        "    GROUP BY (a.id, e.curr)",
        ")",
        "SELECT account.*,",
        "    balances.curr, balances.dr, balances.cr",
        "FROM account",
        "JOIN balances",
        "    ON account.id = balances.account_id",
    ]
    query.extend(_get_where_clause(filters))
    query.extend(_get_query_params(params))

    return await sql.select_all(conn, query, filters.query_data())


async def select_currencies(conn, sql, filters, params):
    query = sql.queries.SELECT(
        "currency", filters=filters.select_filters(), **params.select_params()
    )
    return await sql.select_all(
        conn,
        query,
        filters.model_dump(exclude_none=True),
        Constructor=model.Currency,
    )


async def select_tenants(conn, sql, filters, params):
    query = sql.queries.SELECT(
        "tenant", filters=filters.select_filters(), **params.select_params()
    )
    return await sql.select_all(
        conn, query, filters.query_data(), Constructor=model.Tenant
    )


async def select_transactions(conn, sql, filters, params):
    # select the transactions
    tx_query = [
        # filter transaction ids based on transaction and entry fields
        "WITH filtered_tr AS (",
        "  SELECT distinct(transaction.id)",
        "  FROM transaction",
        "  JOIN entry",
        "    ON transaction.id = entry.tx",
    ]
    tx_query.extend(_get_where_clause(filters))
    tx_query += [
        ")",
        # select matching transactions (only -- entries are separate)
        "SELECT",
        "  DISTINCT tx.*",
        "FROM transaction tx",
        "JOIN filtered_tr",
        "  ON filtered_tr.id = tx.id",
    ]
    tx_query.extend(_get_query_params(params))

    tx_results = await sql.select_all(
        conn, tx_query, filters.query_data(), Constructor=model.Transaction
    )
    # construct a dictionary of transactions by id
    transactions = {tx.id: tx for tx in tx_results}

    # select corresponding entries
    entries_query = """
        SELECT e.*, a.name acct_name FROM entry e
        JOIN transaction t ON e.tx = t.id
        JOIN account a ON e.acct = a.id
        WHERE t.id = ANY(:tx)
    """
    entries_params = {"tx": [str(tx_id.to_uuid()) for tx_id in transactions.keys()]}
    entries_results = await sql.select_all(
        conn, entries_query, entries_params, Constructor=model.Entry
    )
    # collate entries under their transactions
    for entry in entries_results:
        transactions[entry.tx].entries.append(entry)

    # select queries return lists
    return list(transactions.values())
